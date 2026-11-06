use anchor_lang::prelude::*;

use crate::{
    error::GovernanceError,
    events::ProposalDescriptionUpdated,
    state::{GlobalConfig, Proposal, VoteDelegation},
    utils::{check_support_window, is_valid_github_link},
    vote_authority::require_vote_authority,
};

#[derive(Accounts)]
pub struct UpdateProposalDescription<'info> {
    pub signer: Signer<'info>,
    #[account(mut)]
    pub proposal: Account<'info, Proposal>,
    /// CHECK: VoteDelegation PDA for the proposal's vote account; may not exist.
    /// With an accepted delegation on record only the delegate may update the
    /// proposal, otherwise only its author (the vote account's identity) may.
    #[account(
        seeds = [VoteDelegation::SEED, proposal.vote_account_pubkey.as_ref()],
        bump
    )]
    pub vote_delegation: UncheckedAccount<'info>,
    #[account(
        seeds = [b"global_config"],
        bump = global_config.bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,
}

impl<'info> UpdateProposalDescription<'info> {
    pub fn update_proposal_description(&mut self, description: String) -> Result<()> {
        require_vote_authority(
            &self.signer.key(),
            &self.proposal.author,
            &self.vote_delegation,
            &self.proposal.vote_account_pubkey,
            GovernanceError::UnauthorizedProposalUpdate,
        )?;

        let clock = Clock::get()?;
        // Validate the proposal state
        {
            require!(!self.proposal.finalized, GovernanceError::ProposalFinalized);
            if self.proposal.voting {
                require!(
                    clock.epoch < self.proposal.start_epoch,
                    GovernanceError::CannotModifyAfterStart
                );
            } else {
                check_support_window(
                    clock.epoch,
                    self.proposal.creation_epoch,
                    self.global_config.max_support_epochs,
                )?;
            }
        }

        // Validate the new description
        {
            require!(!description.is_empty(), GovernanceError::DescriptionEmpty);
            require!(
                description.len() <= self.global_config.max_description_length as usize,
                GovernanceError::DescriptionTooLong
            );
            require!(
                is_valid_github_link(&description),
                GovernanceError::DescriptionInvalid
            );
        }

        let previous_description = core::mem::replace(&mut self.proposal.description, description);
        emit!(ProposalDescriptionUpdated {
            proposal_id: self.proposal.key(),
            author: self.proposal.author,
            previous_description,
            new_description: self.proposal.description.clone(),
            update_timestamp: clock.unix_timestamp,
            signer: self.signer.key(),
        });

        Ok(())
    }
}
