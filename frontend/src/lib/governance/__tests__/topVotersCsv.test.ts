import BN from "bn.js";
import { createTopVotersCsv } from "../topVotersCsv";
import type { TopVoterRecord } from "@/types";

const voter: TopVoterRecord = {
  id: "validator-1",
  validatorName: 'Validator, "North"\nNode',
  validatorIdentity: "validator-address",
  stakeAccount: "stake-address",
  stakedLamports: 1_500_000_000,
  votePercentage: 12.5,
  voteTimestamp: "2026-08-28T12:34:56.000Z",
  voteData: {
    forVotesBp: new BN(7_500),
    againstVotesBp: new BN(2_000),
    abstainVotesBp: new BN(500),
  },
  accentColor: "#000",
  walletType: "staker",
};

describe("createTopVotersCsv", () => {
  it("serializes voter details and escapes RFC 4180 special characters", () => {
    expect(createTopVotersCsv([voter])).toBe(
      [
        "Validator Name,Validator Identity,Voted As,Stake Account,Staked Lamports,For (%),Against (%),Abstain (%),Vote Percentage,Vote Timestamp",
        '"Validator, ""North""\nNode",validator-address,staker,stake-address,1500000000,75,20,5,12.5,2026-08-28T12:34:56.000Z',
      ].join("\r\n"),
    );
  });

  it("uses an empty stake-account cell for validator votes", () => {
    expect(
      createTopVotersCsv([
        { ...voter, walletType: "validator", stakeAccount: undefined },
      ]).split("\r\n")[1],
    ).toContain("validator,,1500000000");
  });
});
