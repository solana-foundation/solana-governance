use anchor_lang::{
    prelude::*,
    solana_program::{
        epoch_stake::{get_epoch_stake_for_vote_account, get_epoch_total_stake},
        vote::{program as vote_program, state::VoteState},
    },
};
use solana_vote_interface::state::VoteStateVersions;

use crate::{
    constants::ANCHOR_DISCRIMINATOR,
    error::GovernanceError,
    events::ProposalSupported,
    state::{GlobalConfig, Proposal, Support},
    utils::{
        check_support_window, compute_future_snapshot_slot, min_stake_threshold,
        proposal_target_epoch, tally_supporter_stakes,
    },
};

#[derive(Accounts)]
pub struct SupportProposal<'info> {
    #[account(mut)]
    pub signer: Signer<'info>, // Proposal supporter (validator)
    #[account(
        mut,
        // Grow the proposal by exactly one supporter entry (32 bytes). The
        // supporter pays the rent delta for their own slice. INIT_SPACE
        // already includes the 4-byte `num_supporters` count, so total size
        // is disc + fixed fields + 32 per supporter.
        realloc = ANCHOR_DISCRIMINATOR
            + Proposal::INIT_SPACE
            + (proposal.num_supporters as usize + 1) * core::mem::size_of::<Pubkey>(),
        realloc::payer = signer,
        realloc::zero = false,
    )]
    pub proposal: Account<'info, Proposal>,
    #[account(
        init,
        payer = signer,
        space = ANCHOR_DISCRIMINATOR + Support::INIT_SPACE,
        seeds = [b"support", proposal.key().as_ref(), spl_vote_account.key().as_ref()],
        bump
    )]
    pub support: Account<'info, Support>, // New support account
    /// CHECK: Owner == vote program and account size == VoteState::size_of() are
    /// enforced here; the handler then deserializes VoteStateVersions and requires
    /// node_pubkey == signer, so a supporter can only pledge stake from a vote
    /// account they operate.
    #[account(
        constraint = spl_vote_account.owner == &vote_program::ID @ ProgramError::InvalidAccountOwner,
        constraint = spl_vote_account.data_len() == VoteState::size_of() @ GovernanceError::InvalidVoteAccountSize
    )]
    pub spl_vote_account: UncheckedAccount<'info>,

    /// CHECK: Ballot box account - may or may not exist, checked with data_is_empty()
    #[account(mut)]
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

impl<'info> SupportProposal<'info> {
    pub fn support_proposal(&mut self, bumps: &SupportProposalBumps) -> Result<()> {
        let clock = Clock::get()?;
        let current_epoch = clock.epoch;
        // Ensure proposal is eligible for support
        require!(
            self.proposal.voting == false && self.proposal.finalized == false,
            GovernanceError::ProposalClosed
        );

        // Support is accepted anywhere inside the window
        // [creation_epoch, creation_epoch + max_support_epochs], inclusive.
        check_support_window(
            clock.epoch,
            self.proposal.creation_epoch,
            self.global_config.max_support_epochs,
        )?;

        // Cap the supporter count. Every support/retally re-measures the whole
        // list in a single transaction, so an unbounded list would eventually
        // exceed Solana's heap/compute limits and brick the proposal. Checked
        // before the new entry is recorded so the list never exceeds the cap.
        require!(
            (self.proposal.num_supporters as u64) < self.global_config.max_supporters as u64,
            GovernanceError::SupporterLimitReached
        );

        // Ensure signer is the node identity of the vote account, so a supporter
        // can only pledge stake from a vote account they operate.
        let vote_account_data = self.spl_vote_account.data.borrow();
        let versioned = VoteStateVersions::deserialize(&vote_account_data)
            .map_err(|_| GovernanceError::FailedDeserializeNodePubkey)?;
        let node_pubkey_bytes: [u8; 32] = match &versioned {
            VoteStateVersions::V3(v) => v.node_pubkey.to_bytes(),
            VoteStateVersions::V4(v) => v.node_pubkey.to_bytes(),
            VoteStateVersions::V1_14_11(v) => v.node_pubkey.to_bytes(),
            VoteStateVersions::Uninitialized => {
                return Err(GovernanceError::InvalidVoteAccountVersion.into())
            }
        };
        require!(
            node_pubkey_bytes == self.signer.key().to_bytes(),
            GovernanceError::VoteNodePubkeyMismatch
        );
        drop(vote_account_data);

        let proposal_info = self.proposal.to_account_info();

        if current_epoch > self.proposal.last_support_epoch {
            // update last support epoch to current epoch and epoch stake
            self.proposal.last_support_epoch = current_epoch;
            let cluster_min_stake = min_stake_threshold(
                get_epoch_total_stake(),
                self.global_config.cluster_support_pct_min_bps,
            )?;
            self.proposal.support_threshold = cluster_min_stake;

            // retally only if current epoch != last support epoch
            // active stake is not supposed to change during the epoch
            let prior_stake = {
                let proposal_data = proposal_info.try_borrow_data()?;
                let supporters = self.proposal.supporters(&proposal_data)?;
                tally_supporter_stakes(supporters, |pk| get_epoch_stake_for_vote_account(pk))?
            };
            self.proposal.cluster_support_lamports = prior_stake;
        }


        let supporter_stake = get_epoch_stake_for_vote_account(self.spl_vote_account.key);
        // A supporter must clear the same stake floor as a proposal author.
        // This blocks a zero-/dust-stake spam attack that would otherwise let
        // an attacker cheaply fill the supporters list to the cap (or drive up
        // re-tally cost) without contributing meaningful stake.
        require!(
            supporter_stake >= self.global_config.min_proposal_stake_lamports,
            GovernanceError::NotEnoughStake
        );
        let new_support_stake = self
            .proposal
            .cluster_support_lamports
            .checked_add(supporter_stake)
            .ok_or(GovernanceError::ArithmeticOverflow)?;

        let proposal_account = &mut self.proposal;
        proposal_account.cluster_support_lamports = new_support_stake;
        // Record the supporter's vote account so future support/retally calls
        // can re-measure them. The realloc constraint above already grew the
        // account by exactly this entry; the pubkey is written straight into
        // the account data (only `num_supporters` goes through Anchor).
        {
            let mut proposal_data = proposal_info.try_borrow_mut_data()?;
            proposal_account.append_supporter(&mut proposal_data, self.spl_vote_account.key())?;
        }

        // Initialize the support account
        self.support.set_inner(Support {
            proposal: proposal_account.key(),
            validator: self.signer.key(),
            bump: bumps.support,
        });

        // Threshold is measured against the current epoch's total stake — the
        // same epoch every supporter above was just re-measured at, so
        // numerator and denominator can never mix epochs.

        let mut snapshot_slot = 0;
        if new_support_stake >= self.proposal.support_threshold {
            snapshot_slot = activate_voting(
                &mut self.proposal,
                &self.global_config,
                &self.signer,
                &self.ballot_box,
                &self.ballot_program,
                &self.program_config,
                &self.system_program,
                &clock,
            )?;
        }

        emit!(ProposalSupported {
            proposal_id: self.proposal.key(),
            supporter: self.signer.key(),
            cluster_support_lamports: new_support_stake,
            voting_activated: self.proposal.voting,
            snapshot_slot: snapshot_slot,
        });

        Ok(())
    }
}

