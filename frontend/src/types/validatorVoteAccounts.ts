export interface ValidatorVoteAccountData {
  voteAccount: string;
  activeStake: bigint;
  nodePubkey: string;
}

export interface RawValidatorVoteAccountData {
  stake_account: string;
  active_stake: number;
  vote_account: string;
}
