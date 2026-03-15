use anchor_lang::prelude::*;

use crate::{constants::*, state::ProposalIndex};

#[derive(Accounts)]
pub struct InitializedIndex<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        space = ANCHOR_DISCRIMINATOR + ProposalIndex::INIT_SPACE,
        seeds = [b"index"],
        bump
    )]
    pub proposal_index: Account<'info, ProposalIndex>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializedIndex<'info> {
    pub fn init_index(&mut self, bumps: &InitializedIndexBumps) -> Result<()> {
        self.proposal_index.set_inner(ProposalIndex {
            current_index: 0,
            bump: bumps.proposal_index,
        });
        Ok(())
    }
}
