use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct VoteOverrideCache {
    pub validator: Pubkey,
    pub proposal: Pubkey,
    pub vote_account_validator: Pubkey,
    pub for_votes_bp: u64,
    pub against_votes_bp: u64,
    pub abstain_votes_bp: u64,
    pub for_votes_lamports: u64,
    pub against_votes_lamports: u64,
    pub abstain_votes_lamports: u64,
    pub total_stake: u64,
    pub bump: u8,
}
