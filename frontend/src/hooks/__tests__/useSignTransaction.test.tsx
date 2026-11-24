import {
  createSolanaRpc,
  generateKeyPairSigner,
  signBytes,
  SOLANA_ERROR__RPC__TRANSPORT_HTTP_ERROR,
  SolanaError,
  type Address,
  type Instruction,
  type KeyPairSigner,
} from "@solana/kit";
import { renderHook } from "@testing-library/react";
import { webcrypto } from "node:crypto";
import { setImmediate } from "node:timers";
import { TransactionOutcomeUnknownError } from "@/lib/confirmTransaction";
import { useSignTransaction } from "../useSignTransaction";

const mockUseKitTransactionSigner = jest.fn();

jest.mock("@solana/connector/react", () => ({
  useKitTransactionSigner: () => mockUseKitTransactionSigner(),
}));
jest.mock("@/contexts/EndpointContext", () => ({
  useEndpoint: () => ({ endpointUrl: "https://rpc.example/api/rpc?cluster=testnet" }),
}));
jest.mock("@solana/kit", () => ({
  ...jest.requireActual("@solana/kit"),
  createSolanaRpc: jest.fn(),
}));

const INSTRUCTION: Instruction = {
  programAddress: "11111111111111111111111111111111" as Address,
};
const CONFIRMED = {
  value: [{ confirmationStatus: "confirmed", confirmations: 1n, err: null, slot: 5n }],
};

const request = (response: () => Promise<unknown>) => jest.fn(() => ({ send: response }));

/** Installs the RPC the hook creates for the selected endpoint. */
function useRpc(overrides: {
  getSignatureStatuses?: () => Promise<unknown>;
  sendTransaction?: () => Promise<unknown>;
}) {
  const rpc = {
    getBlockHeight: request(() => Promise.resolve(1n)),
    getLatestBlockhash: request(() =>
      Promise.resolve({
        value: { blockhash: "11111111111111111111111111111111", lastValidBlockHeight: 100n },
      }),
    ),
    getSignatureStatuses: request(
      overrides.getSignatureStatuses ?? (() => Promise.resolve(CONFIRMED)),
    ),
    sendTransaction: request(overrides.sendTransaction ?? (() => Promise.resolve("sig"))),
  };
  jest.mocked(createSolanaRpc).mockReturnValue(rpc as unknown as ReturnType<typeof createSolanaRpc>);
  return rpc;
}

let wallet: KeyPairSigner;

beforeAll(async () => {
  Object.defineProperty(globalThis, "crypto", { configurable: true, value: webcrypto });
  Object.defineProperty(globalThis, "isSecureContext", { configurable: true, value: true });
  wallet = await generateKeyPairSigner();
});

beforeEach(() => {
  jest.clearAllMocks();
  mockUseKitTransactionSigner.mockReturnValue({ ready: true, signer: wallet });
});

const signAndSend = () =>
  renderHook(() => useSignTransaction()).result.current.signAndSend(() =>
    Promise.resolve([INSTRUCTION]),
  );

describe("useSignTransaction", () => {
  it("submits through the app's selected endpoint and resolves once confirmed", async () => {
    // The connector's own client stays on the cluster it loaded with, so a
    // transaction built for testnet used to be sent to mainnet.
    const rpc = useRpc({});

    await expect(signAndSend()).resolves.toEqual(expect.any(String));

    expect(createSolanaRpc).toHaveBeenCalledWith("https://rpc.example/api/rpc?cluster=testnet");
    expect(rpc.sendTransaction).toHaveBeenCalledTimes(1);
    expect(rpc.getSignatureStatuses).toHaveBeenCalled();
  });

  it("explains a simulation failure in the program's own words", async () => {
    useRpc({
      sendTransaction: () =>
        Promise.reject(
          jest.requireActual("@solana/kit").getSolanaErrorFromJsonRpcError({
            code: -32002,
            data: {
              err: { InstructionError: [0, { Custom: 6012 }] },
              logs: ["Program log: AnchorError occurred. Error Code: AlreadyVoted. Error Number: 6012. Error Message: Validator has already voted."],
            },
            message: "Transaction simulation failed",
          }),
        ),
    });

    await expect(signAndSend()).rejects.toThrow("Transaction failed: Validator has already voted");
  });

  it("does not send a transaction that a different account signed", async () => {
    // A wallet can switch accounts between connecting and signing; the vote
    // would then be signed by an account the instructions were not built for.
    const rpc = useRpc({});
    const other = await generateKeyPairSigner();
    mockUseKitTransactionSigner.mockReturnValue({
      ready: true,
      signer: {
        address: wallet.address,
        modifyAndSignTransactions: async (transactions: { messageBytes: Uint8Array; signatures: object }[]) => {
          return Promise.all(
            transactions.map(async (transaction) => ({
              ...transaction,
              signatures: {
                ...transaction.signatures,
                [wallet.address]: await signBytes(other.keyPair.privateKey, transaction.messageBytes),
              },
            })),
          );
        },
      },
    });

    await expect(signAndSend()).rejects.toThrow("different account");
    expect(rpc.sendTransaction).not.toHaveBeenCalled();
  });

  describe("when the RPC stops answering after the transaction was sent", () => {
    beforeEach(() => jest.useFakeTimers());
    afterEach(() => jest.useRealTimers());

    it("says the outcome is unknown instead of reporting a failure", async () => {
      // The rate-limit error behind the unknown outcome is a Solana error too,
      // and used to replace this message with "Transaction failed (Solana
      // error #8100002)", inviting a retry of a vote that may have landed.
      const rpc = useRpc({
        getSignatureStatuses: () =>
          Promise.reject(
            new SolanaError(SOLANA_ERROR__RPC__TRANSPORT_HTTP_ERROR, {
              headers: new Headers(),
              message: "Too Many Requests",
              statusCode: 429,
            }),
          ),
      });

      const outcome = signAndSend().catch((error: unknown) => error);
      // Signing settles off the microtask queue, so wait for the first poll
      // on Node's real setImmediate before advancing the poll timers.
      while (rpc.getSignatureStatuses.mock.calls.length === 0) {
        await new Promise((resolve) => setImmediate(resolve));
      }
      await jest.advanceTimersByTimeAsync(95_000);

      const error = await outcome;
      expect(error).toBeInstanceOf(TransactionOutcomeUnknownError);
      expect((error as Error).message).toContain("may still have been processed");
    });
  });
});
