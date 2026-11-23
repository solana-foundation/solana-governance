export type StakeAccountState =
  | "initialized"
  | "delegated"
  | "inactive"
  | "deactivating"
  | "cooldown";
export interface StakeAccountData {
  id: string;
  stakeAccount: string;
  activeStake: bigint;
  voteAccount: string | undefined;
  state?: StakeAccountState;
}
