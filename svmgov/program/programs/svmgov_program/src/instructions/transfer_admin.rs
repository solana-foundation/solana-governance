use anchor_lang::prelude::*;

use crate::{
    error::GovernanceError,
    events::{AdminNominated, AdminTransferred},
    state::GlobalConfig,
};

/// Step 1 of the two-step admin transfer: the current admin nominates a new admin.
///
/// The nomination does not take effect until the nominee calls `accept_admin`. This
/// proves the nominee key is controllable before authority is handed over, which makes
/// the transfer safe for multisig-to-multisig handoffs (each side signs its own, separate
/// transaction) while preventing an accidental, irreversible lock-out of admin access.
#[derive(Accounts)]
pub struct NominateAdmin<'info> {
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
}

impl<'info> NominateAdmin<'info> {
    pub fn nominate_admin(&mut self, proposed_admin: Pubkey) -> Result<()> {
        require_keys_neq!(
            proposed_admin,
            Pubkey::default(),
            GovernanceError::InvalidAdmin
        );

        let config = &mut self.global_config;
        // Overwrites any prior nomination, allowing the admin to correct a mistake
        // before the previous nominee accepts.
        config.pending_admin = Some(proposed_admin);

        emit!(AdminNominated {
            current_admin: config.admin,
            pending_admin: proposed_admin,
        });

        Ok(())
    }
}

/// Step 2 of the two-step admin transfer: the nominated admin accepts and becomes the
/// active admin. Only the pending admin can sign this, so authority is never transferred
/// to a key that cannot sign.
#[derive(Accounts)]
pub struct AcceptAdmin<'info> {
    pub new_admin: Signer<'info>,
    #[account(
        mut,
        seeds = [b"global_config"],
        bump = global_config.bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,
}

impl<'info> AcceptAdmin<'info> {
    pub fn accept_admin(&mut self) -> Result<()> {
        let new_admin = self.new_admin.key();
        let config = &mut self.global_config;

        let pending_admin = config
            .pending_admin
            .ok_or(GovernanceError::NoPendingAdmin)?;
        require_keys_eq!(pending_admin, new_admin, GovernanceError::NotPendingAdmin);

        let previous_admin = config.admin;
        config.admin = new_admin;
        config.pending_admin = None;

        emit!(AdminTransferred {
            previous_admin,
            new_admin,
        });

        Ok(())
    }
}
