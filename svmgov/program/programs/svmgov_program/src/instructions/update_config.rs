use anchor_lang::prelude::*;

use crate::{constants::ADMIN_PUBKEY, error::GovernanceError, state::GlobalConfig};

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(
        constraint = admin.key() == ADMIN_PUBKEY @ GovernanceError::UnauthorizedAdmin,
    )]
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [b"global_config"],
        bump = global_config.bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,
}

impl<'info> UpdateConfig<'info> {
    pub fn update_config(
        &mut self,
        max_title_length: Option<u16>,
        max_description_length: Option<u16>,
        max_support_epochs: Option<u64>,
        min_proposal_stake_lamports: Option<u64>,
        cluster_support_pct_min_bps: Option<u64>,
        discussion_epochs: Option<u64>,
        voting_epochs: Option<u64>,
        snapshot_epoch_extension: Option<u64>,
        snapshot_slot_offset: Option<i64>,
    ) -> Result<()> {
        let config = &mut self.global_config;

        if let Some(v) = max_title_length {
            config.max_title_length = v;
        }
        if let Some(v) = max_description_length {
            config.max_description_length = v;
        }
        if let Some(v) = max_support_epochs {
            config.max_support_epochs = v;
        }
        if let Some(v) = min_proposal_stake_lamports {
            config.min_proposal_stake_lamports = v;
        }
        if let Some(v) = cluster_support_pct_min_bps {
            config.cluster_support_pct_min_bps = v;
        }
        if let Some(v) = discussion_epochs {
            config.discussion_epochs = v;
        }
        if let Some(v) = voting_epochs {
            config.voting_epochs = v;
        }
        if let Some(v) = snapshot_epoch_extension {
            config.snapshot_epoch_extension = v;
        }
        if let Some(v) = snapshot_slot_offset {
            config.snapshot_slot_offset = v;
        }

        Ok(())
    }
}
