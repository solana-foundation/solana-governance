use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct GlobalConfig {
    /// The admin pubkey who can update this config
    pub admin: Pubkey,
    /// Maximum length for proposal titles
    pub max_title_length: u16,
    /// Maximum length for proposal descriptions
    pub max_description_length: u16,
    /// Maximum epochs allowed for support phase (0 means same epoch as creation)
    pub max_support_epochs: u64,
    /// Minimum stake in lamports required to create a proposal
    pub min_proposal_stake_lamports: u64,
    /// Minimum cluster support percentage in BASIS POINTS (1 bp = 0.01%)
    /// e.g., 1000 = 10%, 50 = 0.5%
    pub cluster_support_pct_min_bps: u64,
    /// Number of full epochs reserved for discussion
    pub discussion_epochs: u64,
    /// Number of epochs for voting period
    pub voting_epochs: u64,
    /// Epochs of extension for snapshot
    pub snapshot_epoch_extension: u64,
    /// Slot offset from epoch start for snapshot computation (can be negative)
    pub snapshot_slot_offset: i64,
    /// PDA bump seed
    pub bump: u8,
}
