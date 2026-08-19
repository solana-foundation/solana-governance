import { useEndpoint } from "@/contexts/EndpointContext";
import { getStakeWizValidators } from "@/data";
import { fetchRawVoteAccounts } from "@/lib/rpcVoteAccounts";
import { Validator, Validators } from "@/types";
import { useQuery } from "@tanstack/react-query";

export const useGetValidators = () => {
  const { endpointUrl: endpoint } = useEndpoint();

  return useQuery({
    queryKey: ["validators", endpoint],
    staleTime: 1000 * 120, // 2 minutes
    queryFn: () => getValidators(endpoint),
  });
};

const getValidators = async (endpoint: string): Promise<Validators> => {
  // The RPC is the source of truth for both the validator set and its stake, so
  // a failure here has to surface as a query error — react-query then retries
  // and consumers can render an error instead of a plausible-looking zero.
  // StakeWiz only supplies display metadata (name, image, description), so it is
  // allowed to fail: validators fall back to placeholder names with their stake
  // still correct.
  const [stakeWizValidators, voteAccounts] = await Promise.allSettled([
    getStakeWizValidators(),
    fetchRawVoteAccounts(endpoint),
  ]);

  if (voteAccounts.status === "rejected") {
    throw new Error(
      `Failed to fetch vote accounts from ${endpoint}: ${voteAccounts.reason}`,
    );
  }

  if (stakeWizValidators.status === "rejected") {
    console.warn(
      "StakeWiz metadata unavailable; falling back to placeholder validator names",
      stakeWizValidators.reason,
    );
  }

  const metadataByVoteAccount = new Map<string, Validator>();
  if (stakeWizValidators.status === "fulfilled") {
    for (const validator of stakeWizValidators.value.data) {
      metadataByVoteAccount.set(validator.vote_identity, validator);
    }
  }

  const allVotes = [
    ...voteAccounts.value.current,
    ...voteAccounts.value.delinquent,
  ];

  // For each RPC vote account: votePubkey = vote account address, nodePubkey =
  // validator identity. StakeWiz vote_identity is the vote account address —
  // match to vote.votePubkey, not nodePubkey.
  let unknownCount = 0;
  return allVotes.map((vote) => {
    const metadata = metadataByVoteAccount.get(vote.votePubkey);

    if (metadata) {
      return {
        ...metadata,
        // Always take stake from the RPC. StakeWiz reports a separately-sampled
        // SOL figure, so sourcing it here left one array holding two different
        // measurements depending on whether a validator happened to match.
        activated_stake: vote.activatedStake,
        identity: metadata.identity ?? vote.nodePubkey,
      };
    }

    unknownCount++;
    const unknownValidator: Validator = {
      name: `Unknown validator #${unknownCount}`,
      activated_stake: vote.activatedStake,
      version: "-",
      description: "",
      asn: "-",
      vote_identity: vote.votePubkey,
      identity: vote.nodePubkey,
      commission: vote.commission,
      epoch_credits: vote.epochCredits?.[0]?.[0] || 0,
      last_vote: vote.lastVote,
    };
    return unknownValidator;
  });
};
