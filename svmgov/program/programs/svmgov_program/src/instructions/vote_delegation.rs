use anchor_lang::prelude::*;

use crate::{
    constants::ANCHOR_DISCRIMINATOR,
    error::GovernanceError,
    events::{VoteDelegationAccepted, VoteDelegationCreated, VoteDelegationRemoved},
    state::VoteDelegation,
    vote_authority::{is_live_vote_account, read_vote_account_authorities},
};

/// Step 1 of vote delegation: names a delegate for `subject`.
///
/// When `subject` is a live SPL vote account the signer must be its node
/// identity or authorized withdrawer; otherwise `subject` is a voting wallet
/// and must be the signer itself. The delegation is inert until the delegate
/// calls `accept_vote_delegation`, so a mistyped delegate key cannot lock the
/// voter out and the subject's own key keeps voting in the meantime.
#[derive(Accounts)]
pub struct DelegateVote<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    /// CHECK: The SPL vote account or voting wallet whose vote is delegated. The
    /// handler decides which by owner and size, reads the vote account's
    /// authorities in the former case, and requires the wallet itself to sign in
    /// the latter.
    pub subject: UncheckedAccount<'info>,
    #[account(
        init,
        payer = signer,
        space = ANCHOR_DISCRIMINATOR + VoteDelegation::INIT_SPACE,
        seeds = [VoteDelegation::SEED, subject.key().as_ref()],
        bump
    )]
    pub vote_delegation: Account<'info, VoteDelegation>,
    pub system_program: Program<'info, System>,
}

impl<'info> DelegateVote<'info> {
    pub fn delegate_vote(&mut self, delegate: Pubkey, bumps: &DelegateVoteBumps) -> Result<()> {
        require_keys_neq!(
            delegate,
            Pubkey::default(),
            GovernanceError::InvalidVoteDelegate
        );

        let signer = self.signer.key();
        let subject = self.subject.key();
        // The owner check alone picks the branch, so a vote account can never be
        // delegated under wallet rules even by a holder of its keypair.
        let identity = if is_live_vote_account(&self.subject) {
            let authorities = read_vote_account_authorities(&self.subject.data.borrow())?;
            require!(
                signer == authorities.node_pubkey || signer == authorities.authorized_withdrawer,
                GovernanceError::UnauthorizedVoteDelegation
            );
            authorities.node_pubkey
        } else {
            require_keys_eq!(signer, subject, GovernanceError::UnauthorizedVoteDelegation);
            subject
        };

        self.vote_delegation.set_inner(VoteDelegation {
            delegate,
            subject,
            delegation_accepted: false,
            bump: bumps.vote_delegation,
        });

        emit!(VoteDelegationCreated {
            subject,
            identity,
            delegate,
            signer,
        });
        Ok(())
    }
}

/// Step 2 of vote delegation: the named delegate accepts. From this point the
/// delegate alone holds governance authority for the subject.
#[derive(Accounts)]
pub struct AcceptVoteDelegation<'info> {
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [VoteDelegation::SEED, vote_delegation.subject.as_ref()],
        bump = vote_delegation.bump,
        constraint = vote_delegation.delegate == signer.key() @ GovernanceError::NotVoteDelegate,
        constraint = !vote_delegation.delegation_accepted @ GovernanceError::VoteDelegationAlreadyAccepted,
    )]
    pub vote_delegation: Account<'info, VoteDelegation>,
}

impl<'info> AcceptVoteDelegation<'info> {
    pub fn accept_vote_delegation(&mut self) -> Result<()> {
        self.vote_delegation.delegation_accepted = true;

        emit!(VoteDelegationAccepted {
            subject: self.vote_delegation.subject,
            delegate: self.vote_delegation.delegate,
        });
        Ok(())
    }
}

/// Removes a vote delegation and returns its rent to the subject's root
/// authority: the authorized withdrawer of a live SPL vote account, or the
/// subject itself otherwise.
///
/// The delegate may always resign. For a live SPL vote account the authorized
/// withdrawer may always revoke, while the node identity may only withdraw a
/// delegation that has not been accepted: once accepted, the hot identity key
/// is locked out of governance until the delegate or the withdrawer removes
/// the delegation. For any other subject (a voting wallet, or a vote account
/// that has since been closed) the subject itself may always remove.
#[derive(Accounts)]
pub struct RemoveVoteDelegation<'info> {
    pub signer: Signer<'info>,
    /// CHECK: The SPL vote account or voting wallet the delegation was created
    /// for; the handler reads a live vote account's authorities to decide who
    /// may remove and who receives the rent.
    pub subject: UncheckedAccount<'info>,
    #[account(
        mut,
        close = rent_recipient,
        seeds = [VoteDelegation::SEED, subject.key().as_ref()],
        bump = vote_delegation.bump,
    )]
    pub vote_delegation: Account<'info, VoteDelegation>,
    /// CHECK: Receives the closed account's rent. The handler requires it to be
    /// the subject's authorized withdrawer for a live vote account, or the
    /// subject itself otherwise.
    #[account(mut)]
    pub rent_recipient: UncheckedAccount<'info>,
}

impl<'info> RemoveVoteDelegation<'info> {
    pub fn remove_vote_delegation(&mut self) -> Result<()> {
        let signer = self.signer.key();
        let subject = self.subject.key();
        let delegation = &self.vote_delegation;

        let (authorized, rent_recipient) = if is_live_vote_account(&self.subject) {
            let authorities = read_vote_account_authorities(&self.subject.data.borrow())?;
            (
                signer == delegation.delegate
                    || signer == authorities.authorized_withdrawer
                    || (!delegation.delegation_accepted && signer == authorities.node_pubkey),
                authorities.authorized_withdrawer,
            )
        } else {
            (signer == delegation.delegate || signer == subject, subject)
        };
        require!(
            authorized,
            GovernanceError::UnauthorizedVoteDelegationRemoval
        );
        // Anchor's `close` runs after this handler, so checking here is enough
        // to bind the refund to the subject's root authority.
        require_keys_eq!(
            self.rent_recipient.key(),
            rent_recipient,
            GovernanceError::InvalidVoteDelegationRentRecipient
        );

        emit!(VoteDelegationRemoved {
            subject: delegation.subject,
            delegate: delegation.delegate,
            signer,
            was_accepted: delegation.delegation_accepted,
        });
        Ok(())
    }
}
