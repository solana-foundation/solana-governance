import { ChainVoteAccountData } from "@/chain";
import { ViewType } from "@/types";

export enum WalletRole {
  VALIDATOR = "validator",
  STAKER = "staker",
  BOTH = "both",
  NONE = "none",
}

export function determineWalletRole(
  stakeAccounts: unknown[] | undefined,
  _voteAccounts: unknown[] | undefined,
  chainVoteAccount: ChainVoteAccountData | null | undefined
): WalletRole {
  const hasStake = !!stakeAccounts?.length;
  const hasValidatorIdentity = hasOnChainValidatorIdentity(chainVoteAccount);

  if (hasStake && hasValidatorIdentity) return WalletRole.BOTH;
  else if (hasValidatorIdentity) return WalletRole.VALIDATOR;
  else if (hasStake) return WalletRole.STAKER;

  // default to staker even if there are no stake accounts
  return WalletRole.STAKER;
}

export function hasOnChainValidatorIdentity(
  chainVoteAccount: ChainVoteAccountData | null | undefined
): boolean {
  return chainVoteAccount != null;
}

export type VoteModalNames = {
  castModalName: "cast-vote" | "override-vote";
  modifyModalName: "modify-vote" | "modify-override-vote";
};

export function getVoteModalNames(
  chainVoteAccount: ChainVoteAccountData | null | undefined
): VoteModalNames {
  if (hasOnChainValidatorIdentity(chainVoteAccount)) {
    return {
      castModalName: "cast-vote",
      modifyModalName: "modify-vote",
    };
  }

  return {
    castModalName: "override-vote",
    modifyModalName: "modify-override-vote",
  };
}

export function getDefaultView(role: WalletRole): ViewType | undefined {
  switch (role) {
    case WalletRole.VALIDATOR:
      return "validator";
    case WalletRole.STAKER:
      return "staker";
    case WalletRole.BOTH:
      return "validator";
    default:
      return undefined;
  }
}
