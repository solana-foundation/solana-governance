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

const FORMULA_PREFIX = /^[=+\-@\uFF1D\uFF0B\uFF0D\uFF20]/u;
const CONTROL_PREFIX = /^[\t\r\n\0]/u;

function neutralizeSpreadsheetFormula(text: string): string {
  const startsWithFormula = FORMULA_PREFIX.test(text.trimStart());
  return startsWithFormula || CONTROL_PREFIX.test(text) ? `'${text}` : text;
}

function escapeCsvCell(value: string | number): string {
  const text =
    typeof value === "string"
      ? neutralizeSpreadsheetFormula(value)
      : String(value);
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
