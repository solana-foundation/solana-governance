use anchor_lang::prelude::*;

use crate::{error::ErrorCode, BallotBox};

#[derive(Accounts)]
pub struct RemoveVote<'info> {
    pub operator: Signer<'info>,
    #[account(mut)]
    pub ballot_box: Box<Account<'info, BallotBox>>,
}

pub fn handler(ctx: Context<RemoveVote>) -> Result<()> {
    let operator = &ctx.accounts.operator.key();
    let ballot_box = &mut ctx.accounts.ballot_box;

    // Check if operator is in the voter list snapshot
    require!(
        ballot_box.voter_list.contains(operator),
        ErrorCode::OperatorNotWhitelisted
    );

    require!(
        !ballot_box.has_vote_expired(Clock::get()?.unix_timestamp),
        ErrorCode::VotingExpired
    );
    require!(
        !ballot_box.has_consensus_reached(),
        ErrorCode::ConsensusReached
    );

    let operator_vote_idx = ballot_box
        .operator_votes
        .iter()
        .position(|vote| vote.operator == *operator);

    // Get operator's ballot index and remove operator from OperatorVotes.
    let ballot_index: u8;
    if let Some(idx) = operator_vote_idx {
        ballot_index = ballot_box.operator_votes[idx].ballot_index;
        ballot_box.operator_votes.remove(idx);
    } else {
        return err!(ErrorCode::OperatorHasNotVoted);
    }

    // Decrement tally on BallotTally. BallotTally is kept even when tally is 0 to maintain
    // order of indices.
    let ballot_tally = &mut ballot_box.ballot_tallies[ballot_index as usize];
    ballot_tally.tally = ballot_tally.tally.checked_sub(1).unwrap();

    Ok(())
}
