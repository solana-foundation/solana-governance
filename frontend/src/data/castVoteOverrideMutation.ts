import {
  BlockchainParams,
  castVoteOverride,
  CastVoteOverrideParams,
  TransactionResult,
} from "@/chain";

export const castVoteOverrideMutation = async (
  params: CastVoteOverrideParams,
  blockchainParams: BlockchainParams
): Promise<TransactionResult> => {
  return castVoteOverride(params, blockchainParams);
};
