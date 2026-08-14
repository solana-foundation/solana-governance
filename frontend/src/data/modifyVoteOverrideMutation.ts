import {
  BlockchainParams,
  CastVoteOverrideParams,
  modifyVoteOverride,
  TransactionResult,
} from "@/chain";

export const modifyVoteOverrideMutation = async (
  params: CastVoteOverrideParams,
  blockchainParams: BlockchainParams
): Promise<TransactionResult> => {
  return modifyVoteOverride(params, blockchainParams);
};
