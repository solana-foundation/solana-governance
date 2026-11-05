import {
  BlockchainParams,
  TransactionResult,
  UpdateProposalDescriptionParams,
  updateProposalDescription,
} from "@/chain";

/** Executes the proposal-document update transaction. */
export async function updateProposalDescriptionMutation(
  params: UpdateProposalDescriptionParams,
  blockchainParams: BlockchainParams,
): Promise<TransactionResult> {
  return updateProposalDescription(params, blockchainParams);
}
