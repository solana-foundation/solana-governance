#![allow(clippy::too_many_arguments)]

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
    events::ProposalCreated,
    stake_weight_bp,
    state::{GlobalConfig, Proposal, ProposalIndex, VoteDelegation},
    utils::is_valid_github_link,
    vote_authority::{read_vote_account_authorities, require_vote_authority},
};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct CreateProposal<'info> {
    #[account(mut)]
    pub signer: Signer<'info>, // Proposal creator (validator)
    #[account(
        init,
        payer = signer,
        seeds = [b"proposal", seed.to_le_bytes().as_ref(), spl_vote_account.key().as_ref()],
        bump,
        space = ANCHOR_DISCRIMINATOR + Proposal::INIT_SPACE,
    )]
    pub proposal: Account<'info, Proposal>,
    #[account(
        mut,
        seeds = [b"index"],
        bump = proposal_index.bump
    )]
    pub proposal_index: Account<'info, ProposalIndex>,
    /// CHECK: Owner == vote program and account size == VoteState::size_of() are
    /// enforced here; the handler then reads node_pubkey and requires the signer
    /// to be it (or its accepted delegate), proving the signer operates this
    /// vote account.
    #[account(
        constraint = spl_vote_account.owner == &vote_program::ID @ ProgramError::InvalidAccountOwner,
        constraint = spl_vote_account.data_len() == VoteState::size_of() @ GovernanceError::InvalidVoteAccountSize
    )]
    pub spl_vote_account: UncheckedAccount<'info>,
    /// CHECK: VoteDelegation PDA for `spl_vote_account`; may not exist. With an
    /// accepted delegation on record only the delegate may sign, otherwise only
    /// the vote account's node identity may.
    #[account(
        seeds = [VoteDelegation::SEED, spl_vote_account.key().as_ref()],
        bump
    )]
    pub vote_delegation: UncheckedAccount<'info>,
    #[account(
        seeds = [b"global_config"],
        bump = global_config.bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,

    pub system_program: Program<'info, System>,
}

impl<'info> CreateProposal<'info> {
    pub fn create_proposal(
        &mut self,
        seed: u64,
        title: String,
        description: String,
        bumps: &CreateProposalBumps,
    ) -> Result<()> {
        require!(
            self.global_config.new_proposals_allowed,
            GovernanceError::NewProposalsNotAllowed
        );
        // Validate proposal inputs
        require!(!title.is_empty(), GovernanceError::TitleEmpty);

        require!(
            title.len() <= self.global_config.max_title_length as usize,
            GovernanceError::TitleTooLong
        );
        require!(!description.is_empty(), GovernanceError::DescriptionEmpty);
        require!(
            description.len() <= self.global_config.max_description_length as usize,
            GovernanceError::DescriptionTooLong
        );
        require!(
            is_valid_github_link(&description),
            GovernanceError::DescriptionInvalid
        );

        let clock = Clock::get()?;

        // The signer must operate this vote account: its node identity, or the
        // accepted delegate. The proposal is attributed to the identity either way.
        let identity = {
            let vote_account_data = self.spl_vote_account.data.borrow();
            read_vote_account_authorities(&vote_account_data)
                .map_err(|_| GovernanceError::InvalidVoteAccount)?
                .node_pubkey
        };
        require_vote_authority(
            &self.signer.key(),
            &identity,
            &self.vote_delegation,
            &self.spl_vote_account.key(),
            GovernanceError::UnauthorizedVoteSigner,
        )?;

        // Calculate stake weight basis points
        let cluster_stake = get_epoch_total_stake();
        let proposer_stake = get_epoch_stake_for_vote_account(self.spl_vote_account.key);
        let proposer_stake_weight_bp = stake_weight_bp!(proposer_stake, cluster_stake)?;

        require!(
            proposer_stake >= self.global_config.min_proposal_stake_lamports,
            GovernanceError::NotEnoughStake
        );

        // Initialize proposal account
        self.proposal.set_inner(Proposal {
            author: identity,
            title,
            description,
            creation_epoch: clock.epoch,
            start_epoch: 0,
            end_epoch: 0,
            proposer_stake_weight_bp,
            proposal_bump: bumps.proposal,
            creation_timestamp: clock.unix_timestamp,
            index: self.proposal_index.current_index + 1,
            proposal_seed: seed,
            vote_account_pubkey: self.spl_vote_account.key(),
            ..Proposal::default()
        });
        self.proposal_index.current_index += 1;

        // Emit proposal created event
        emit!(ProposalCreated {
            proposal_id: self.proposal.key(),
            author: identity,
            title: self.proposal.title.clone(),
            description: self.proposal.description.clone(),
            creation_timestamp: self.proposal.creation_timestamp,
            signer: self.signer.key(),
        });

        Ok(())
    }
}
