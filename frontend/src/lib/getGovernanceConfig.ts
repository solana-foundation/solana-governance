import { createSolanaRpc } from "@solana/kit";
import {
  fetchGlobalConfig,
  findGlobalConfigPda,
  type GlobalConfig,
} from "@solana/svmgov";

export interface GovernanceConfigDto {
  admin: string;
  maxTitleLength: number;
  maxDescriptionLength: number;
  maxSupportEpochs: bigint;
  minProposalStakeLamports: bigint;
  clusterSupportPctMinBps: bigint;
  discussionEpochs: bigint;
  votingEpochs: bigint;
  snapshotEpochExtension: bigint;
  snapshotSlotOffset: bigint;
  maxSupporters: number;
  bump: number;
}

/** Maps chain account to the public DTO. */
export function toGovernanceConfigDto(
  account: GlobalConfig,
): GovernanceConfigDto {
  return {
    admin: account.admin,
    // TODO: revisit this, once global config account is initialized
    //   we cant simply default to 0, since we will be using this in FE validations
    maxTitleLength: account.maxTitleLength ?? 0,
    // TODO: revisit this, once global config account is initialized
    //   we cant simply default to 0, since we will be using this in FE validations
    maxDescriptionLength: account.maxDescriptionLength ?? 0,
    maxSupportEpochs: account.maxSupportEpochs,
    minProposalStakeLamports: account.minProposalStakeLamports,
    clusterSupportPctMinBps: account.clusterSupportPctMinBps,
    discussionEpochs: account.discussionEpochs,
    votingEpochs: account.votingEpochs,
    snapshotEpochExtension: account.snapshotEpochExtension,
    snapshotSlotOffset: account.snapshotSlotOffset,
    maxSupporters: account.maxSupporters,
    bump: account.bump,
  };
}

/**
 * Fetches governance config from chain (reads the on-chain globalConfig account).
 */
export async function fetchGovernanceConfigFromChain(
  rpcUrl: string,
): Promise<GovernanceConfigDto> {
  const rpc = createSolanaRpc(rpcUrl);
  const [address] = await findGlobalConfigPda();
  const account = await fetchGlobalConfig(rpc, address);
  return toGovernanceConfigDto(account.data);
}
