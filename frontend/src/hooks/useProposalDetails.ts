import { useProposals } from "./useProposals";
import { ProposalRecord } from "@/types";
import { useMemo } from "react";

const findProposalByPubKey = (
  proposals: ProposalRecord[],
  proposalPublicKey: string
) =>
  proposals.find(
    (proposal) =>
      proposal.publicKey.toLowerCase() ===
      proposalPublicKey.toLowerCase()
  );

export const useProposalDetails = (proposalPublicKey: string) => {
  const { data: proposalsData, ...restQueryConfig } = useProposals();

  const proposalDetails = useMemo(
    () => findProposalByPubKey(proposalsData || [], proposalPublicKey),
    [proposalsData, proposalPublicKey]
  );

  return { data: proposalDetails, ...restQueryConfig };
};
