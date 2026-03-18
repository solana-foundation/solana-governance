use anchor_lang::prelude::Pubkey;

// Admin wallet that can initialize and update GlobalConfig
// TODO: Replace with actual admin pubkey before deployment
pub const ADMIN_PUBKEY: Pubkey = Pubkey::new_from_array([0u8; 32]);

// Structural constants that never change
pub const ANCHOR_DISCRIMINATOR: usize = 8;
pub const BASIS_POINTS_MAX: u64 = 10_000;

// Account sizing constants - upper bounds for Proposal account allocation
// These are NOT governance parameters; they define the max possible account size
pub const MAX_TITLE_ACCOUNT_SIZE: usize = 200;
pub const MAX_DESC_ACCOUNT_SIZE: usize = 500;
