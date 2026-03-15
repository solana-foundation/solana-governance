use anchor_lang::prelude::*;

use crate::MetaMerkleProof;

#[derive(Accounts)]
pub struct CloseMetaMerkleProof<'info> {
    /// Account to receive the reclaimed rent from StakingRecord
    /// CHECK: must match payer in MetaMerkleProof
    #[account(mut)]
    pub payer: UncheckedAccount<'info>,
    #[account(
        mut,
        close = payer,
        has_one = payer
    )]
    pub meta_merkle_proof: Box<Account<'info, MetaMerkleProof>>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<CloseMetaMerkleProof>) -> Result<()> {
    let meta_merkle_proof = &ctx.accounts.meta_merkle_proof;

    // Check that close timestamp has elapsed if the payer hasn't signed the transaction.
    if !ctx.accounts.payer.is_signer {
        require_gte!(
            Clock::get()?.unix_timestamp,
            meta_merkle_proof.close_timestamp
        )
    }

    Ok(())
}
