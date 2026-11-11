import { useEndpoint } from "@/contexts/EndpointContext";
import { getValidatorProposalVoteAccount } from "@/data";
import { GET_VALIDATOR_HAS_VOTED } from "@/helpers";
import { useWallet } from "@solana/wallet-adapter-react";
import { useQuery } from "@tanstack/react-query";
import { useChainVoteAccount } from "./useChainVoteAccount";
import { useWalletRole } from "./useWalletRole";
import { WalletRole } from "@/types";

/**
 * Whether the validator the connected wallet acts for (as its identity or its
 * accepted vote delegate) has voted on the proposal.
 */
export const useHasValidatorVoted = (
  proposalPublicKey: string | undefined,
  enabledProp = true
) => {
  const { endpointUrl: endpoint } = useEndpoint();
  const { publicKey, connected } = useWallet();

  const { walletRole } = useWalletRole(publicKey?.toBase58());
  const { data: chainVoteAccount, isLoading: isResolvingIdentity } =
    useChainVoteAccount(publicKey?.toBase58());
  const validatorIdentity = chainVoteAccount?.nodePubkey ?? publicKey?.toBase58();

  const isValidator = walletRole === WalletRole.VALIDATOR;
  const isBoth = walletRole === WalletRole.BOTH;

  const enabled =
    connected &&
    !!publicKey &&
    enabledProp &&
    !!proposalPublicKey &&
    (isValidator || isBoth) &&
    !isResolvingIdentity;

  const query = useQuery({
    queryKey: [
      GET_VALIDATOR_HAS_VOTED,
      endpoint,
      proposalPublicKey,
      validatorIdentity,
    ],
    enabled,
    staleTime: 1000 * 120, // 2 minutes
    queryFn: async () => {
      const voteAccount = await getValidatorProposalVoteAccount(
        endpoint,
        proposalPublicKey,
        validatorIdentity
      );
      return !!voteAccount;
    },
  });

  return query;
};
