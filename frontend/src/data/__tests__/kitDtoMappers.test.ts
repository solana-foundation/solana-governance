import type { Proposal, Vote, VoteOverride } from "@/lib/governance/programAccounts";
import { address } from "@solana/kit";
import { mapProposalDto } from "../getProposals";
import { mapVoteAccountDto } from "../getVoteAccounts";
import { mapVoteOverrideAccountDto } from "../getVoteOverrideAccounts";

const ADDRESS = address("11111111111111111111111111111111");
const UNSAFE_INTEGER = 9_007_199_254_740_993n;

describe("Kit DTO mappers", () => {
  it("keeps vote accounts as Address and bigint values", () => {
    const vote = mapVoteAccountDto({
      proposal: ADDRESS,
      validator: ADDRESS,
      forVotesBp: 6_000n,
      againstVotesBp: 2_000n,
      abstainVotesBp: 2_000n,
      forVotesLamports: 10n,
      againstVotesLamports: 20n,
      abstainVotesLamports: 30n,
      stake: 40n,
      overrideLamports: 50n,
      voteTimestamp: 60n,
      bump: 1,
    } as Vote, ADDRESS);

    expect(vote.voteAccount).toBe(ADDRESS);
    expect(vote.proposal).toBe(ADDRESS);
    expect(vote.forVotesBp).toBe(6_000n);
    expect(vote.voteTimestamp).toBe(60n);
  });

  it("keeps vote overrides as Address and bigint values", () => {
    const override = mapVoteOverrideAccountDto({
      delegator: ADDRESS,
      stakeAccount: ADDRESS,
      validator: ADDRESS,
      proposal: ADDRESS,
      voteAccountValidator: ADDRESS,
      forVotesBp: 6_000n,
      againstVotesBp: 2_000n,
      abstainVotesBp: 2_000n,
      forVotesLamports: 10n,
      againstVotesLamports: 20n,
      abstainVotesLamports: 30n,
      stakeAmount: 40n,
      voteOverrideTimestamp: 50n,
      bump: 1,
    } as VoteOverride, ADDRESS);

    expect(override.publicKey).toBe(ADDRESS);
    expect(override.stakeAccount).toBe(ADDRESS);
    expect(override.forVotesBp).toBe(6_000n);
    expect(override.voteOverrideTimestamp).toBe(50n);
  });

  it("keeps proposal identifiers as Address values", () => {
    const proposal = mapProposalDto({
      author: ADDRESS,
      title: "Title",
      description: "https://github.com/solana-foundation/solana-governance-proposals/blob/main/proposals/sgp-0001.md",
      creationEpoch: 1n,
      startEpoch: 2n,
      endEpoch: 3n,
      proposerStakeWeightBp: 0n,
      clusterSupportLamports: UNSAFE_INTEGER,
      forVotesLamports: UNSAFE_INTEGER,
      againstVotesLamports: UNSAFE_INTEGER,
      abstainVotesLamports: UNSAFE_INTEGER,
      voting: false,
      finalized: false,
      proposalBump: 1,
      creationTimestamp: UNSAFE_INTEGER,
      voteCount: 0,
      index: 0,
      consensusResult: { __option: "Some", value: ADDRESS },
      snapshotSlot: UNSAFE_INTEGER,
      proposalSeed: 1n,
      voteAccountPubkey: ADDRESS,
      numSupporters: 0,
    } as Proposal, ADDRESS, 0, 1n, 0n, {
      SUPPORT_EPOCHS: 1n,
      DISCUSSION_EPOCHS: 1n,
      SNAPSHOT_EPOCHS: 1n,
      VOTING_EPOCHS: 1n,
    }, 1_500n);

    expect(proposal.publicKey).toBe(ADDRESS);
    expect(proposal.consensusResult).toBe(ADDRESS);
    expect(proposal.clusterSupportLamports).toBe(UNSAFE_INTEGER);
    expect(proposal.forVotesLamports).toBe(UNSAFE_INTEGER);
    expect(proposal.creationTimestamp).toBe(UNSAFE_INTEGER);
    expect(proposal.snapshotSlot).toBe(UNSAFE_INTEGER);
  });
});
