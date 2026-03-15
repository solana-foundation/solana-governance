use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Vote {
    pub validator: Pubkey,
    pub proposal: Pubkey,
    pub for_votes_bp: u64,
    pub against_votes_bp: u64,
    pub abstain_votes_bp: u64,
    pub for_votes_lamports: u64,
    pub against_votes_lamports: u64,
    pub abstain_votes_lamports: u64,
    pub stake: u64,
    pub override_lamports: u64,
    pub vote_timestamp: i64,
    pub bump: u8,
}
