import { PublicKey, Transaction } from "@solana/web3.js";

// helpers.ts transitively imports EndpointContext -> env.ts (an ESM-only package Jest does not
// transform). Stub it so the real helpers (including the signTransactionForWallet guard under
// test) can be required without pulling in the untransformed module.
jest.mock("@/contexts/EndpointContext", () => ({
  RPC_URLS: { testnet: "http://localhost:8899" },
}));

const mockCreateProgramWithWallet = jest.fn();

jest.mock("../helpers", () => {
  const actual = jest.requireActual("../helpers");
  return {
    ...actual,
    createProgramWithWallet: (...args: unknown[]) =>
      mockCreateProgramWithWallet(...args),
  };
});

import type { AnchorWallet } from "@solana/wallet-adapter-react";

import { createProposal } from "../createProposal";
import { SVMGOV_PROGRAM_ID } from "../types";

// PublicKey.findProgramAddressSync is unreliable under next/jest's web3.js build (see
// castVoteOverride.test.ts), and this flow derives proposal PDAs through helpers. Stub it with a
// fixed viable nonce; the derived addresses are not what these tests assert.
jest.spyOn(PublicKey, "findProgramAddressSync").mockImplementation(
  () => [new PublicKey(new Uint8Array(32).fill(9)), 255]
);

const keyFromByte = (b: number): string =>
  new PublicKey(new Uint8Array(32).fill(b)).toBase58();
const VOTE_ACCOUNT = keyFromByte(1);
const SIGNER = keyFromByte(3);
const BLOCKHASH = keyFromByte(4);

// A fixed-commit proposal URL, the same shape assertValidProposalUrl accepts in its own tests.
const DESCRIPTION =
  "https://github.com/solana-foundation/solana-governance-proposals/blob/27bca51e5c0fc34ddbea6904faf86f5098225316/proposals/sgp-0001-solana-constitution.md";

describe("createProposal", () => {
  const mockGetVoteAccounts = jest.fn();
  const mockGetLatestBlockhash = jest.fn();
  const mockSendRawTransaction = jest.fn();

  function buildFakeProgram() {
    const fakeIx = {
      keys: [],
      programId: SVMGOV_PROGRAM_ID,
      data: Buffer.alloc(0),
    };
    const instruction = jest.fn(async () => fakeIx);
    const accountsStrict = jest.fn(() => ({ instruction }));
    const createProposalMethod = jest.fn(() => ({ accountsStrict }));

    return {
      programId: SVMGOV_PROGRAM_ID,
      provider: {
        connection: {
          getVoteAccounts: mockGetVoteAccounts,
          getLatestBlockhash: mockGetLatestBlockhash,
          sendRawTransaction: mockSendRawTransaction,
        },
      },
      methods: { createProposal: createProposalMethod },
    };
  }

  const mockSignTransaction = jest.fn();
  const walletState = {
    publicKey: new PublicKey(SIGNER),
    signTransaction: mockSignTransaction,
    signAllTransactions: jest.fn(),
  };
  const wallet = walletState as unknown as AnchorWallet;

  function signedTransactionResponse(
    signatures: { publicKey: PublicKey; signature: Buffer | null }[] = [
      { publicKey: new PublicKey(SIGNER), signature: Buffer.alloc(64) },
    ]
  ): Transaction {
    return {
      signatures,
      verifySignatures: () => true,
      serialize: () => Buffer.alloc(0),
    } as unknown as Transaction;
  }

  beforeEach(() => {
    jest.clearAllMocks();
    walletState.publicKey = new PublicKey(SIGNER);
    // Emulates a real wallet: whatever account is connected NOW provides the signature.
    mockSignTransaction.mockImplementation(async () =>
      signedTransactionResponse([
        { publicKey: new PublicKey(walletState.publicKey), signature: Buffer.alloc(64) },
      ])
    );
    mockCreateProgramWithWallet.mockReturnValue(buildFakeProgram());
    mockGetVoteAccounts.mockResolvedValue({
      current: [{ nodePubkey: SIGNER, votePubkey: VOTE_ACCOUNT }],
      delinquent: [],
    });
    mockGetLatestBlockhash.mockResolvedValue({
      blockhash: BLOCKHASH,
      lastValidBlockHeight: 123,
    });
    mockSendRawTransaction.mockResolvedValue("test-signature");
  });

  const params = { title: "Test proposal", description: DESCRIPTION, wallet };
  const blockchainParams = {
    network: "testnet" as const,
    endpoint: "http://localhost:8899",
    ncnApiUrl: "http://localhost:9000",
  };

  it("signs with the connected account and submits the proposal", async () => {
    const result = await createProposal(params, blockchainParams);

    expect(result).toEqual({ signature: "test-signature", success: true });
    expect(mockSignTransaction).toHaveBeenCalledTimes(1);
    expect(mockSendRawTransaction).toHaveBeenCalledTimes(1);
  });

  it("reports an unsigned wallet response without submitting the transaction", async () => {
    mockSignTransaction.mockResolvedValueOnce(
      signedTransactionResponse([
        { publicKey: new PublicKey(SIGNER), signature: null },
      ])
    );

    await expect(createProposal(params, blockchainParams)).rejects.toThrow(
      /wallet did not sign the transaction/i
    );
    expect(mockSendRawTransaction).not.toHaveBeenCalled();
  });

  it("reports a returned signature from a different account without submitting", async () => {
    const otherAccount = new PublicKey(keyFromByte(12));
    mockSignTransaction.mockResolvedValueOnce(
      signedTransactionResponse([
        { publicKey: new PublicKey(SIGNER), signature: null },
        { publicKey: otherAccount, signature: Buffer.alloc(64) },
      ])
    );

    await expect(createProposal(params, blockchainParams)).rejects.toThrow(
      /signed with a different account/i
    );
    expect(mockSendRawTransaction).not.toHaveBeenCalled();
  });

  it("keeps validating the originally captured signer after an awaited fetch", async () => {
    // The vote-account lookup happens before signing; switch the connected account during it.
    // The lookup itself resolves against whichever account is connected at that point, so the
    // switched-to identity must be present in the response.
    const switchedAccount = keyFromByte(13);
    mockGetVoteAccounts.mockImplementationOnce(async () => {
      walletState.publicKey = new PublicKey(switchedAccount);
      return {
        current: [
          { nodePubkey: SIGNER, votePubkey: VOTE_ACCOUNT },
          { nodePubkey: switchedAccount, votePubkey: VOTE_ACCOUNT },
        ],
        delinquent: [],
      };
    });

    await expect(createProposal(params, blockchainParams)).rejects.toThrow(
      /signed with a different account|did not sign the transaction/i
    );
    expect(mockSendRawTransaction).not.toHaveBeenCalled();
  });
});
