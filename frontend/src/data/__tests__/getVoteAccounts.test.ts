import { PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";

import { mapVoteAccountDto } from "../getVoteAccounts";
import type { RawVoteAccountDataAccount } from "@/types";

const key = (byte: number) => new PublicKey(new Uint8Array(32).fill(byte));

function rawVoteAccount(): RawVoteAccountDataAccount {
  return {
    publicKey: key(3),
    account: {
      validator: key(1),
      proposal: key(2),
      forVotesBp: new BN(10_000),
      againstVotesBp: new BN(0),
      abstainVotesBp: new BN(0),
      forVotesLamports: new BN(25_000_000_000),
      againstVotesLamports: new BN(0),
      abstainVotesLamports: new BN(0),
      stake: new BN(25_000_000_000),
      overrideLamports: new BN(0),
      voteTimestamp: new BN(1_700_000_000),
      bump: 254,
    },
  } as unknown as RawVoteAccountDataAccount;
}

describe("mapVoteAccountDto", () => {
  it("leaves validator metadata unset", () => {
    // None of this is on chain. Filling it with zeros renders a 0% commission
    // validator with no credits, which reads as real data rather than as
    // metadata that has not been joined yet.
    const mapped = mapVoteAccountDto(rawVoteAccount());

    expect(mapped.commission).toBeUndefined();
    expect(mapped.lastVote).toBeUndefined();
    expect(mapped.credits).toBeUndefined();
    expect(mapped.epochCredits).toBeUndefined();
    expect(mapped.activatedStake).toBeUndefined();
  });

  it("carries the fields the table needs from the account", () => {
    // The proposal and the ballot are both on the record, so a row can be
    // attributed to a proposal and show which way it voted.
    const mapped = mapVoteAccountDto(rawVoteAccount());

    expect(mapped.proposal.equals(key(2))).toBe(true);
    expect(mapped.identity?.equals(key(1))).toBe(true);
    expect(mapped.forVotesBp.toNumber()).toBe(10_000);
    expect(mapped.activeStake).toBe(25_000_000_000);
  });

  it("uses the record's own address as the row key", () => {
    // This is the Vote PDA, not the validator's vote account.
    expect(mapVoteAccountDto(rawVoteAccount()).voteAccount.equals(key(3))).toBe(
      true,
    );
  });
});
