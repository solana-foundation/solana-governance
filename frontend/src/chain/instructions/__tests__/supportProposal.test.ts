import { ComputeBudgetProgram, PublicKey, Transaction } from "@solana/web3.js";

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

import { supportProposal } from "../supportProposal";
import { SVMGOV_PROGRAM_ID } from "../types";

// PublicKey.findProgramAddressSync is unreliable under next/jest's web3.js build (see
// castVoteOverride.test.ts), and this flow derives PDAs both through helpers and inline. Stub it
// with a fixed viable nonce; the derived addresses are not what these tests assert.
jest.spyOn(PublicKey, "findProgramAddressSync").mockImplementation(
  () => [new PublicKey(new Uint8Array(32).fill(9)), 255]
);

// ComputeBudgetProgram.setComputeUnitLimit trips over the buffer-layout shim under next/jest;
// the compute-budget instruction is not what these tests assert.
jest
  .spyOn(ComputeBudgetProgram, "setComputeUnitLimit")
  .mockReturnValue(({
    keys: [],
    programId: new PublicKey(new Uint8Array(32).fill(30)),
    data: Buffer.alloc(0),
  } as unknown) as ReturnType<typeof ComputeBudgetProgram.setComputeUnitLimit>);

const keyFromByte = (b: number): string =>
  new PublicKey(new Uint8Array(32).fill(b)).toBase58();
const VOTE_ACCOUNT = keyFromByte(1);
const PROPOSAL = keyFromByte(2);
const SIGNER = keyFromByte(3);
const BLOCKHASH = keyFromByte(4);

describe("supportProposal", () => {
  const mockFetchProposal = jest.fn();
  const mockGetEpochInfo = jest.fn();
  const mockGetEpochSchedule = jest.fn();
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
    const supportProposalMethod = jest.fn(() => ({ accountsStrict }));

    return {
      programId: SVMGOV_PROGRAM_ID,
      provider: {
        connection: {
          getEpochInfo: mockGetEpochInfo,
          getEpochSchedule: mockGetEpochSchedule,
          getLatestBlockhash: mockGetLatestBlockhash,
          sendRawTransaction: mockSendRawTransaction,
        },
      },
      account: {
        proposal: {
          fetch: mockFetchProposal,
        },
      },
      methods: { supportProposal: supportProposalMethod },
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
    mockFetchProposal.mockResolvedValue({ numSupporters: 5 });
    mockGetEpochInfo.mockResolvedValue({ epoch: 700 });
    mockGetEpochSchedule.mockResolvedValue({
      getFirstSlotInEpoch: (epoch: number) => epoch * 432_000,
    });
    mockGetLatestBlockhash.mockResolvedValue({
      blockhash: BLOCKHASH,
      lastValidBlockHeight: 123,
    });
    mockSendRawTransaction.mockResolvedValue("test-signature");
  });

  const params = { proposalId: PROPOSAL, wallet };
  const blockchainParams = {
    network: "testnet" as const,
    endpoint: "http://localhost:8899",
    ncnApiUrl: "http://localhost:9000",
  };
  const validatorVoteAccount = {
    activeStake: 500_000,
    voteAccount: VOTE_ACCOUNT,
    nodePubkey: SIGNER,
  };
  const globalConfig = {
    discussionEpochs: 1,
    snapshotEpochExtension: 1,
    snapshotSlotOffset: 100,
  };

  it("signs with the connected account and submits the support", async () => {
    const result = await supportProposal(
      params,
      blockchainParams,
      340_850_340,
      validatorVoteAccount,
      globalConfig
    );

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

    await expect(
      supportProposal(
        params,
        blockchainParams,
        340_850_340,
        validatorVoteAccount,
        globalConfig
      )
    ).rejects.toThrow(/wallet did not sign the transaction/i);
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

    await expect(
      supportProposal(
        params,
        blockchainParams,
        340_850_340,
        validatorVoteAccount,
        globalConfig
      )
    ).rejects.toThrow(/signed with a different account/i);
    expect(mockSendRawTransaction).not.toHaveBeenCalled();
  });

  it("keeps validating the originally captured signer after an awaited fetch", async () => {
    // The proposal fetch happens before signing; switch the connected account during it.
    mockFetchProposal.mockImplementationOnce(async () => {
      walletState.publicKey = new PublicKey(keyFromByte(13));
      return { numSupporters: 5 };
    });

    await expect(
      supportProposal(
        params,
        blockchainParams,
        340_850_340,
        validatorVoteAccount,
        globalConfig
      )
    ).rejects.toThrow(/signed with a different account|did not sign the transaction/i);
    expect(mockSendRawTransaction).not.toHaveBeenCalled();
  });
});
