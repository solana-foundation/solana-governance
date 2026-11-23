use anchor_lang::{
    prelude::*,
    system_program::{self, Transfer},
    Discriminator,
};

use crate::{
    constants::{ANCHOR_DISCRIMINATOR, DELEGATE_SIGNER_SPACE},
    error::GovernanceError,
    state::{Vote, VoteOverride},
};

/// One-time, permissionless migration of `Vote` and `VoteOverride` accounts
/// recorded before `delegate_signer` existed.
///
/// The accounts to migrate are passed as remaining accounts, each writable.
/// Every account must be owned by this program and carry the `Vote` or
/// `VoteOverride` discriminator. One at the legacy size grows to the current
/// size, with the payer funding the added rent; one already at the current
/// size is left untouched, so a batch may be retried. Any other size is
/// rejected.
#[derive(Accounts)]
pub struct MigrateVoteRecords<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> MigrateVoteRecords<'info> {
    pub fn migrate_vote_records(&mut self, records: &[AccountInfo<'info>]) -> Result<()> {
        let rent = Rent::get()?;
        for record in records {
            require_keys_eq!(*record.owner, crate::ID, GovernanceError::InvalidVoteRecord);

            let current_len = record.data_len();
            let target_len = {
                let data = record.data.borrow();
                require!(
                    current_len >= ANCHOR_DISCRIMINATOR,
                    GovernanceError::InvalidVoteRecord
                );
                let discriminator = &data[..ANCHOR_DISCRIMINATOR];
                if discriminator == Vote::DISCRIMINATOR {
                    ANCHOR_DISCRIMINATOR + Vote::INIT_SPACE
                } else if discriminator == VoteOverride::DISCRIMINATOR {
                    ANCHOR_DISCRIMINATOR + VoteOverride::INIT_SPACE
                } else {
                    return Err(GovernanceError::InvalidVoteRecord.into());
                }
            };
            if current_len == target_len {
                continue;
            }
            require!(
                current_len == target_len - DELEGATE_SIGNER_SPACE,
                GovernanceError::InvalidVoteRecord
            );

            // Fund the growth before resizing so the account never dips below
            // rent exemption, which the runtime checks against the new size.
            let shortfall = rent
                .minimum_balance(target_len)
                .saturating_sub(record.lamports());
            if shortfall > 0 {
                system_program::transfer(
                    CpiContext::new(
                        self.system_program.to_account_info(),
                        Transfer {
                            from: self.payer.to_account_info(),
                            to: record.clone(),
                        },
                    ),
                    shortfall,
                )?;
            }
            // Borsh encodes `None` as a single zero byte, so zero-filled growth
            // already reads as `delegate_signer: None`.
            record.realloc(target_len, true)?;
        }
        Ok(())
    }
}
