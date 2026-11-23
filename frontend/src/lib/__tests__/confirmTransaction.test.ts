import {
  isSolanaError,
  SOLANA_ERROR__BLOCK_HEIGHT_EXCEEDED,
  SOLANA_ERROR__INSTRUCTION_ERROR__CUSTOM,
  type Signature,
} from "@solana/kit";
import { confirmTransactionByPolling } from "../confirmTransaction";

const SIGNATURE = "signature" as Signature;

/** A kit RPC stub whose methods resolve to the given responses. */
function rpcReturning(responses: {
  getSignatureStatuses: unknown;
  getBlockHeight?: bigint;
}) {
  const getSignatureStatuses = jest.fn(() => ({
    send: () => Promise.resolve(responses.getSignatureStatuses),
  }));
  const getBlockHeight = jest.fn(() => ({
    send: () => Promise.resolve(responses.getBlockHeight),
  }));
  return {
    getBlockHeight,
    getSignatureStatuses,
    rpc: { getBlockHeight, getSignatureStatuses } as unknown as Parameters<
      typeof confirmTransactionByPolling
    >[0],
  };
}

describe("confirmTransactionByPolling", () => {
  it("returns after an HTTP status reaches confirmed", async () => {
    const { rpc, getSignatureStatuses } = rpcReturning({
      getSignatureStatuses: {
        value: [{ confirmationStatus: "confirmed", confirmations: 1n, err: null, slot: 123n }],
      },
    });

    await expect(confirmTransactionByPolling(rpc, SIGNATURE, 123n)).resolves.toBeUndefined();
    expect(getSignatureStatuses).toHaveBeenCalledWith([SIGNATURE]);
  });

  it("throws the transaction error from status polling", async () => {
    const { rpc } = rpcReturning({
      getSignatureStatuses: {
        value: [{
          confirmationStatus: "confirmed",
          confirmations: 1n,
          err: { InstructionError: [0, { Custom: 6000 }] },
          slot: 123n,
        }],
      },
    });

    const error = await confirmTransactionByPolling(rpc, SIGNATURE, 123n).catch((e: unknown) => e);
    expect(isSolanaError(error, SOLANA_ERROR__INSTRUCTION_ERROR__CUSTOM)).toBe(true);
    expect(
      isSolanaError(error, SOLANA_ERROR__INSTRUCTION_ERROR__CUSTOM) && error.context.code,
    ).toBe(6000);
  });

  it("stops polling only after the transaction's blockhash expires", async () => {
    const { rpc, getBlockHeight } = rpcReturning({
      getSignatureStatuses: { value: [null] },
      getBlockHeight: 124n,
    });

    const error = await confirmTransactionByPolling(rpc, SIGNATURE, 123n).catch((e: unknown) => e);
    expect(isSolanaError(error, SOLANA_ERROR__BLOCK_HEIGHT_EXCEEDED)).toBe(true);
    expect(getBlockHeight).toHaveBeenCalledWith({ commitment: "confirmed" });
  });
});
