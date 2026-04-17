use anchor_lang::prelude::*;

use crate::{
    constants::{ADMIN_PUBKEY, ANCHOR_DISCRIMINATOR},
    error::GovernanceError,
    state::GlobalConfig,
};

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(
        mut,
        constraint = admin.key() == ADMIN_PUBKEY @ GovernanceError::UnauthorizedAdmin,
    )]
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
        self.global_config.set_inner(GlobalConfig {
            admin: self.admin.key(),
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
