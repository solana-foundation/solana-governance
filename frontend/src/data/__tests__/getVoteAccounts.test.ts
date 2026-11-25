import { getAddressDecoder, type Address } from "@solana/kit";

import { mapVoteAccountDto } from "../getVoteAccounts";
import type { Vote } from "@/lib/governance/programAccounts";

const key = (byte: number): Address =>
  getAddressDecoder().decode(new Uint8Array(32).fill(byte));

function rawVoteAccount(): Vote {
  return {
    validator: key(1),
    proposal: key(2),
    forVotesBp: 10_000n,
    againstVotesBp: 0n,
    abstainVotesBp: 0n,
    forVotesLamports: 25_000_000_000n,
    againstVotesLamports: 0n,
    abstainVotesLamports: 0n,
    stake: 25_000_000_000n,
    overrideLamports: 0n,
    voteTimestamp: 1_700_000_000n,
    bump: 254,
  } as Vote;
}

describe("mapVoteAccountDto", () => {
  it("leaves validator metadata unset", () => {
    const mapped = mapVoteAccountDto(rawVoteAccount(), key(3));

    expect(mapped.commission).toBeUndefined();
    expect(mapped.lastVote).toBeUndefined();
    expect(mapped.credits).toBeUndefined();
    expect(mapped.epochCredits).toBeUndefined();
    expect(mapped.activatedStake).toBeUndefined();
  });

  it("carries the fields the table needs from the account", () => {
    // The proposal and the ballot are both on the record, so a row can be
    // attributed to a proposal and show which way it voted.
    const mapped = mapVoteAccountDto(rawVoteAccount(), key(3));

    expect(mapped.proposal).toBe(key(2));
    expect(mapped.identity).toBe(key(1));
    expect(mapped.forVotesBp).toBe(10_000n);
    expect(mapped.activeStake).toBe(25_000_000_000n);
  });

  it("uses the record's own address as the row key", () => {
    // This is the Vote PDA, not the validator's vote account.
    expect(mapVoteAccountDto(rawVoteAccount(), key(3)).voteAccount).toBe(key(3));
  });
});
