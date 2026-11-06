use anchor_lang::{
    prelude::*,
    solana_program::vote::{program as vote_program, state::VoteState},
};

use crate::{
    constants::ANCHOR_DISCRIMINATOR,
    error::GovernanceError,
    events::{VoteDelegationAccepted, VoteDelegationCreated, VoteDelegationRemoved},
    state::VoteDelegation,
    vote_authority::read_vote_account_authorities,
};

/// Step 1 of vote delegation: the vote account's node identity or authorized
/// withdrawer names a delegate. The delegation is inert until the delegate calls
/// `accept_vote_delegation`, so a mistyped delegate key cannot lock the validator
/// out and the identity keeps voting in the meantime.
#[derive(Accounts)]
pub struct DelegateVote<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    /// CHECK: Owner == vote program and account size == VoteState::size_of() are
    /// enforced here; the handler requires the signer to be this vote account's
    /// node identity or authorized withdrawer.
    #[account(
        constraint = spl_vote_account.owner == &vote_program::ID @ ProgramError::InvalidAccountOwner,
        constraint = spl_vote_account.data_len() == VoteState::size_of() @ GovernanceError::InvalidVoteAccountSize
    )]
    pub spl_vote_account: UncheckedAccount<'info>,
    #[account(
        init,
        payer = signer,
        space = ANCHOR_DISCRIMINATOR + VoteDelegation::INIT_SPACE,
        seeds = [VoteDelegation::SEED, spl_vote_account.key().as_ref()],
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

        let authorities = read_vote_account_authorities(&self.spl_vote_account.data.borrow())?;
        let signer = self.signer.key();
        require!(
            signer == authorities.node_pubkey || signer == authorities.authorized_withdrawer,
            GovernanceError::UnauthorizedVoteDelegation
        );

        self.vote_delegation.set_inner(VoteDelegation {
            delegate,
            vote_account: self.spl_vote_account.key(),
            creator: signer,
            delegation_accepted: false,
            bump: bumps.vote_delegation,
        });

        emit!(VoteDelegationCreated {
            vote_account: self.spl_vote_account.key(),
            identity: authorities.node_pubkey,
            delegate,
            creator: signer,
        });
        Ok(())
    }
}

/// Step 2 of vote delegation: the named delegate accepts. From this point the
/// delegate alone holds governance authority for the vote account.
#[derive(Accounts)]
pub struct AcceptVoteDelegation<'info> {
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [VoteDelegation::SEED, vote_delegation.vote_account.as_ref()],
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
            vote_account: self.vote_delegation.vote_account,
            delegate: self.vote_delegation.delegate,
        });
        Ok(())
    }
}

/// Removes a vote delegation and returns its rent to the creator.
///
/// The delegate may always resign and the vote account's authorized withdrawer
/// may always revoke, so a lost or hostile delegate key never strands the vote.
/// The node identity may only withdraw a delegation that has not been accepted:
/// once accepted, the hot identity key is locked out of governance until the
/// delegate or the withdrawer removes the delegation.
#[derive(Accounts)]
pub struct RemoveVoteDelegation<'info> {
    pub signer: Signer<'info>,
    /// CHECK: The SPL vote account the delegation was created for. While it is a
    /// live vote account (owner and size checked in the handler) its authorities
    /// decide who may remove; if it has since been closed, only the delegate or
    /// the creator may reclaim the rent.
    pub spl_vote_account: UncheckedAccount<'info>,
    #[account(
        mut,
        close = creator,
        seeds = [VoteDelegation::SEED, spl_vote_account.key().as_ref()],
        bump = vote_delegation.bump,
    )]
    pub vote_delegation: Account<'info, VoteDelegation>,
    /// CHECK: Receives the closed account's rent; must be the recorded creator.
    #[account(
        mut,
        address = vote_delegation.creator @ GovernanceError::InvalidVoteDelegationCreator
    )]
    pub creator: UncheckedAccount<'info>,
}

impl<'info> RemoveVoteDelegation<'info> {
    pub fn remove_vote_delegation(&mut self) -> Result<()> {
        let signer = self.signer.key();
        let delegation = &self.vote_delegation;

        let is_live_vote_account = self.spl_vote_account.owner == &vote_program::ID
            && self.spl_vote_account.data_len() == VoteState::size_of();
        let authorized = if is_live_vote_account {
            let authorities = read_vote_account_authorities(&self.spl_vote_account.data.borrow())?;
            signer == delegation.delegate
                || signer == authorities.authorized_withdrawer
                || (!delegation.delegation_accepted && signer == authorities.node_pubkey)
        } else {
            signer == delegation.delegate || signer == delegation.creator
        };
        require!(
            authorized,
            GovernanceError::UnauthorizedVoteDelegationRemoval
        );

        emit!(VoteDelegationRemoved {
            vote_account: delegation.vote_account,
            delegate: delegation.delegate,
            signer,
            was_accepted: delegation.delegation_accepted,
        });
        Ok(())
    }
}
