import { useEndpoint } from "@/contexts/EndpointContext";
import { getUserHasSupported } from "@/data";
import { GET_USER_HAS_SUPPORTED } from "@/helpers";
import { useWalletSession } from "@/contexts/WalletSessionContext";
import { useQuery } from "@tanstack/react-query";
import { buildSupportFilters, useSupportAccounts } from "./useSupportAccounts";
import { useWalletRole } from "./useWalletRole";
import { WalletRole } from "@/types";

export const useHasUserSupported = (
  proposalPublicKey: string | undefined,
  enabledProp = true
) => {
  const { endpointUrl: endpoint } = useEndpoint();
  const { publicKey, connected } = useWalletSession();

  const { walletRole } = useWalletRole(publicKey?.toBase58());

  const isValidator = walletRole === WalletRole.VALIDATOR;
  const isBoth = walletRole === WalletRole.BOTH;

  const fetchSupportEnabled = isBoth || isValidator;

  const supportFilters = buildSupportFilters(proposalPublicKey, publicKey);

  const fetchSupportAccountsEnabled =
    fetchSupportEnabled && supportFilters.length > 0; // at least one filter is required

  const { data: supportAccounts = [], isLoading: isLoadingSupportAccounts } =
    useSupportAccounts(supportFilters, fetchSupportAccountsEnabled);

  const enabled =
    connected &&
    !!publicKey &&
    enabledProp &&
    !!proposalPublicKey &&
    !isLoadingSupportAccounts;

  const query = useQuery({
    queryKey: [
      GET_USER_HAS_SUPPORTED,
      endpoint,
      proposalPublicKey,
      supportAccounts.length,
    ],
    enabled,
    staleTime: 1000 * 120, // 2 minutes
    queryFn: () => getUserHasSupported(supportAccounts),
  });

  const isLoading = isLoadingSupportAccounts || query.isLoading;

  return { ...query, isLoading };
};
