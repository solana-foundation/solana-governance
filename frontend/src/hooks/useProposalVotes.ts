import { useQuery } from "@tanstack/react-query";
import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import { TopVoterRecord } from "@/types/topVoters";
import { Validator } from "@/types";
import { getProposalVotes, getProposalVoteOverrides } from "@/data";
import { useEndpoint } from "@/contexts/EndpointContext";
import { useGetValidators } from "./useGetValidators";

const accentColors = [
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

const getColorFromString = (str: string): string => {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i);
    hash = (hash << 5) - hash + char;
    hash = hash & hash;
  }
  const index = Math.abs(hash) % accentColors.length;
  return accentColors[index];
};

export const useProposalVotes = (proposalPublicKey: PublicKey | undefined) => {
  const { endpointUrl: endpoint } = useEndpoint();
  const { data: validators } = useGetValidators();

  return useQuery({
    queryKey: ["proposal-votes", proposalPublicKey?.toBase58(), endpoint],
    staleTime: 1000 * 120, // 2 minutes
    enabled: !!proposalPublicKey,
    queryFn: async (): Promise<TopVoterRecord[]> => {
      if (!proposalPublicKey) {
        throw new Error("Missing proposal public key");
      }

      // 1. Fetch votes and voteOverride data from governance program
      const [votes, voteOverrides] = await Promise.all([
        getProposalVotes(proposalPublicKey, endpoint),
        getProposalVoteOverrides(proposalPublicKey, endpoint),
      ]);

      // 2. Optionally fetch validator details (name, etc.)
      // If you don't need detailed mapping, you might skip this, or use a static list.
      let validatorMap: Record<string, Validator> = {};
      if (validators) {
        validatorMap = Object.fromEntries(
          validators.map((v) => [v.vote_identity, v])
        );
      }

      // Total stake is calculated from all validators' activated_stake
      const totalStakedLamports = validators
        ? validators.reduce((sum, v) => sum + (v.activated_stake || 0), 0)
        : 0;

      // 5. Map to TopVoterRecord[]
      const validatorVoters = votes.map((v) => {
        const identity = v.identity?.toBase58
          ? v.identity.toBase58()
          : typeof v.identity === "string"
          ? v.identity
          : "unknown";
        const validator = validatorMap[identity];
        const validatorName = validator?.name || "Unknown Validator";
        const stakedLamports = v.activeStake || 0;
        const votePercentage =
          totalStakedLamports > 0 && stakedLamports > 0
            ? (stakedLamports / totalStakedLamports) * 100
            : 0;
        // If voteTimestamp is unix/BN, convert to string
        let voteTimestamp: string;
        if (v.voteTimestamp && typeof v.voteTimestamp.toNumber === "function") {
          voteTimestamp = new Date(
            v.voteTimestamp.toNumber() * 1000
          ).toISOString();
        } else if (typeof v.voteTimestamp === "number") {
          voteTimestamp = new Date(v.voteTimestamp * 1000).toISOString();
        } else {
          voteTimestamp = new Date().toISOString();
        }

        return {
          id: identity,
          validatorName,
          validatorIdentity: identity,
          stakedLamports,
          votePercentage,
          voteTimestamp,
          voteData: {
            forVotesBp: v.forVotesBp ? new BN(v.forVotesBp) : new BN(0),
            againstVotesBp: v.againstVotesBp
              ? new BN(v.againstVotesBp)
              : new BN(0),
            abstainVotesBp: v.abstainVotesBp
              ? new BN(v.abstainVotesBp)
              : new BN(0),
          },
          accentColor: getColorFromString(validatorName),
          walletType: "validator" as const,
        };
      });

      const stakerVoters = voteOverrides.map((v) => {
        const identity = v.identity?.toBase58
          ? v.identity.toBase58()
          : typeof v.identity === "string"
          ? v.identity
          : "unknown";

        const validator = validatorMap[identity];
        const validatorName = validator?.name || "Unknown Validator";
        const stakedLamports = v.activeStake || 0;
        const votePercentage =
          totalStakedLamports > 0 && stakedLamports > 0
            ? (stakedLamports / totalStakedLamports) * 100
            : 0;
        // If voteTimestamp is unix/BN, convert to string
        let voteTimestamp: string;
        if (v.voteTimestamp && typeof v.voteTimestamp.toNumber === "function") {
          voteTimestamp = new Date(
            v.voteTimestamp.toNumber() * 1000
          ).toISOString();
        } else if (typeof v.voteTimestamp === "number") {
          voteTimestamp = new Date(v.voteTimestamp * 1000).toISOString();
        } else {
          voteTimestamp = new Date().toISOString();
        }

        return {
          id: identity,
          validatorName,
          validatorIdentity: identity,
          stakedLamports,
          votePercentage,
          voteTimestamp,
          stakeAccount: v.stakeAccount.toBase58(),
          voteData: {
            forVotesBp: v.forVotesBp ? new BN(v.forVotesBp) : new BN(0),
            againstVotesBp: v.againstVotesBp
              ? new BN(v.againstVotesBp)
              : new BN(0),
            abstainVotesBp: v.abstainVotesBp
              ? new BN(v.abstainVotesBp)
              : new BN(0),
          },
          accentColor: getColorFromString(validatorName),
          walletType: "staker" as const,
        };
      });

      const topVoters = [...validatorVoters, ...stakerVoters];

      return topVoters;
    },
  });
};
