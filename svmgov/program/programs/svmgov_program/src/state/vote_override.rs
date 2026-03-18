use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct VoteOverride {
    pub delegator: Pubkey,
    pub stake_account: Pubkey,
    pub validator: Pubkey,
    pub proposal: Pubkey,
    pub vote_account_validator: Pubkey,
    pub for_votes_bp: u64,
    pub against_votes_bp: u64,
    pub abstain_votes_bp: u64,
    pub for_votes_lamports: u64,
    pub against_votes_lamports: u64,
    pub abstain_votes_lamports: u64,
    pub stake_amount: u64,
    pub vote_override_timestamp: i64,
    pub bump: u8,
}
