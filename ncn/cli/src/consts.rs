use anchor_lang::prelude::Pubkey as AnchorPubkey;
use solana_program::pubkey::Pubkey;

/// Marinade's withdraw authority for stake accounts
pub const MARINADE_WITHDRAW_AUTHORITY: AnchorPubkey =
    AnchorPubkey::from_str_const("9eG63CdHjsfhHmobHgLtESGC8GabbmRcaSpHAZrtmhco");

/// Marinade's operations voting wallet
pub const MARINADE_OPS_VOTING_WALLET: AnchorPubkey =
    AnchorPubkey::from_str_const("opLSF7LdfyWNBby5o6FT8UFsr2A4UGKteECgtLSYrSm");

/// spl-stake-pool program id
pub const STAKE_POOL_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy");