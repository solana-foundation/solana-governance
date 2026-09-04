use anchor_lang::prelude::*;

use crate::{
    error::GovernanceError,
    instructions::{
        validate_cluster_support_pct_min_bps, validate_max_description_length,
        validate_max_supporters, validate_max_title_length,
    },
    state::GlobalConfig,
};

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: Validated after possible resize
    #[account(
        mut,
        seeds = [b"global_config"],
        bump
    )]
    pub global_config: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
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
        max_supporters: Option<u32>,
        mut new_proposals_allowed: Option<bool>,
    ) -> Result<()> {
        if self.requires_migration() {
            // resize
            let new_size = GlobalConfig::INIT_SPACE + 8;
            let min_rent = Rent::get()?.minimum_balance(new_size);

            if self.global_config.lamports() < min_rent {
                anchor_lang::system_program::transfer(
                    anchor_lang::context::CpiContext::new(
                        self.system_program.to_account_info(),
                        anchor_lang::system_program::Transfer {
                            from: self.admin.to_account_info(),
                            to: self.global_config.to_account_info(),
                        },
                    ),
                    min_rent.checked_sub(self.global_config.lamports()).unwrap(),
                )?;
            }

            self.global_config.realloc(new_size, true)?;

            if new_proposals_allowed.is_none() {
                new_proposals_allowed = Some(false);
            }
        }

        let mut config =
            GlobalConfig::try_deserialize(&mut self.global_config.try_borrow_data()?.as_ref())?;

        require_keys_eq!(
            config.admin,
            self.admin.key(),
            GovernanceError::UnauthorizedAdmin
        );

        if let Some(v) = max_title_length {
            validate_max_title_length(v)?;
            config.max_title_length = v;
        }
        if let Some(v) = max_description_length {
            validate_max_description_length(v)?;
            config.max_description_length = v;
        }
        if let Some(v) = max_support_epochs {
            config.max_support_epochs = v;
        }
        if let Some(v) = min_proposal_stake_lamports {
            config.min_proposal_stake_lamports = v;
        }
        if let Some(v) = cluster_support_pct_min_bps {
            validate_cluster_support_pct_min_bps(v)?;
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
        if let Some(v) = max_supporters {
            validate_max_supporters(v)?;
            config.max_supporters = v;
        }
        if let Some(v) = new_proposals_allowed {
            config.new_proposals_allowed = v;
        }
        let mut data = self.global_config.try_borrow_mut_data()?;
        let mut dst: &mut [u8] = &mut data;
        GlobalConfig::try_serialize(&config, &mut dst)?;

        Ok(())
    }

    fn requires_migration(&self) -> bool {
        self.global_config.data_len() < GlobalConfig::INIT_SPACE + 8
    }
}
