import { useEndpoint } from "@/contexts/EndpointContext";
import { getValidatorProposalVoteAccount } from "@/data";
import { GET_VALIDATOR_HAS_VOTED } from "@/helpers";
import { useWalletSession } from "@/contexts/WalletSessionContext";
import { useQuery } from "@tanstack/react-query";
import { useWalletRole } from "./useWalletRole";
import { WalletRole } from "@/types";

export const useHasValidatorVoted = (
  proposalPublicKey: string | undefined,
  enabledProp = true
) => {
  const { endpointUrl: endpoint } = useEndpoint();
  const { publicKey, connected } = useWalletSession();

  const { walletRole } = useWalletRole(publicKey);

  const isValidator = walletRole === WalletRole.VALIDATOR;
  const isBoth = walletRole === WalletRole.BOTH;

  const enabled =
    connected &&
    !!publicKey &&
    enabledProp &&
    !!proposalPublicKey &&
    (isValidator || isBoth);

  const query = useQuery({
    queryKey: [
      GET_VALIDATOR_HAS_VOTED,
      endpoint,
      proposalPublicKey,
      publicKey,
    ],
    enabled,
    staleTime: 1000 * 120, // 2 minutes
    queryFn: async () => {
      const voteAccount = await getValidatorProposalVoteAccount(
        endpoint,
        proposalPublicKey,
        publicKey
      );
      return !!voteAccount;
    },
  });

  return query;
};