/// Shared activation path for `support_proposal` and `retally_support`: fixes
/// the voting schedule, binds the ballot box, and flips `voting` on.
///
/// The schedule anchors on `clock.epoch` — the epoch in which the
/// threshold-crossing call lands, which may be any epoch inside the support
/// window. A proposal that gathers support quickly therefore votes earlier
/// than one that crosses the threshold on the window's last epoch.
/// (`flush_merkle_root` is unaffected: it is an admin-only recovery path that
/// re-anchors off the current epoch and never reconstructs this schedule.)
///
/// Returns the computed snapshot slot.
#[allow(clippy::too_many_arguments)]
pub(crate) fn activate_voting<'info>(
    proposal: &mut Account<'info, Proposal>,
    global_config: &GlobalConfig,
    payer: &Signer<'info>,
    ballot_box: &UncheckedAccount<'info>,
    ballot_program: &UncheckedAccount<'info>,
    program_config: &UncheckedAccount<'info>,
    system_program: &Program<'info, System>,
    clock: &Clock,
) -> Result<u64> {
    let target_epoch = proposal_target_epoch(
        clock.epoch,
        global_config.discussion_epochs,
        global_config.snapshot_epoch_extension,
    )?;
    // SECURITY: enforce the future-slot invariant before mutating proposal
    // state. The init_ballot_box CPI below is skipped whenever `ballot_box`
    // already exists, so this re-check prevents a proposal from being bound
    // onto an already-finalized ConsensusResult for a past slot.
    let snapshot_slot =
        compute_future_snapshot_slot(target_epoch, global_config.snapshot_slot_offset, clock.slot)?;

    // SECURITY: bind `ballot_box` to the exact PDA implied by the snapshot
    // slot so a caller cannot pass an arbitrary non-empty account to skip
    // the init_ballot_box CPI (and its validation) below.
    let (expected_ballot_box, _) = Pubkey::find_program_address(
        &[b"BallotBox", &snapshot_slot.to_le_bytes()],
        ballot_program.key,
    );
    require_keys_eq!(
        ballot_box.key(),
        expected_ballot_box,
        GovernanceError::InvalidBallotBox
    );

    // start voting 1 epoch after snapshot
    // checking in any vote or others is start_epoch <= current_epoch < end_epoch
    proposal.start_epoch = target_epoch + 1;
    proposal.end_epoch = target_epoch + 1 + global_config.voting_epochs;
    proposal.snapshot_slot = snapshot_slot; // 1000 slots into snapshot

    let (consensus_result_pda, _) = Pubkey::find_program_address(
        &[b"ConsensusResult", &snapshot_slot.to_le_bytes()],
        ballot_program.key,
    );
    proposal.consensus_result = Some(consensus_result_pda);
    proposal.voting = true;

    if ballot_box.data_is_empty() {
        // Create seed components with sufficient lifetime
        let proposal_seed_val = proposal.proposal_seed.to_le_bytes();
        let vote_account_key = proposal.vote_account_pubkey.key();

        let seeds: &[&[u8]] = &[
            b"proposal".as_ref(),
            &proposal_seed_val,
            vote_account_key.as_ref(),
            &[proposal.proposal_bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            ballot_program.to_account_info(),
            ncn_snapshot::cpi::accounts::InitBallotBox {
                payer: payer.to_account_info(),
                proposal: proposal.to_account_info(),
                ballot_box: ballot_box.to_account_info(),
                program_config: program_config.to_account_info(),
                system_program: system_program.to_account_info(),
            },
            signer_seeds,
        );
        ncn_snapshot::cpi::init_ballot_box(
            cpi_ctx,
            snapshot_slot,
            proposal.proposal_seed,
            proposal.vote_account_pubkey,
        )?;
    }

    Ok(snapshot_slot)
}
