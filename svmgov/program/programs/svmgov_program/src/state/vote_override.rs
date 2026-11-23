use anchor_lang::prelude::*;

/// A staker's override of their validator's vote for one stake account.
/// PDA seeds: `["vote_override", proposal, stake_account, validator_vote]`.
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
    /// The accepted vote delegate that signed the current state of this
    /// override, or `None` when `delegator` signed it itself. Always `None` on
    /// overrides recorded before vote delegation existed.
    pub delegate_signer: Option<Pubkey>,
}
