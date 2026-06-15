use anchor_lang::{prelude::*, solana_program::vote};

use crate::{
    error::GovernanceError,
    events::MerkleRootFlushed,
    state::{GlobalConfig, Proposal},
    utils::compute_future_snapshot_slot,
};

#[derive(Accounts)]
pub struct FlushMerkleRoot<'info> {
    #[account(
        mut,
        constraint = signer.key() == global_config.admin @ GovernanceError::UnauthorizedAdmin,
    )]
    pub signer: Signer<'info>, // must equal global_config.admin (NCN-operator multisig)
    #[account(
        mut,
        constraint = !proposal.finalized @ GovernanceError::ProposalFinalized,
    )]
    pub proposal: Account<'info, Proposal>,
    /// CHECK: Owner == vote program is enforced here. The spl_vote_account is bound
    /// to the proposal by the init_ballot_box CPI's PDA seed check, which re-derives
    /// [b"proposal", proposal_seed, spl_vote_account] against the signing proposal
    /// PDA. A mismatched vote account is therefore rejected regardless of who signs
    /// this instruction (the signer is the admin, not necessarily the author).
    #[account(
        constraint = spl_vote_account.owner == &vote::program::ID @ ProgramError::InvalidAccountOwner,
    )]
    pub spl_vote_account: UncheckedAccount<'info>,
    /// CHECK: Ballot box account - may or may not exist, checked with data_is_empty()
    pub ballot_box: UncheckedAccount<'info>,
    /// CHECK: Ballot program account
    #[account(
        constraint = ballot_program.key == &ncn_snapshot::ID @ ProgramError::InvalidAccountOwner,
    )]
    pub ballot_program: UncheckedAccount<'info>,
    /// CHECK: Program config account
    #[account(
        seeds = [b"ProgramConfig"],
        bump,
        seeds::program = ballot_program.key(),
        constraint = program_config.owner == &ncn_snapshot::ID @ ProgramError::InvalidAccountOwner,
    )]
    pub program_config: UncheckedAccount<'info>,
    #[account(
        seeds = [b"global_config"],
        bump = global_config.bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,
    pub system_program: Program<'info, System>,
}

impl<'info> FlushMerkleRoot<'info> {
    pub fn flush_merkle_root(&mut self) -> Result<()> {
        let clock = Clock::get()?;

        // Prevent flushing once voting has started
        require!(
            clock.epoch < self.proposal.start_epoch,
            GovernanceError::CannotModifyAfterStart
        );

        // Clear the consensus_result
        require!(
            self.proposal.snapshot_slot > 0,
            GovernanceError::InvalidSnapshotSlot
        );
        require!(
            self.proposal.consensus_result.is_some(),
            GovernanceError::ConsensusResultNotSet
        );

        // This is an admin-only recovery path: the signer is constrained to
        // global_config.admin (the NCN-operator multisig). Re-anchor the
        // snapshot/voting window forward off the *current* epoch so a proposal whose
        // NCN snapshot failed to reach consensus can be rescheduled far enough ahead
        // for operators to re-snapshot and re-run consensus.
        let target_epoch = clock
            .epoch
            .checked_add(self.global_config.snapshot_epoch_extension)
            .ok_or(GovernanceError::ArithmeticOverflow)?;
        // SECURITY: enforce the future-slot invariant *before* mutating any proposal
        // state. `init_ballot_box` below is skipped whenever `ballot_box` already
        // exists, so this is the only place the `snapshot_slot > clock.slot` guard is
        // guaranteed to run. Without it a proposal could be backdated onto an
        // already-finalized ConsensusResult for a past slot.
        let snapshot_slot = compute_future_snapshot_slot(
            target_epoch,
            self.global_config.snapshot_slot_offset,
            clock.slot,
        )?;

        // SECURITY: bind `ballot_box` to the exact PDA implied by the recomputed
        // snapshot slot so a caller cannot pass an arbitrary non-empty account to
        // skip the init_ballot_box CPI (and its validation) below.
        let (expected_ballot_box, _) = Pubkey::find_program_address(
            &[b"BallotBox", &snapshot_slot.to_le_bytes()],
            &self.ballot_program.key,
        );
        require_keys_eq!(
            self.ballot_box.key(),
            expected_ballot_box,
            GovernanceError::InvalidBallotBox
        );

        // Calculate new consensus_result PDA based on new snapshot_slot
        let (consensus_result_pda, _) = Pubkey::find_program_address(
            &[b"ConsensusResult", &snapshot_slot.to_le_bytes()],
            &self.ballot_program.key,
        );

        // All validation passed; commit the recomputed lineage.
        self.proposal.snapshot_slot = snapshot_slot;
        // start voting 1 epoch after snapshot
        let start_epoch = target_epoch
            .checked_add(1)
            .ok_or(GovernanceError::ArithmeticOverflow)?;
        self.proposal.start_epoch = start_epoch;
        self.proposal.end_epoch = start_epoch
            .checked_add(self.global_config.voting_epochs)
            .ok_or(GovernanceError::ArithmeticOverflow)?;
        self.proposal.consensus_result = Some(consensus_result_pda);

        // Initialize ballot box if it doesn't exist
        if self.ballot_box.data_is_empty() {
            // Create seed components with sufficient lifetime
            let proposal_seed_val = self.proposal.proposal_seed.to_le_bytes();
            let vote_account_key = self.proposal.vote_account_pubkey.key();

            let seeds: &[&[u8]] = &[
                b"proposal".as_ref(),
                &proposal_seed_val,
                vote_account_key.as_ref(),
                &[self.proposal.proposal_bump],
            ];
            let signer = &[&seeds[..]];
            // Initialize the ballot box via CPI
            let cpi_ctx = CpiContext::new_with_signer(
                self.ballot_program.to_account_info(),
                ncn_snapshot::cpi::accounts::InitBallotBox {
                    payer: self.signer.to_account_info(),
                    proposal: self.proposal.to_account_info(),
                    ballot_box: self.ballot_box.to_account_info(),
                    program_config: self.program_config.to_account_info(),
                    system_program: self.system_program.to_account_info(),
                },
                signer,
            );

            ncn_snapshot::cpi::init_ballot_box(
                cpi_ctx,
                snapshot_slot,
                self.proposal.proposal_seed,
                self.spl_vote_account.key(),
            )?;
        }

        emit!(MerkleRootFlushed {
            proposal_id: self.proposal.key(),
            author: self.signer.key(),
            new_snapshot_slot: self.proposal.snapshot_slot,
            flush_timestamp: clock.unix_timestamp,
        });

        Ok(())
    }
}
