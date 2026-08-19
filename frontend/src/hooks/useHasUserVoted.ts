import { useEndpoint } from "@/contexts/EndpointContext";
import { getUserHasVoted, GetVoteOverrideFilters } from "@/data";
import { GET_USER_HAS_VOTED } from "@/helpers";
import { useWalletSession } from "@/contexts/WalletSessionContext";
import { useQuery } from "@tanstack/react-query";
import { PublicKey } from "@solana/web3.js";
import { useVoteOverrideAccounts } from "./useVoteOverrideAccounts";
import { useWalletRole } from "./useWalletRole";
import { WalletRole } from "@/types";
import { useValidatorProposalVoteAccount } from "./useValidatorProposalVoteAccount";

/**
 * Builds vote override filters for a specific proposal and delegator
 */
function buildVoteOverrideFilters(
  proposalPublicKey: string | undefined,
  delegatorPublicKey: PublicKey | null
): GetVoteOverrideFilters {
  const filters: GetVoteOverrideFilters = [];

  if (proposalPublicKey) {
    filters.push({
      name: "proposal" as const,
      value: proposalPublicKey,
    });
  }

  if (delegatorPublicKey) {
    filters.push({
      name: "delegator" as const,
      value: delegatorPublicKey.toBase58(),
    });
  }

  return filters;
}

export const useHasUserVoted = (
  proposalPublicKey: string | undefined,
  enabledProp = true
) => {
  const { endpointUrl: endpoint } = useEndpoint();
  const { publicKey, connected } = useWalletSession();

  const { walletRole } = useWalletRole(publicKey?.toBase58());

  const isValidator = walletRole === WalletRole.VALIDATOR;
  const isStaker = walletRole === WalletRole.STAKER;
  const isBoth = walletRole === WalletRole.BOTH;

  const fetchVoteAccountsEnabled = isBoth || isValidator;

  const voteOverrideFilters = buildVoteOverrideFilters(
    proposalPublicKey,
    publicKey
  );

  const fetchVoteOverrideEnabled =
    (isBoth || isStaker) && voteOverrideFilters.length > 0; // at least one filter is required

  const {
    data: voteOverrideAccounts = [],
    isLoading: isLoadingVoteOverrideAccounts,
  } = useVoteOverrideAccounts(voteOverrideFilters, fetchVoteOverrideEnabled);

  const { data: voteAccount, isLoading: isLoadingVoteAccount } =
    useValidatorProposalVoteAccount(
      proposalPublicKey,
      fetchVoteAccountsEnabled
    );

  const enabled =
    connected &&
    !!publicKey &&
    enabledProp &&
    !!proposalPublicKey &&
    !isLoadingVoteOverrideAccounts &&
    !isLoadingVoteAccount;

  const query = useQuery({
    queryKey: [
      GET_USER_HAS_VOTED,
      endpoint,
      proposalPublicKey,
      voteOverrideAccounts.length,
      !!voteAccount,
    ],
    enabled,
    staleTime: 1000 * 120, // 2 minutes
    queryFn: () => getUserHasVoted(voteOverrideAccounts, voteAccount),
  });

  const isLoading = isLoadingVoteOverrideAccounts || query.isLoading;

  return { ...query, isLoading };
};
