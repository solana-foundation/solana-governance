use anchor_lang::prelude::*;

use crate::{error::ErrorCode, BallotBox, ProgramConfig};

#[cfg(not(feature = "skip-pda-check"))]
const GOV_PROGRAM_ID: Pubkey = pubkey!("EKwRPoyRactBV2z2XhUSVU1YbZuyTVq4kU5U5dM2JyZY");

#[derive(Accounts)]
#[instruction(snapshot_slot: u64, proposal_seed: u64, spl_vote_account: Pubkey)]
pub struct InitBallotBox<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[cfg_attr(not(feature = "skip-pda-check"), account(
        seeds = [
            b"proposal",
            &proposal_seed.to_le_bytes(),
            spl_vote_account.as_ref()
        ],
        bump,
        seeds::program = GOV_PROGRAM_ID
    ))]
    /// Verifies that signer is a Proposal PDA from the governance program.
    /// When `skip-pda-check` feature is enabled, this check is disabled to allow local testing without CPI.
    pub proposal: Signer<'info>,
    #[account(
        init,
        seeds = [
            b"BallotBox".as_ref(),
            &snapshot_slot.to_le_bytes()
        ],
        bump,
        payer = payer,
        space = 8 + BallotBox::INIT_SPACE
    )]
    pub ballot_box: Box<Account<'info, BallotBox>>,
    pub program_config: Box<Account<'info, ProgramConfig>>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<InitBallotBox>,
    snapshot_slot: u64,
    _proposal_seed: u64,
    _spl_vote_account: Pubkey,
) -> Result<()> {
    let clock = Clock::get()?;

    // Check that snapshot slot is greater than current slot to
    // allow sufficient lead time for snapshot.
    require!(snapshot_slot > clock.slot, ErrorCode::InvalidSnapshotSlot);

    let program_config = &ctx.accounts.program_config;
    let ballot_box = &mut ctx.accounts.ballot_box;

    ballot_box.bump = ctx.bumps.ballot_box;
    ballot_box.epoch = clock.epoch;
    ballot_box.slot_created = clock.slot;
    ballot_box.snapshot_slot = snapshot_slot;
    ballot_box.min_consensus_threshold_bps = program_config.min_consensus_threshold_bps;
    ballot_box.vote_expiry_timestamp = clock
        .unix_timestamp
        .checked_add(program_config.vote_duration)
        .unwrap();
    ballot_box.voter_list = program_config.whitelisted_operators.clone();
    ballot_box.tie_breaker_consensus = false;

    Ok(())
}
