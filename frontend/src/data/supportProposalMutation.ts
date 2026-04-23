import {
  BlockchainParams,
  SupportProposalParams,
  supportProposal,
  TransactionResult,
  ChainVoteAccountData,
} from "@/chain";
import type { GovernanceConfigDto } from "@/lib/getGovernanceConfig";

export const supportProposalMutation = async (
  params: SupportProposalParams,
  blockchainParams: BlockchainParams,
  slot: number | undefined,
  chainVoteAccount: ChainVoteAccountData | undefined,
  governanceConfig: GovernanceConfigDto,
): Promise<TransactionResult> => {
  return supportProposal(
    params,
    blockchainParams,
    slot,
    chainVoteAccount,
    governanceConfig,
  );
};
