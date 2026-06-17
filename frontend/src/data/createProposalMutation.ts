import {
  BlockchainParams,
  createProposal,
  CreateProposalParams,
  TransactionResult,
} from "@/chain";

export const createProposalMutation = async (
  params: CreateProposalParams,
  blockchainParams: BlockchainParams,
): Promise<TransactionResult> => {
  return createProposal(params, blockchainParams);
};
