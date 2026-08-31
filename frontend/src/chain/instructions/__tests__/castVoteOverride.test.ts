import {
  PublicKey,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";

// helpers.ts transitively imports EndpointContext -> env.ts (an ESM-only package Jest does not
// transform). Stub it so the real helpers (PDA derivations, converters, assertOverrideProofLineage)
// can be required without pulling in the untransformed module.
jest.mock("@/contexts/EndpointContext", () => ({
  RPC_URLS: { testnet: "http://localhost:8899" },
}));

// Mock the network / program-creating helpers and the PDA-derivation helpers; keep the real
// converters and assertOverrideProofLineage. (PublicKey.findProgramAddressSync is unreliable under
// next/jest's web3.js build, so the PDA helpers are stubbed — we assert on the inputs they receive,
// which is what the fix is about: every derivation must be driven by the snapshot vote account.)
const mockGetStakeAccountProof = jest.fn();
const mockGetVoteAccountProof = jest.fn();
const mockCreateProgramWithWallet = jest.fn();
const mockCreateGovV1ProgramWithWallet = jest.fn();
const mockComputeProofCloseTimestamp = jest.fn();
const mockGetMetaMerkleProofPda = jest.fn();
const mockDeriveVotePda = jest.fn();
const mockDeriveVoteOverridePda = jest.fn();
const mockDeriveVoteOverrideCachePda = jest.fn();
const mockConfirmTransactionByPolling = jest.fn();

jest.mock("../helpers", () => {
  const actual = jest.requireActual("../helpers");
  return {
    ...actual,
    getStakeAccountProof: (...args: unknown[]) =>
      mockGetStakeAccountProof(...args),
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
    deriveVoteOverridePda: (...args: unknown[]) =>
      mockDeriveVoteOverridePda(...args),
    deriveVoteOverrideCachePda: (...args: unknown[]) =>
      mockDeriveVoteOverrideCachePda(...args),
    confirmTransactionByPolling: (...args: unknown[]) =>
      mockConfirmTransactionByPolling(...args),
  };
});

import { BN } from "@coral-xyz/anchor";
import type { AnchorWallet } from "@solana/wallet-adapter-react";

import { castVoteOverride } from "../castVoteOverride";
import { SVMGOV_PROGRAM_ID } from "../types";
import type { RpcNetwork } from "@/types";

// Distinct, valid 32-byte public keys used as stand-ins (byte-filled so PDA derivation always
// resolves a viable nonce).
const keyFromByte = (b: number): string =>
  new PublicKey(new Uint8Array(32).fill(b)).toBase58();
const SNAPSHOT_VOTE_ACCOUNT = keyFromByte(1); // validator A at snapshot
const LIVE_VOTE_ACCOUNT = keyFromByte(2); // validator B (post-snapshot redelegation)
const STAKE_ACCOUNT = keyFromByte(3);
const DELEGATOR_WALLET = keyFromByte(4);
const VALIDATOR_WALLET = keyFromByte(10);
const STAKE_MERKLE_ROOT = keyFromByte(5);
const CONSENSUS_RESULT = keyFromByte(6);
const PROPOSAL = keyFromByte(7);
const SIGNER = keyFromByte(8);
const BLOCKHASH = keyFromByte(9);
const PROPOSAL_SNAPSHOT_SLOT = 340_850_340;
const LATEST_META_SLOT = 341_000_000;

// Stand-in return values for the mocked PDA-derivation helpers.
const META_MERKLE_PROOF_PDA = new PublicKey(keyFromByte(20));
const VALIDATOR_VOTE_PDA = new PublicKey(keyFromByte(21));
const VOTE_OVERRIDE_PDA = new PublicKey(keyFromByte(22));
const VOTE_OVERRIDE_CACHE_PDA = new PublicKey(keyFromByte(23));

describe("castVoteOverride", () => {
  let recordedAccounts: Record<string, PublicKey>;
  const mockFetchProposal = jest.fn();
  const mockGetAccountInfo = jest.fn();
  const mockGetLatestBlockhash = jest.fn();
  const mockSendRawTransaction = jest.fn();

  function buildFakeProgram() {
    recordedAccounts = {};
    const fakeIx = new TransactionInstruction({
      keys: [],
      programId: SVMGOV_PROGRAM_ID,
      data: Buffer.alloc(0),
    });
    const instruction = jest.fn(async () => fakeIx);
    const accountsStrict = jest.fn((accts: Record<string, PublicKey>) => {
      recordedAccounts = accts;
      return { instruction };
    });
    const castVoteOverrideMethod = jest.fn(() => ({ accountsStrict }));

    return {
      programId: SVMGOV_PROGRAM_ID,
      provider: {
        connection: {
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
      methods: { castVoteOverride: castVoteOverrideMethod },
    };
  }

  function buildFakeGovV1Program() {
    const fakeIx = new TransactionInstruction({
      keys: [],
      programId: new PublicKey(keyFromByte(11)),
      data: Buffer.alloc(0),
    });
    const instruction = jest.fn(async () => fakeIx);
    const accountsStrict = jest.fn(() => ({ instruction }));
    const initMetaMerkleProofMethod = jest.fn(() => ({ accountsStrict }));

    return {
      methods: { initMetaMerkleProof: initMetaMerkleProofMethod },
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
    ],
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
    mockSignTransaction.mockResolvedValue(signedTransactionResponse());
    mockCreateProgramWithWallet.mockReturnValue(buildFakeProgram());
    mockCreateGovV1ProgramWithWallet.mockReturnValue(buildFakeGovV1Program());
    mockComputeProofCloseTimestamp.mockResolvedValue(2_000_000_000);
    mockGetAccountInfo.mockResolvedValue({ data: Buffer.alloc(0) });
    mockGetLatestBlockhash.mockResolvedValue({
      blockhash: BLOCKHASH,
      lastValidBlockHeight: 123,
    });
    mockSendRawTransaction.mockResolvedValue("test-signature");
    mockConfirmTransactionByPolling.mockResolvedValue({ value: { err: null } });
    mockGetMetaMerkleProofPda.mockReturnValue(META_MERKLE_PROOF_PDA);
    mockDeriveVotePda.mockReturnValue(VALIDATOR_VOTE_PDA);
    mockDeriveVoteOverridePda.mockReturnValue(VOTE_OVERRIDE_PDA);
    mockDeriveVoteOverrideCachePda.mockReturnValue(VOTE_OVERRIDE_CACHE_PDA);
    mockFetchProposal.mockResolvedValue({
      snapshotSlot: new BN(PROPOSAL_SNAPSHOT_SLOT),
      endEpoch: new BN(100),
    });

    // The verifier returns the validator the stake was delegated to AT SNAPSHOT TIME (A),
    // regardless of any later redelegation to B.
    mockGetStakeAccountProof.mockResolvedValue({
      network: "testnet",
      snapshot_slot: PROPOSAL_SNAPSHOT_SLOT,
      stake_merkle_leaf: {
        active_stake: 500,
        stake_account: STAKE_ACCOUNT,
        voting_wallet: DELEGATOR_WALLET,
      },
      stake_merkle_proof: [],
      vote_account: SNAPSHOT_VOTE_ACCOUNT,
    });
    mockGetVoteAccountProof.mockResolvedValue({
      network: "testnet",
      snapshot_slot: PROPOSAL_SNAPSHOT_SLOT,
      meta_merkle_leaf: {
        active_stake: 500,
        stake_merkle_root: STAKE_MERKLE_ROOT,
        vote_account: SNAPSHOT_VOTE_ACCOUNT,
        voting_wallet: VALIDATOR_WALLET,
      },
      meta_merkle_proof: [],
    });
  });

  const params = {
    proposalId: PROPOSAL,
    forVotesBp: 10_000,
    againstVotesBp: 0,
    abstainVotesBp: 0,
    stakeAccount: STAKE_ACCOUNT,
    wallet,
    consensusResult: new PublicKey(CONSENSUS_RESULT),
  };
  const blockchainParams = {
    network: "testnet" as RpcNetwork,
    endpoint: "http://localhost:8899",
    ncnApiUrl: "http://localhost:9000",
  };

  it("fetches proofs at the proposal snapshot slot, not a later network-wide /meta slot", async () => {
    const result = await castVoteOverride(params, blockchainParams);

    expect(result).toEqual({ signature: "test-signature", success: true });
    expect(mockFetchProposal).toHaveBeenCalled();
    expect(mockGetStakeAccountProof).toHaveBeenCalledWith(
      STAKE_ACCOUNT,
      "testnet",
      PROPOSAL_SNAPSHOT_SLOT,
      blockchainParams.ncnApiUrl,
    );
    expect(mockGetVoteAccountProof).toHaveBeenCalledWith(
      SNAPSHOT_VOTE_ACCOUNT,
      "testnet",
      PROPOSAL_SNAPSHOT_SLOT,
      blockchainParams.ncnApiUrl,
    );
    expect(mockGetStakeAccountProof).not.toHaveBeenCalledWith(
      expect.anything(),
      expect.anything(),
      LATEST_META_SLOT,
      expect.anything(),
    );
  });

  it("resolves the meta proof and PDAs from the stake proof's SNAPSHOT vote account, not the live delegation", async () => {
    const result = await castVoteOverride(params, blockchainParams);

    expect(result).toEqual({ signature: "test-signature", success: true });

    // The stake proof is fetched by stake account, from the configured ncn API.
    expect(mockGetStakeAccountProof).toHaveBeenCalledWith(
      STAKE_ACCOUNT,
      "testnet",
      PROPOSAL_SNAPSHOT_SLOT,
      blockchainParams.ncnApiUrl,
    );

    // The meta proof is fetched for the SNAPSHOT validator (A) carried by the stake proof —
    // never the live/redelegated validator (B). This is the core of the fix.
    expect(mockGetVoteAccountProof).toHaveBeenCalledWith(
      SNAPSHOT_VOTE_ACCOUNT,
      "testnet",
      PROPOSAL_SNAPSHOT_SLOT,
      blockchainParams.ncnApiUrl,
    );
    expect(mockGetVoteAccountProof).not.toHaveBeenCalledWith(
      LIVE_VOTE_ACCOUNT,
      expect.anything(),
      expect.anything(),
      expect.anything(),
    );

    const snapshotVote = new PublicKey(SNAPSHOT_VOTE_ACCOUNT);

    // The meta-proof PDA is derived from the meta proof whose leaf carries the snapshot validator.
    const metaProofArg = mockGetMetaMerkleProofPda.mock.calls[0][0] as {
      meta_merkle_leaf: { vote_account: string };
    };
    expect(metaProofArg.meta_merkle_leaf.vote_account).toBe(
      SNAPSHOT_VOTE_ACCOUNT,
    );

    // The validator_vote PDA is derived from the snapshot vote account (2nd positional arg),
    // which in turn binds the vote_override PDA — so a redelegated stake can no longer be paired
    // with the live validator.
    expect(mockDeriveVotePda).toHaveBeenCalledTimes(1);
    const votePdaVoteArg = mockDeriveVotePda.mock.calls[0][1] as PublicKey;
    expect(votePdaVoteArg.equals(snapshotVote)).toBe(true);

    // The instruction accounts use the snapshot validator and the snapshot-derived PDAs.
    expect(recordedAccounts.splVoteAccount.equals(snapshotVote)).toBe(true);
    expect(recordedAccounts.validatorVote.equals(VALIDATOR_VOTE_PDA)).toBe(
      true,
    );
    expect(recordedAccounts.metaMerkleProof.equals(META_MERKLE_PROOF_PDA)).toBe(
      true,
    );
    expect(recordedAccounts.voteOverride.equals(VOTE_OVERRIDE_PDA)).toBe(true);
  });

  it("throws when the verifier returns a meta proof whose vote account disagrees with the stake proof", async () => {
    // Defense-in-depth: if the verifier ever returns a meta proof for a different validator than
    // the stake proof's snapshot vote account, the override must be rejected client-side.
    mockGetVoteAccountProof.mockResolvedValue({
      network: "testnet",
      snapshot_slot: PROPOSAL_SNAPSHOT_SLOT,
      meta_merkle_leaf: {
        active_stake: 500,
        stake_merkle_root: STAKE_MERKLE_ROOT,
        vote_account: LIVE_VOTE_ACCOUNT,
        voting_wallet: VALIDATOR_WALLET,
      },
      meta_merkle_proof: [],
    });

    await expect(castVoteOverride(params, blockchainParams)).rejects.toThrow(
      /does not match meta proof vote account/,
    );
  });

  it("throws when the proposal has no snapshot slot yet", async () => {
    mockFetchProposal.mockResolvedValue({
      snapshotSlot: new BN(0),
      endEpoch: new BN(100),
    });

    await expect(castVoteOverride(params, blockchainParams)).rejects.toThrow(
      /no snapshot slot/,
    );
    expect(mockGetStakeAccountProof).not.toHaveBeenCalled();
  });

  it("initializes and confirms a missing meta proof in a separate transaction", async () => {
    mockGetAccountInfo.mockResolvedValue(null);
    mockSendRawTransaction
      .mockResolvedValueOnce("init-signature")
      .mockResolvedValueOnce("vote-signature");

    const result = await castVoteOverride(params, blockchainParams);

    expect(result).toEqual({ signature: "vote-signature", success: true });
    expect(mockSignTransaction).toHaveBeenCalledTimes(2);

    const initTransaction = mockSignTransaction.mock.calls[0][0];
    const voteTransaction = mockSignTransaction.mock.calls[1][0];
    expect(initTransaction.instructions).toHaveLength(1);
    expect(voteTransaction.instructions).toHaveLength(1);
    expect(initTransaction.instructions[0]).not.toBe(
      voteTransaction.instructions[0],
    );
    expect(mockConfirmTransactionByPolling).toHaveBeenCalledWith(
      expect.any(Object),
      "init-signature",
      123,
    );
    expect(
      mockConfirmTransactionByPolling.mock.invocationCallOrder[0],
    ).toBeLessThan(mockSignTransaction.mock.invocationCallOrder[1]);
  });

  it("does not submit the vote when proof initialization fails", async () => {
    mockGetAccountInfo.mockResolvedValue(null);
    mockSendRawTransaction.mockResolvedValue("init-signature");
    mockConfirmTransactionByPolling.mockResolvedValue({
      value: { err: { InstructionError: [0, "Custom"] } },
    });

    await expect(castVoteOverride(params, blockchainParams)).rejects.toThrow(
      /Failed to initialize meta merkle proof/,
    );
    expect(mockSignTransaction).toHaveBeenCalledTimes(1);
    expect(mockSendRawTransaction).toHaveBeenCalledTimes(1);
  });

  it("reports an unsigned wallet response without submitting the transaction", async () => {
    mockSignTransaction.mockResolvedValueOnce(
      signedTransactionResponse([
        { publicKey: new PublicKey(SIGNER), signature: null },
      ]),
    );

    await expect(castVoteOverride(params, blockchainParams)).rejects.toThrow(
      /wallet did not sign the transaction/i,
    );
    expect(mockSendRawTransaction).not.toHaveBeenCalled();
  });

  it("reports a returned signature from a different account", async () => {
    const otherAccount = new PublicKey(keyFromByte(12));
    mockSignTransaction.mockResolvedValueOnce(
      signedTransactionResponse([
        { publicKey: new PublicKey(SIGNER), signature: null },
        { publicKey: otherAccount, signature: Buffer.alloc(64) },
      ]),
    );

    await expect(castVoteOverride(params, blockchainParams)).rejects.toThrow(
      /signed with a different account/i,
    );
    expect(mockSendRawTransaction).not.toHaveBeenCalled();
  });
});
