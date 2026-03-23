import {
  createProgramWitDummyWallet,
  deriveGlobalConfigPda,
  type GlobalConfigAccount,
} from "@/chain";

export interface GovernanceConfigDto {
  admin: string;
  maxTitleLength: number;
  maxDescriptionLength: number;
  maxSupportEpochs: number;
  minProposalStakeLamports: number;
  clusterSupportPctMinBps: number;
  discussionEpochs: number;
  votingEpochs: number;
  snapshotEpochExtension: number;
  bump: number;
}

/** Maps chain account to the public DTO. */
export function toGovernanceConfigDto(
  account: GlobalConfigAccount,
): GovernanceConfigDto {
  const n = (v: unknown) =>
    typeof v === "number"
      ? v
      : ((v as { toNumber: () => number })?.toNumber?.() ?? 0);
  return {
    admin: account.admin.toBase58(),
    // TODO: revisit this, once global config account is initialized
    //   we cant simply default to 0, since we will be using this in FE validations
    maxTitleLength: account.maxTitleLength ?? 0,
    // TODO: revisit this, once global config account is initialized
    //   we cant simply default to 0, since we will be using this in FE validations
    maxDescriptionLength: account.maxDescriptionLength ?? 0,
    maxSupportEpochs: n(account.maxSupportEpochs),
    minProposalStakeLamports: n(account.minProposalStakeLamports),
    clusterSupportPctMinBps: n(account.clusterSupportPctMinBps),
    discussionEpochs: n(account.discussionEpochs),
    votingEpochs: n(account.votingEpochs),
    snapshotEpochExtension: n(account.snapshotEpochExtension),
    bump: account.bump,
  };
}

/**
 * Fetches governance config from chain (reads the on-chain globalConfig account).
 */
export async function fetchGovernanceConfigFromChain(
  rpcUrl: string,
): Promise<GovernanceConfigDto> {
  const program = createProgramWitDummyWallet(rpcUrl);
  const pda = deriveGlobalConfigPda(program.programId);
  console.log("globalConfig pda addr:", pda.toBase58());
  const account = await program.account.globalConfig.fetch(pda);
  return toGovernanceConfigDto(account);
}
