use anchor_lang::prelude::*;

use crate::{error::ErrorCode, state::ballot_box::MAX_BALLOT_TALLIES, BallotBox, ProgramConfig};

#[derive(Accounts)]
pub struct ResetBallotBox<'info> {
    pub tie_breaker_admin: Signer<'info>,
    #[account(mut)]
    pub ballot_box: Box<Account<'info, BallotBox>>,
    #[account(has_one = tie_breaker_admin)]
    pub program_config: Box<Account<'info, ProgramConfig>>,
}

pub fn handler(ctx: Context<ResetBallotBox>) -> Result<()> {
    let ballot_box = &mut ctx.accounts.ballot_box;
    let clock = Clock::get()?;

    // Consensus must not have been reached yet and voting must not have expired.
    require!(
        !ballot_box.has_consensus_reached(),
        ErrorCode::ConsensusReached
    );
    require!(
        !ballot_box.has_vote_expired(clock.unix_timestamp),
        ErrorCode::VotingExpired
    );

    // Check that BallotBox is fully in use before allowing reset.
    require!(
        ballot_box.ballot_tallies.len() == MAX_BALLOT_TALLIES,
        ErrorCode::BallotTalliesNotMaxLength
    );

    ballot_box.operator_votes.clear();
    ballot_box.ballot_tallies.clear();

    Ok(())
}
