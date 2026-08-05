use anchor_lang::{
    prelude::*,
    solana_program::epoch_stake::{get_epoch_stake_for_vote_account, get_epoch_total_stake},
};

use crate::{
    error::GovernanceError,
    events::ProposalRetallied,
    instructions::support_proposal::activate_voting,
    state::{GlobalConfig, Proposal},
    utils::{check_support_window, min_stake_threshold, tally_supporter_stakes},
};

/// Permissionless re-tally of a proposal's support at the current epoch.
///
/// Support stake is only re-measured when a *new* supporter arrives, and the
/// Support PDA prevents anyone from supporting twice. Without this
/// instruction, a proposal left just under the threshold would stay inactive
/// even if stake drift alone later pushed its existing supporters over the
/// line — nothing would trigger a re-measure. Anyone may call this during the
/// support window to re-tally and, if the threshold is now met, activate
/// voting.
#[derive(Accounts)]
pub struct RetallySupport<'info> {
    /// Any caller. Pays only the tx fee — plus the ballot-box rent iff this
    /// retally activates voting and the ballot box does not exist yet.
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub proposal: Account<'info, Proposal>,

    /// CHECK: Ballot box account - may or may not exist, checked with data_is_empty()
    #[account(mut)]
    pub ballot_box: UncheckedAccount<'info>,

    /// CHECK: Ballot program account
    #[account(
        constraint = ballot_program.key == &ncn_snapshot::ID @ ProgramError::InvalidAccountOwner,
    )]
    pub ballot_program: UncheckedAccount<'info>,

    /// CHECK: Program config account
    #[account(
        seeds = [b"ProgramConfig"],
        bump,
        seeds::program = ballot_program.key(),
        constraint = program_config.owner == &ncn_snapshot::ID @ ProgramError::InvalidAccountOwner,
    )]
    pub program_config: UncheckedAccount<'info>,
    #[account(
        seeds = [b"global_config"],
        bump = global_config.bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,

    pub system_program: Program<'info, System>,
}

impl<'info> RetallySupport<'info> {
    pub fn retally_support(&mut self) -> Result<()> {
        let clock = Clock::get()?;

        // Same eligibility gates as support_proposal: not yet voting, not
        // finalized, and inside the support window.
        require!(
            self.proposal.voting == false && self.proposal.finalized == false,
            GovernanceError::ProposalClosed
        );
        check_support_window(
            clock.epoch,
            self.proposal.creation_epoch,
            self.global_config.max_support_epochs,
        )?;

        // A retally of zero supporters is meaningless — and under a zero-bps
        // threshold config it could activate voting on a proposal nobody
        // supported, so refuse it outright.
        require!(
            self.proposal.num_supporters > 0,
            GovernanceError::NoSupporters
        );

        // Rebuild the tally from every supporter's stake at the CURRENT epoch;
        // nothing from earlier epochs' readings is retained. The supporter
        // list is read zero-copy from the account data.
        let proposal_info = self.proposal.to_account_info();
        let total = {
            let proposal_data = proposal_info.try_borrow_data()?;
            let supporters = self.proposal.supporters(&proposal_data)?;
            tally_supporter_stakes(supporters, |pk| get_epoch_stake_for_vote_account(pk))?
        };
        self.proposal.cluster_support_lamports = total;

        let cluster_min_stake = min_stake_threshold(
            get_epoch_total_stake(),
            self.global_config.cluster_support_pct_min_bps,
        )?;
        self.proposal.support_threshold = cluster_min_stake;
        self.proposal.last_support_epoch = clock.epoch;

        let mut snapshot_slot = 0;
        if total >= cluster_min_stake {
            snapshot_slot = activate_voting(
                &mut self.proposal,
                &self.global_config,
                &self.signer,
                &self.ballot_box,
                &self.ballot_program,
                &self.program_config,
                &self.system_program,
                &clock,
            )?;
        }

        emit!(ProposalRetallied {
            proposal_id: self.proposal.key(),
            caller: self.signer.key(),
            cluster_support_lamports: total,
            voting_activated: self.proposal.voting,
            snapshot_slot: snapshot_slot,
        });

        Ok(())
    }
}
