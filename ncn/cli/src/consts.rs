use anchor_lang::prelude::Pubkey as AnchorPubkey;

/// Marinade's withdraw authority for stake accounts
pub const MARINADE_WITHDRAW_AUTHORITY: AnchorPubkey =
    AnchorPubkey::from_str_const("9eG63CdHjsfhHmobHgLtESGC8GabbmRcaSpHAZrtmhco");

/// Marinade's operations voting wallet
pub const MARINADE_OPS_VOTING_WALLET: AnchorPubkey =
    AnchorPubkey::from_str_const("opLSF7LdfyWNBby5o6FT8UFsr2A4UGKteECgtLSYrSm");
