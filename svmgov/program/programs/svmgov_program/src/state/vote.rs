use anchor_lang::prelude::*;

/// A validator's vote on a proposal. PDA seeds: `["vote", proposal, spl_vote_account]`.
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
    /// The accepted vote delegate that signed the current state of this vote,
    /// or `None` when `validator` signed it itself. Always `None` on votes
    /// recorded before vote delegation existed.
    pub delegate_signer: Option<Pubkey>,
}
