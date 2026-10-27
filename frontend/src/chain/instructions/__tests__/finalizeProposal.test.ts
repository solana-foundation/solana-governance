import type { AnchorWallet } from "@solana/wallet-adapter-react";
import { PublicKey, Transaction } from "@solana/web3.js";

const mockCreateProgramWithWallet = jest.fn();
const mockConfirmTransactionByPolling = jest.fn();
const mockSignTransactionForWallet = jest.fn();

jest.mock("../helpers", () => ({
  createProgramWithWallet: (...args: unknown[]) =>
    mockCreateProgramWithWallet(...args),
  confirmTransactionByPolling: (...args: unknown[]) =>
    mockConfirmTransactionByPolling(...args),
  signTransactionForWallet: (...args: unknown[]) =>
    mockSignTransactionForWallet(...args),
}));

import { finalizeProposal } from "../finalizeProposal";
import { SVMGOV_PROGRAM_ID } from "../types";

const keyFromByte = (byte: number): PublicKey =>
  new PublicKey(new Uint8Array(32).fill(byte));
const PROPOSAL = keyFromByte(1);
const SIGNER = keyFromByte(2);
const BLOCKHASH = keyFromByte(3).toBase58();

describe("finalizeProposal", () => {
  const mockInstruction = jest.fn();
  const mockGetLatestBlockhash = jest.fn();
  const mockSendRawTransaction = jest.fn();
  const mockRpc = jest.fn();
  const wallet = {
    publicKey: SIGNER,
    signTransaction: jest.fn(),
    signAllTransactions: jest.fn(),
  } as unknown as AnchorWallet;
  const consoleError = jest
    .spyOn(console, "error")
    .mockImplementation(() => undefined);

  afterAll(() => consoleError.mockRestore());

  beforeEach(() => {
    jest.clearAllMocks();

    const instruction = {
      keys: [],
      programId: SVMGOV_PROGRAM_ID,
      data: Buffer.alloc(0),
    };
    mockInstruction.mockResolvedValue(instruction);
    mockGetLatestBlockhash.mockResolvedValue({
      blockhash: BLOCKHASH,
      lastValidBlockHeight: 123,
    });
    mockSendRawTransaction.mockResolvedValue("finalize-signature");
    mockConfirmTransactionByPolling.mockResolvedValue({ value: { err: null } });
    mockSignTransactionForWallet.mockImplementation(
      async (_wallet: AnchorWallet, transaction: Transaction) => ({
        serialize: () => Buffer.from("signed-transaction"),
        transaction,
      }),
    );

    const accounts = jest.fn(() => ({
      instruction: mockInstruction,
      rpc: mockRpc,
    }));
    mockCreateProgramWithWallet.mockReturnValue({
      provider: {
        connection: {
          getLatestBlockhash: mockGetLatestBlockhash,
          sendRawTransaction: mockSendRawTransaction,
        },
      },
      methods: { finalizeProposal: jest.fn(() => ({ accounts })) },
    });
  });

  const blockchainParams = {
    network: "testnet" as const,
    endpoint: "http://localhost:8899",
  };

  it("submits and confirms finalization entirely over HTTP", async () => {
    await expect(
      finalizeProposal(
        { proposalId: PROPOSAL.toBase58(), wallet },
        blockchainParams,
      ),
    ).resolves.toEqual({ signature: "finalize-signature", success: true });

    expect(mockRpc).not.toHaveBeenCalled();
    expect(mockSendRawTransaction).toHaveBeenCalledWith(
      Buffer.from("signed-transaction"),
      { preflightCommitment: "confirmed" },
    );
    expect(mockConfirmTransactionByPolling).toHaveBeenCalledWith(
      expect.objectContaining({
        getLatestBlockhash: mockGetLatestBlockhash,
        sendRawTransaction: mockSendRawTransaction,
      }),
      "finalize-signature",
      123,
    );
  });

  it("reports an on-chain confirmation error", async () => {
    mockConfirmTransactionByPolling.mockResolvedValueOnce({
      value: { err: { InstructionError: [0, "Custom"] } },
    });

    await expect(
      finalizeProposal(
        { proposalId: PROPOSAL.toBase58(), wallet },
        blockchainParams,
      ),
    ).resolves.toEqual(
      expect.objectContaining({
        signature: "",
        success: false,
        error: expect.stringContaining("Failed to finalize proposal"),
      }),
    );
  });
});
