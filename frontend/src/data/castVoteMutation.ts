import {
  BlockchainParams,
  castVote,
  CastVoteParams,
  TransactionResult,
} from "@/chain";

export const castVoteMutation = async (
  params: CastVoteParams,
  blockchainParams: BlockchainParams
): Promise<TransactionResult> => {
  return castVote(params, blockchainParams);
};
