import { PublicKey, TransactionInstruction } from "@solana/web3.js";

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
const mockGetMetaMerkleProofPda = jest.fn();
const mockDeriveVotePda = jest.fn();
const mockDeriveVoteOverridePda = jest.fn();
const mockDeriveVoteOverrideCachePda = jest.fn();

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
    getMetaMerkleProofPda: (...args: unknown[]) =>
      mockGetMetaMerkleProofPda(...args),
    deriveVotePda: (...args: unknown[]) => mockDeriveVotePda(...args),
    deriveVoteOverridePda: (...args: unknown[]) =>
      mockDeriveVoteOverridePda(...args),
    deriveVoteOverrideCachePda: (...args: unknown[]) =>
      mockDeriveVoteOverrideCachePda(...args),
  };
});

import type { AnchorWallet } from "@solana/wallet-adapter-react";

import { castVoteOverride } from "../castVoteOverride";
import { SVMGOV_PROGRAM_ID } from "../types";
import type { RPCEndpoint } from "@/types";

// Distinct, valid 32-byte public keys used as stand-ins (byte-filled so PDA derivation always
// resolves a viable nonce).
const keyFromByte = (b: number): string =>
  new PublicKey(new Uint8Array(32).fill(b)).toBase58();
const SNAPSHOT_VOTE_ACCOUNT = keyFromByte(1); // validator A at snapshot
const LIVE_VOTE_ACCOUNT = keyFromByte(2); // validator B (post-snapshot redelegation)
const STAKE_ACCOUNT = keyFromByte(3);
const VOTING_WALLET = keyFromByte(4);
const STAKE_MERKLE_ROOT = keyFromByte(5);
const CONSENSUS_RESULT = keyFromByte(6);
const PROPOSAL = keyFromByte(7);
const SIGNER = keyFromByte(8);
const BLOCKHASH = keyFromByte(9);

// Stand-in return values for the mocked PDA-derivation helpers.
const META_MERKLE_PROOF_PDA = new PublicKey(keyFromByte(20));
const VALIDATOR_VOTE_PDA = new PublicKey(keyFromByte(21));
const VOTE_OVERRIDE_PDA = new PublicKey(keyFromByte(22));
const VOTE_OVERRIDE_CACHE_PDA = new PublicKey(keyFromByte(23));

