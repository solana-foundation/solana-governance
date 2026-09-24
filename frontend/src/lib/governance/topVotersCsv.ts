import type { TopVoterRecord } from "@/types";

const HEADERS = [
  "Validator Name",
  "Validator Identity",
  "Voted As",
  "Stake Account",
  "Staked Lamports",
  "For (%)",
  "Against (%)",
  "Abstain (%)",
  "Vote Percentage",
  "Vote Timestamp",
];

function escapeCsvCell(value: string | number): string {
  const text = String(value);
  return /[",\r\n]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text;
}

function basisPointsToPercentage(
  value: TopVoterRecord["voteData"]["forVotesBp"],
): number {
  return value.toNumber() / 100;
}

export function createTopVotersCsv(voters: TopVoterRecord[]): string {
  const rows = voters.map((voter) => [
    voter.validatorName,
    voter.validatorIdentity,
    voter.walletType,
    voter.stakeAccount ?? "",
    voter.stakedLamports,
    basisPointsToPercentage(voter.voteData.forVotesBp),
    basisPointsToPercentage(voter.voteData.againstVotesBp),
    basisPointsToPercentage(voter.voteData.abstainVotesBp),
    voter.votePercentage,
    voter.voteTimestamp,
  ]);

  return [HEADERS, ...rows]
    .map((row) => row.map(escapeCsvCell).join(","))
    .join("\r\n");
}
