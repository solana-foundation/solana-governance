use anchor_lang::prelude::*;

use crate::{
    error::GovernanceError,
    events::ProposalDescriptionUpdated,
    state::{GlobalConfig, Proposal},
    utils::{check_support_window, is_valid_github_link},
};

#[derive(Accounts)]
pub struct UpdateProposalDescription<'info> {
    pub signer: Signer<'info>,
    #[account(
        mut,
        constraint = proposal.author == signer.key() @ GovernanceError::UnauthorizedProposalUpdate,
    )]
    pub proposal: Account<'info, Proposal>,
    #[account(
        seeds = [b"global_config"],
        bump = global_config.bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,
}

impl<'info> UpdateProposalDescription<'info> {
    pub fn update_proposal_description(&mut self, description: String) -> Result<()> {
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
            author: self.signer.key(),
            previous_description,
            new_description: self.proposal.description.clone(),
            update_timestamp: clock.unix_timestamp,
        });

        Ok(())
    }
}
