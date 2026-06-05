import BN from "bn.js";

export interface TopVoterRecord {
  id: string;
  validatorName: string;
  validatorIdentity: string;
  validatorImage?: string | null;
  stakeAccount?: string;
  stakedLamports: number;
  // voteOutcome: VoteOutcome;
  votePercentage: number;
  voteTimestamp: string;
  voteData: {
    forVotesBp: BN;
    againstVotesBp: BN;
    abstainVotesBp: BN;
  };
  accentColor: string;
  walletType: "validator" | "staker";
}

export const accentColors = [
  "linear-gradient(135deg, #a855f7 0%, #7c3aed 100%)",
  "linear-gradient(135deg, #06b6d4 0%, #0ea5e9 100%)",
  "linear-gradient(135deg, #f97316 0%, #ea580c 100%)",
  "linear-gradient(135deg, #fb7185 0%, #f43f5e 100%)",
  "linear-gradient(135deg, #22d3ee 0%, #0891b2 100%)",
  "linear-gradient(135deg, #84cc16 0%, #65a30d 100%)",
  "linear-gradient(135deg, #fbbf24 0%, #f59e0b 100%)",
  "linear-gradient(135deg, #8b5cf6 0%, #6d28d9 100%)",
  "linear-gradient(135deg, #ec4899 0%, #db2777 100%)",
  "linear-gradient(135deg, #10b981 0%, #059669 100%)",
  "linear-gradient(135deg, #3b82f6 0%, #2563eb 100%)",
  "linear-gradient(135deg, #ef4444 0%, #dc2626 100%)",
  "linear-gradient(135deg, #14b8a6 0%, #0d9488 100%)",
  "linear-gradient(135deg, #6366f1 0%, #4f46e5 100%)",
];
