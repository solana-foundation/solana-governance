use anchor_lang::prelude::*;

use crate::{error::GovernanceError, state::GlobalConfig};

#[derive(Accounts)]
pub struct UpdateConfigAdmin<'info> {
    #[account(
        constraint = admin.key() == global_config.admin @ GovernanceError::UnauthorizedAdmin,
    )]
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [b"global_config"],
        bump = global_config.bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,

    pub proposed_admin: Signer<'info>,
}

impl<'info> UpdateConfigAdmin<'info> {
    pub fn update_config_admin(&mut self) -> Result<()> {
        let config = &mut self.global_config;

        config.admin = self.proposed_admin.key();

        Ok(())
    }
}