describe("castVoteOverride", () => {
  let recordedAccounts: Record<string, PublicKey>;

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
          // Truthy account info => the MetaMerkleProof already exists, so the init branch (which
          // needs the gov-v1 program / proposal fetch / block time) is skipped.
          getAccountInfo: jest.fn(async () => ({ data: Buffer.alloc(0) })),
          getLatestBlockhash: jest.fn(async () => ({ blockhash: BLOCKHASH })),
          sendRawTransaction: jest.fn(async () => "test-signature"),
        },
      },
      methods: { castVoteOverride: castVoteOverrideMethod },
    };
  }

  const wallet = {
    publicKey: new PublicKey(SIGNER),
    signTransaction: jest.fn(async () => ({ serialize: () => Buffer.alloc(0) })),
    signAllTransactions: jest.fn(),
  } as unknown as AnchorWallet;

  beforeEach(() => {
    jest.clearAllMocks();
    mockCreateProgramWithWallet.mockReturnValue(buildFakeProgram());
    mockGetMetaMerkleProofPda.mockReturnValue(META_MERKLE_PROOF_PDA);
    mockDeriveVotePda.mockReturnValue(VALIDATOR_VOTE_PDA);
    mockDeriveVoteOverridePda.mockReturnValue(VOTE_OVERRIDE_PDA);
    mockDeriveVoteOverrideCachePda.mockReturnValue(VOTE_OVERRIDE_CACHE_PDA);

    // The verifier returns the validator the stake was delegated to AT SNAPSHOT TIME (A),
    // regardless of any later redelegation to B.
    mockGetStakeAccountProof.mockResolvedValue({
      network: "testnet",
      snapshot_slot: 340_850_340,
      stake_merkle_leaf: {
        active_stake: 500,
        stake_account: STAKE_ACCOUNT,
        voting_wallet: VOTING_WALLET,
      },
      stake_merkle_proof: [],
      vote_account: SNAPSHOT_VOTE_ACCOUNT,
    });
    mockGetVoteAccountProof.mockResolvedValue({
      network: "testnet",
      snapshot_slot: 340_850_340,
      meta_merkle_leaf: {
        active_stake: 500,
        stake_merkle_root: STAKE_MERKLE_ROOT,
        vote_account: SNAPSHOT_VOTE_ACCOUNT,
        voting_wallet: VOTING_WALLET,
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
    network: "testnet" as RPCEndpoint,
    endpoint: "http://localhost:8899",
  };
  const slot = 340_850_340;

  it("resolves the meta proof and PDAs from the stake proof's SNAPSHOT vote account, not the live delegation", async () => {
    const result = await castVoteOverride(params, blockchainParams, slot);

    expect(result).toEqual({ signature: "test-signature", success: true });

    // The stake proof is fetched by stake account.
    expect(mockGetStakeAccountProof).toHaveBeenCalledWith(
      STAKE_ACCOUNT,
      "testnet",
      slot
    );

    // The meta proof is fetched for the SNAPSHOT validator (A) carried by the stake proof —
    // never the live/redelegated validator (B). This is the core of the fix.
    expect(mockGetVoteAccountProof).toHaveBeenCalledWith(
      SNAPSHOT_VOTE_ACCOUNT,
      "testnet",
      slot
    );
    expect(mockGetVoteAccountProof).not.toHaveBeenCalledWith(
      LIVE_VOTE_ACCOUNT,
      expect.anything(),
      expect.anything()
    );

    const snapshotVote = new PublicKey(SNAPSHOT_VOTE_ACCOUNT);

    // The meta-proof PDA is derived from the meta proof whose leaf carries the snapshot validator.
    const metaProofArg = mockGetMetaMerkleProofPda.mock.calls[0][0] as {
      meta_merkle_leaf: { vote_account: string };
    };
    expect(metaProofArg.meta_merkle_leaf.vote_account).toBe(
      SNAPSHOT_VOTE_ACCOUNT
    );

    // The validator_vote PDA is derived from the snapshot vote account (2nd positional arg),
    // which in turn binds the vote_override PDA — so a redelegated stake can no longer be paired
    // with the live validator.
    expect(mockDeriveVotePda).toHaveBeenCalledTimes(1);
    const votePdaVoteArg = mockDeriveVotePda.mock.calls[0][1] as PublicKey;
    expect(votePdaVoteArg.equals(snapshotVote)).toBe(true);

    // The instruction accounts use the snapshot validator and the snapshot-derived PDAs.
    expect(recordedAccounts.splVoteAccount.equals(snapshotVote)).toBe(true);
    expect(recordedAccounts.validatorVote.equals(VALIDATOR_VOTE_PDA)).toBe(true);
    expect(recordedAccounts.metaMerkleProof.equals(META_MERKLE_PROOF_PDA)).toBe(
      true
    );
    expect(recordedAccounts.voteOverride.equals(VOTE_OVERRIDE_PDA)).toBe(true);
  });

  it("throws when the verifier returns a meta proof whose vote account disagrees with the stake proof", async () => {
    // Defense-in-depth: if the verifier ever returns a meta proof for a different validator than
    // the stake proof's snapshot vote account, the override must be rejected client-side.
    mockGetVoteAccountProof.mockResolvedValue({
      network: "testnet",
      snapshot_slot: 340_850_340,
      meta_merkle_leaf: {
        active_stake: 500,
        stake_merkle_root: STAKE_MERKLE_ROOT,
        vote_account: LIVE_VOTE_ACCOUNT,
        voting_wallet: VOTING_WALLET,
      },
      meta_merkle_proof: [],
    });

    await expect(
      castVoteOverride(params, blockchainParams, slot)
    ).rejects.toThrow(/does not match meta proof vote account/);
  });
});
