use anchor_lang::prelude::*;

use crate::{
    constants::{
        ANCHOR_DISCRIMINATOR, BASIS_POINTS_MAX, MAX_DESC_ACCOUNT_SIZE, MAX_TITLE_ACCOUNT_SIZE,
    },
    error::GovernanceError,
    state::GlobalConfig,
};

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = ANCHOR_DISCRIMINATOR + GlobalConfig::INIT_SPACE,
        seeds = [b"global_config"],
        bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,
    pub system_program: Program<'info, System>,
    // links the passed program_data to THIS program's id
    #[account(
      constraint = program.programdata_address()? == Some(program_data.key()) @ GovernanceError::InvalidProgram
  )]
    pub program: Program<'info, crate::program::SvmgovProgram>,
    // Ensure the admin is the upgrade authority for the program to prevent unauthorized initialization
    #[account(
        constraint = program_data.upgrade_authority_address == Some(admin.key()) @ GovernanceError::UnauthorizedAdmin
    )]
    pub program_data: Account<'info, ProgramData>,
}

impl<'info> InitializeConfig<'info> {
    pub fn initialize_config(
        &mut self,
        max_title_length: u16,
        max_description_length: u16,
        max_support_epochs: u64,
        min_proposal_stake_lamports: u64,
        cluster_support_pct_min_bps: u64,
        discussion_epochs: u64,
        voting_epochs: u64,
        snapshot_epoch_extension: u64,
        snapshot_slot_offset: i64,
        bumps: &InitializeConfigBumps,
    ) -> Result<()> {
        validate_cluster_support_pct_min_bps(cluster_support_pct_min_bps)?;

        validate_max_title_length(max_title_length)?;

        validate_max_description_length(max_description_length)?;

        self.global_config.set_inner(GlobalConfig {
            admin: self.admin.key(),
            pending_admin: None,
            max_title_length,
            max_description_length,
            max_support_epochs,
            min_proposal_stake_lamports,
            cluster_support_pct_min_bps,
            discussion_epochs,
            voting_epochs,
            snapshot_epoch_extension,
            snapshot_slot_offset,
            bump: bumps.global_config,
        });
        Ok(())
    }
}

/// Validate that the cluster support percentage minimum in basis points is between 0 and 10,000 (inclusive)
/// This will prevent a downstream error when calculating the required support for a proposal, which would
/// occur if this value is set above 10,000 (100%).
pub fn validate_cluster_support_pct_min_bps(cluster_support_pct_min_bps: u64) -> Result<()> {
    if cluster_support_pct_min_bps > BASIS_POINTS_MAX {
        return Err(GovernanceError::InvalidClusterSupportPctMin.into());
    }
    Ok(())
}

/// Validate that the max title length is greater than 0 and less than the maximum title size enforced
/// downstream.
pub fn validate_max_title_length(max_title_length: u16) -> Result<()> {
    if max_title_length == 0 || max_title_length as usize > MAX_TITLE_ACCOUNT_SIZE {
        return Err(GovernanceError::InvalidMaxTitleLength.into());
    }
    Ok(())
}

/// Validate that the max description length is greater than 0 and less than the maximum description size enforced
/// downstream.
pub fn validate_max_description_length(max_description_length: u16) -> Result<()> {
    if max_description_length == 0 || max_description_length as usize > MAX_DESC_ACCOUNT_SIZE {
        return Err(GovernanceError::InvalidMaxDescriptionLength.into());
    }
    Ok(())
}
