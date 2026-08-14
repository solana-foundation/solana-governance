import {
  BlockchainParams,
  modifyVote,
  ModifyVoteParams,
  TransactionResult,
} from "@/chain";

export const modifyVoteMutation = async (
  params: ModifyVoteParams,
  blockchainParams: BlockchainParams
): Promise<TransactionResult> => {
  return modifyVote(params, blockchainParams);
};
