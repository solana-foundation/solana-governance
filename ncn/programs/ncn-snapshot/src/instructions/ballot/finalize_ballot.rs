use anchor_lang::prelude::*;

use crate::{error::ErrorCode, BallotBox, ConsensusResult};

#[derive(Accounts)]
pub struct FinalizeBallot<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub ballot_box: Box<Account<'info, BallotBox>>,
    #[account(
        init,
        seeds = [
            b"ConsensusResult".as_ref(),
            &ballot_box.snapshot_slot.to_le_bytes()
        ],
        bump,
        payer = payer,
        space = 8 + ConsensusResult::INIT_SPACE
    )]
    pub consensus_result: Box<Account<'info, ConsensusResult>>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<FinalizeBallot>) -> Result<()> {
    let ballot_box = &ctx.accounts.ballot_box;
    require!(
        ballot_box.has_consensus_reached(),
        ErrorCode::ConsensusNotReached
    );

    let consensus_result = &mut ctx.accounts.consensus_result;
    consensus_result.snapshot_slot = ballot_box.snapshot_slot;
    consensus_result.ballot = ballot_box.winning_ballot.clone();
    consensus_result.tie_breaker_consensus = ballot_box.tie_breaker_consensus;

    Ok(())
}
