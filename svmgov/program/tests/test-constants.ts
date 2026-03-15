import { randomBytes } from "crypto";

// Use BN from bn.js directly since anchor.BN is not available in test environment
import BN from "bn.js";

// Shared constants
export const SNAPSHOT_SLOT = new BN(1000000);
export const MERKLE_ROOT_HASH = Array.from(randomBytes(32));
export const BALLOT_ID = new BN(12345);

// Test data for MetaMerkleProof leaves
export const createTestLeaf = (votingWallet: any, voteAccount: any) => ({
  votingWallet,
  voteAccount,
  stakeMerkleRoot: MERKLE_ROOT_HASH,
  activeStake: new BN(100_000 * 1000000000), // 100 SOL in lamports
});

// Test data for proofs (dummy data)
export const createTestProof = () => [
  Array.from(randomBytes(32)),
  Array.from(randomBytes(32))
];

// Vote parameters for testing
export const TEST_VOTE_PARAMS = {
  for: new BN(4_000),
  against: new BN(4_000),
  abstain: new BN(2_000),
};

export const TEST_VOTE_MODIFY_PARAMS = {
  for: new BN(4_000),
  against: new BN(2_000),
  abstain: new BN(4_000),
};

export const TEST_VOTE_OVERRIDE_PARAMS = {
  for: new BN(7_000),
  against: new BN(3_000),
  abstain: new BN(0),
};

export const TEST_VOTE_OVERRIDE_MODIFY_PARAMS = {
  for: new BN(5_000),
  against: new BN(2_000),
  abstain: new BN(3_000),
};

// Proposal creation parameters
export const TEST_PROPOSAL_PARAMS = {
  title: "Proposal1",
  description: "https://github.com/repo/test-proposal",
};

// Error test parameters
export const ERROR_TEST_PARAMS = {
  emptyTitle: "",
  emptyDescription: "",
  invalidDescription: "not a github link",
  overflowValue: new BN("18446744073709551616"), // u64::MAX + 1
};
