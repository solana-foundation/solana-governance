use anchor_lang::{
    prelude::*,
    solana_program::{
        epoch_stake::{get_epoch_stake_for_vote_account, get_epoch_total_stake},
        vote::{program as vote_program, state::VoteState},
    },
};

use crate::{
    constants::ANCHOR_DISCRIMINATOR,
    error::GovernanceError,
    events::ProposalSupported,
    state::{GlobalConfig, Proposal, Support},
    utils::get_epoch_slot_range,
};

#[derive(Accounts)]
pub struct SupportProposal<'info> {
    #[account(mut)]
    pub signer: Signer<'info>, // Proposal supporter (validator)
    #[account(mut)]
    pub proposal: Account<'info, Proposal>,
    #[account(
        init,
        payer = signer,
        space = ANCHOR_DISCRIMINATOR + Support::INIT_SPACE,
        seeds = [b"support", proposal.key().as_ref(), spl_vote_account.key().as_ref()],
        bump
    )]
    pub support: Account<'info, Support>, // New support account
    /// CHECK: Vote account is too big to deserialize, so we check on owner and size, then compare node_pubkey with signer
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

        // Ensure proposal is eligible for support
        require!(
            self.proposal.voting == false && self.proposal.finalized == false,
            GovernanceError::ProposalClosed
        );

        require!(
            clock.epoch == self.proposal.creation_epoch + self.global_config.max_support_epochs,
            GovernanceError::NotInSupportPeriod
        );

        // assuming this returns in lamports
        let supporter_stake = get_epoch_stake_for_vote_account(self.spl_vote_account.key);

        let proposal_account = &mut self.proposal;
        let new_support_stake = proposal_account
            .cluster_support_lamports
            .checked_add(supporter_stake)
            .ok_or(GovernanceError::ArithmeticOverflow)?;

        // update the cluster support
        proposal_account.cluster_support_lamports = new_support_stake;

        // Initialize the support account
        self.support.set_inner(Support {
            proposal: proposal_account.key(),
            validator: self.signer.key(),
            bump: bumps.support,
        });

        let cluster_stake = get_epoch_total_stake();

        let cluster_min_stake = (cluster_stake as u128)
            .checked_mul(self.global_config.cluster_support_pct_min_bps as u128)
            .and_then(|v| v.checked_div(10_000))
            .ok_or(GovernanceError::ArithmeticOverflow)
            .map(|result| result as u64)?;

        let mut current_voting_emit = proposal_account.voting;
        let mut snapshot_slot = 0;
        proposal_account.voting = if new_support_stake >= cluster_min_stake {
            // this is for emit checks
            current_voting_emit = true;

            let (start_slot, _) = get_epoch_slot_range(
                clock.epoch
                    + self.global_config.discussion_epochs
                    + self.global_config.snapshot_epoch_extension,
            );
            let offset_result = (start_slot as i64)
                .checked_add(self.global_config.snapshot_slot_offset)
                .ok_or(GovernanceError::ArithmeticOverflow)?;
            require!(offset_result >= 0, GovernanceError::ArithmeticOverflow);
            snapshot_slot = offset_result as u64;
            // start voting 1 epoch after snapshot
            // checking in any vote or others is start_epoch <= current_epoch < end_epoch
            proposal_account.start_epoch = clock.epoch
                + self.global_config.discussion_epochs
                + self.global_config.snapshot_epoch_extension
                + 1;
            proposal_account.end_epoch = clock.epoch
                + self.global_config.discussion_epochs
                + self.global_config.snapshot_epoch_extension
                + 1
                + self.global_config.voting_epochs;
            proposal_account.snapshot_slot = snapshot_slot; // 1000 slots into snapshot

            let (consensus_result_pda, _) = Pubkey::find_program_address(
                &[b"ConsensusResult", &snapshot_slot.to_le_bytes()],
                &self.ballot_program.key,
            );

            proposal_account.consensus_result = Some(consensus_result_pda);

            if self.ballot_box.data_is_empty() {
                // Create seed components with sufficient lifetime
                let proposal_seed_val = proposal_account.proposal_seed.to_le_bytes();
                let vote_account_key = proposal_account.vote_account_pubkey.key();

                let seeds: &[&[u8]] = &[
                    b"proposal".as_ref(),
                    &proposal_seed_val,
                    vote_account_key.as_ref(),
                    &[proposal_account.proposal_bump],
                ];
                let signer_seeds = &[&seeds[..]];

                let cpi_ctx = CpiContext::new_with_signer(
                    self.ballot_program.to_account_info(),
                    ncn_snapshot::cpi::accounts::InitBallotBox {
                        payer: self.signer.to_account_info(),
                        proposal: proposal_account.to_account_info(),
                        ballot_box: self.ballot_box.to_account_info(),
                        program_config: self.program_config.to_account_info(),
                        system_program: self.system_program.to_account_info(),
                    },
                    signer_seeds,
                );
                ncn_snapshot::cpi::init_ballot_box(
                    cpi_ctx,
                    snapshot_slot,
                    proposal_account.proposal_seed,
                    proposal_account.vote_account_pubkey,
                )?;
            }

            true
        } else {
            false
        };

        emit!(ProposalSupported {
            proposal_id: self.proposal.key(),
            supporter: self.signer.key(),
            cluster_support_lamports: new_support_stake,
            voting_activated: current_voting_emit,
            snapshot_slot: snapshot_slot,
        });

        Ok(())
    }
}
