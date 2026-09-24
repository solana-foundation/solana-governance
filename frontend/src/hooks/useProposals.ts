import { useEndpoint } from "@/contexts/EndpointContext";
import { useGovernanceConfigContext } from "@/contexts/GovernanceConfigContext";
import { getProposals } from "@/data";
import { GET_ALL_PROPOSALS } from "@/helpers";
import { useQuery } from "@tanstack/react-query";
import { useEpochInfo } from "./useEpochInfo";
import { useRawVoteAccounts } from "./useRawVoteAccounts";

const CONSENSUS_PENDING_REFETCH_MS = 30 * 1000;

export const useProposals = (filters?: {
  voting?: boolean;
  finalized?: boolean;
}) => {
  const { endpointUrl: endpoint } = useEndpoint();
  const governanceConfigQuery = useGovernanceConfigContext();
  const { data: epochData, isLoading: isLoadingEpochInfo } = useEpochInfo();
  const { data: voteAccountsData, isLoading: isLoadingVoteAccounts } =
    useRawVoteAccounts();

  const governanceConfig = governanceConfigQuery.data;
  const governanceConfigKey = governanceConfig
    ? [
        governanceConfig.maxSupportEpochs,
        governanceConfig.discussionEpochs,
        governanceConfig.snapshotEpochExtension,
        governanceConfig.votingEpochs,
      ]
    : null;

  const query = useQuery({
    staleTime: 1000 * 120, // 2 minutes
    // While a proposal in its voting window is still waiting for the NCN
    // snapshot consensus (or its lookup failed), poll so the status flips to
    // "voting" without a reload once the ConsensusResult account appears.
    refetchInterval: (query) =>
      query.state.data?.some((proposal) => !proposal.consensusReached)
        ? CONSENSUS_PENDING_REFETCH_MS
        : false,
    queryKey: [
      GET_ALL_PROPOSALS,
      endpoint,
      filters,
      epochData?.epochInfo.epoch,
      voteAccountsData?.current.length,
      governanceConfigKey,
    ],
    queryFn: () => {
      if (!epochData) {
        throw new Error("Epoch info not available");
      }
      if (!voteAccountsData) {
        throw new Error("Vote accounts not available");
      }
      if (!governanceConfig) {
        throw new Error("Governance config not available");
      }
      return getProposals(
        endpoint,
        filters,
        epochData.epochInfo,
        voteAccountsData,
        governanceConfig,
      );
    },
    enabled:
      !!epochData &&
      !!voteAccountsData &&
      governanceConfigQuery.isSuccess &&
      !!governanceConfig,
  });

  const isLoading =
    isLoadingEpochInfo ||
    isLoadingVoteAccounts ||
    governanceConfigQuery.isLoading ||
    governanceConfigQuery.isPending ||
    query.isLoading;
  return { ...query, isLoading };
};
