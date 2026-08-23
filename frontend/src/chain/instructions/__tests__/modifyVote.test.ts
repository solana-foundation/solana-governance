import { PublicKey, Transaction } from "@solana/web3.js";

// helpers.ts transitively imports EndpointContext -> env.ts (an ESM-only package Jest does not
// transform). Stub it so the real helpers (including the signTransactionForWallet guard under
// test) can be required without pulling in the untransformed module.
jest.mock("@/contexts/EndpointContext", () => ({
  RPC_URLS: { testnet: "http://localhost:8899" },
}));

const mockGetVoteAccountProof = jest.fn();
const mockCreateProgramWithWallet = jest.fn();
const mockCreateGovV1ProgramWithWallet = jest.fn();
const mockComputeProofCloseTimestamp = jest.fn();
const mockGetMetaMerkleProofPda = jest.fn();
const mockDeriveVotePda = jest.fn();

jest.mock("../helpers", () => {
  const actual = jest.requireActual("../helpers");
  return {
    ...actual,
    getVoteAccountProof: (...args: unknown[]) =>
      mockGetVoteAccountProof(...args),
    createProgramWithWallet: (...args: unknown[]) =>
      mockCreateProgramWithWallet(...args),
    createGovV1ProgramWithWallet: (...args: unknown[]) =>
      mockCreateGovV1ProgramWithWallet(...args),
    computeProofCloseTimestamp: (...args: unknown[]) =>
      mockComputeProofCloseTimestamp(...args),
    getMetaMerkleProofPda: (...args: unknown[]) =>
      mockGetMetaMerkleProofPda(...args),
    deriveVotePda: (...args: unknown[]) => mockDeriveVotePda(...args),
  };
});

import { BN } from "@coral-xyz/anchor";
import type { AnchorWallet } from "@solana/wallet-adapter-react";

import { modifyVote } from "../modifyVote";
import { SVMGOV_PROGRAM_ID } from "../types";
import type { RPCEndpoint } from "@/types";

const keyFromByte = (b: number): string =>
  new PublicKey(new Uint8Array(32).fill(b)).toBase58();
const VOTE_ACCOUNT = keyFromByte(1);
const CONSENSUS_RESULT = keyFromByte(2);
const PROPOSAL = keyFromByte(3);
const SIGNER = keyFromByte(4);
const BLOCKHASH = keyFromByte(5);

const META_MERKLE_PROOF_PDA = new PublicKey(keyFromByte(20));
const VALIDATOR_VOTE_PDA = new PublicKey(keyFromByte(21));

describe("modifyVote", () => {
  const mockFetchProposal = jest.fn();
  const mockGetVoteAccounts = jest.fn();
  const mockGetAccountInfo = jest.fn();
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
    const modifyVoteMethod = jest.fn(() => ({ accountsStrict }));

    return {
      programId: SVMGOV_PROGRAM_ID,
      provider: {
        connection: {
          getVoteAccounts: mockGetVoteAccounts,
          getAccountInfo: mockGetAccountInfo,
          getLatestBlockhash: mockGetLatestBlockhash,
          sendRawTransaction: mockSendRawTransaction,
        },
      },
      account: {
        proposal: {
          fetch: mockFetchProposal,
        },
      },
      methods: { modifyVote: modifyVoteMethod },
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
    mockCreateGovV1ProgramWithWallet.mockReturnValue({
      methods: { initMetaMerkleProof: jest.fn() },
    });
    mockComputeProofCloseTimestamp.mockResolvedValue(2_000_000_000);
    mockGetVoteAccounts.mockResolvedValue({
      current: [{ nodePubkey: SIGNER, votePubkey: VOTE_ACCOUNT }],
      delinquent: [],
    });
    mockGetAccountInfo.mockResolvedValue({ data: Buffer.alloc(0) });
    mockGetLatestBlockhash.mockResolvedValue({
      blockhash: BLOCKHASH,
      lastValidBlockHeight: 123,
    });
    mockSendRawTransaction.mockResolvedValue("test-signature");
    mockGetMetaMerkleProofPda.mockReturnValue(META_MERKLE_PROOF_PDA);
    mockDeriveVotePda.mockReturnValue(VALIDATOR_VOTE_PDA);
    mockFetchProposal.mockResolvedValue({
      snapshotSlot: new BN(340_850_340),
      endEpoch: new BN(100),
    });
    mockGetVoteAccountProof.mockResolvedValue({
      network: "testnet",
      snapshot_slot: 340_850_340,
      meta_merkle_leaf: {
        active_stake: 500,
        stake_merkle_root: keyFromByte(6),
        vote_account: VOTE_ACCOUNT,
        voting_wallet: SIGNER,
      },
      meta_merkle_proof: [],
    });
  });

  const params = {
    proposalId: PROPOSAL,
    forVotesBp: 10_000,
    againstVotesBp: 0,
    abstainVotesBp: 0,
    wallet,
    consensusResult: new PublicKey(CONSENSUS_RESULT),
  };
  const blockchainParams = {
    network: "testnet" as RPCEndpoint,
    endpoint: "http://localhost:8899",
    ncnApiUrl: "http://localhost:9000",
  };

  it("signs with the connected account and submits the modified vote", async () => {
    const result = await modifyVote(params, blockchainParams);

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

    await expect(modifyVote(params, blockchainParams)).rejects.toThrow(
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

    await expect(modifyVote(params, blockchainParams)).rejects.toThrow(
      /signed with a different account/i
    );
    expect(mockSendRawTransaction).not.toHaveBeenCalled();
  });

  it("keeps validating the originally captured signer after an awaited fetch", async () => {
    mockGetVoteAccountProof.mockImplementationOnce(async () => {
      walletState.publicKey = new PublicKey(keyFromByte(13));
      return {
        network: "testnet",
        snapshot_slot: 340_850_340,
        meta_merkle_leaf: {
          active_stake: 500,
          stake_merkle_root: keyFromByte(6),
          vote_account: VOTE_ACCOUNT,
          voting_wallet: SIGNER,
        },
        meta_merkle_proof: [],
      };
    });

    await expect(modifyVote(params, blockchainParams)).rejects.toThrow(
      /signed with a different account|did not sign the transaction/i
    );
    expect(mockSendRawTransaction).not.toHaveBeenCalled();
  });
});
