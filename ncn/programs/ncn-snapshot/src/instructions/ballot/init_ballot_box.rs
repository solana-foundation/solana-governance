use anchor_lang::prelude::*;

use crate::{error::ErrorCode, BallotBox, ProgramConfig, MIN_VOTE_EXPIRY_SLOTS};

#[derive(Accounts)]
#[instruction(snapshot_slot: u64, proposal_seed: u64, spl_vote_account: Pubkey)]
pub struct InitBallotBox<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        seeds = [
            b"proposal",
            &proposal_seed.to_le_bytes(),
            spl_vote_account.as_ref()
        ],
        bump,
        seeds::program = program_config.svmgov_program_pubkey
    )]
    /// Verifies the signer is a Proposal PDA from the svmgov program recorded in
    /// `ProgramConfig.svmgov_program_pubkey` — only that program (via CPI with
    /// the proposal's seeds) can open a ballot box.
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
    vote_expiry_slot: u64,
) -> Result<()> {
    let clock = Clock::get()?;

    // Check that snapshot slot is greater than current slot to
    // allow sufficient lead time for snapshot.
    require!(snapshot_slot > clock.slot, ErrorCode::InvalidSnapshotSlot);

    let program_config = &ctx.accounts.program_config;
    let ballot_box = &mut ctx.accounts.ballot_box;

    validate_vote_expiry_window(snapshot_slot, vote_expiry_slot)?;

    ballot_box.bump = ctx.bumps.ballot_box;
    ballot_box.epoch = clock.epoch;
    ballot_box.slot_created = clock.slot;
    ballot_box.snapshot_slot = snapshot_slot;
    ballot_box.min_consensus_threshold_bps = program_config.min_consensus_threshold_bps;
    ballot_box.vote_expiry_slot = vote_expiry_slot;
    ballot_box.voter_list = program_config.whitelisted_operators.clone();
    ballot_box.tie_breaker_consensus = false;

    Ok(())
}

fn validate_vote_expiry_window(snapshot_slot: u64, vote_expiry_slot: u64) -> Result<()> {
    require!(
        vote_expiry_slot.saturating_sub(snapshot_slot) >= MIN_VOTE_EXPIRY_SLOTS,
        ErrorCode::VoteExpiryTooSoon
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::error::{Error, ERROR_CODE_OFFSET};

    fn assert_expiry_too_soon(result: Result<()>) {
        match result.expect_err("expiry window must be rejected") {
            Error::AnchorError(error) => assert_eq!(
                error.error_code_number,
                ERROR_CODE_OFFSET + ErrorCode::VoteExpiryTooSoon as u32
            ),
            Error::ProgramError(error) => panic!("unexpected program error: {error:?}"),
        }
    }

    #[test]
    fn minimum_vote_expiry_window_is_inclusive() {
        let snapshot_slot = 1_000;

        assert!(
            validate_vote_expiry_window(snapshot_slot, snapshot_slot + MIN_VOTE_EXPIRY_SLOTS)
                .is_ok()
        );
        assert_expiry_too_soon(validate_vote_expiry_window(
            snapshot_slot,
            snapshot_slot + MIN_VOTE_EXPIRY_SLOTS - 1,
        ));
        assert_expiry_too_soon(validate_vote_expiry_window(
            snapshot_slot,
            snapshot_slot - 1,
        ));
    }
}
