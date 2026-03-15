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
    state::{GlobalConfig, Proposal, ProposalIndex},
    utils::is_valid_github_link,
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
    /// CHECK: Vote account is too big to deserialize, so we check on owner and size, then compare node_pubkey with signer
    #[account(
        constraint = spl_vote_account.owner == &vote_program::ID @ ProgramError::InvalidAccountOwner,
        constraint = spl_vote_account.data_len() == VoteState::size_of() @ GovernanceError::InvalidVoteAccountSize
    )]
    pub spl_vote_account: UncheckedAccount<'info>,
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

        let vote_account_data = self.spl_vote_account.data.borrow();
        // TODO: Check if this deserialization is correct
        let vote_account = match VoteState::deserialize(&vote_account_data) {
            Ok(vote_account) => vote_account,
            Err(_) => return Err(GovernanceError::InvalidVoteAccount.into()),
        };

        // Ensuring signer is the same as the vote account node_pubkey
        require_keys_eq!(
            vote_account.node_pubkey,
            self.signer.key(),
            GovernanceError::InvalidVoteAccount
        );

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
            author: self.signer.key(),
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
            author: self.signer.key(),
            title: self.proposal.title.clone(),
            description: self.proposal.description.clone(),
            creation_timestamp: self.proposal.creation_timestamp,
        });

        Ok(())
    }
}
