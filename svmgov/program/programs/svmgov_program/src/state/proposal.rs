use crate::{constants::*, error::GovernanceError};
use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Proposal {
    /// The public key of the validator who created this proposal
    pub author: Pubkey,
    #[max_len(MAX_TITLE_ACCOUNT_SIZE)]
    pub title: String,
    #[max_len(MAX_DESC_ACCOUNT_SIZE)]
    pub description: String,
    pub creation_epoch: u64,
    pub start_epoch: u64,
    pub end_epoch: u64,
    pub proposer_stake_weight_bp: u64,
    pub cluster_support_lamports: u64,
    /// Total lamports voted in favor of this proposal
    pub for_votes_lamports: u64,
    /// Total lamports voted against this proposal
    pub against_votes_lamports: u64,
    /// Total lamports that abstained from voting on this proposal
    pub abstain_votes_lamports: u64,
    pub voting: bool,
    pub finalized: bool,
    pub proposal_bump: u8,
    pub creation_timestamp: i64,
    pub vote_count: u32,
    pub index: u32,
    pub consensus_result: Option<Pubkey>,
    /// Slot number when the validator stake snapshot was taken
    pub snapshot_slot: u64,
    // Seeds for CPI
    pub proposal_seed: u64,
    pub vote_account_pubkey: Pubkey,
}

impl Default for Proposal {
    fn default() -> Self {
        Self {
            author: Pubkey::default(),
            title: "".to_string(),
            description: "".to_string(),
            creation_epoch: 0,
            start_epoch: 0,
            end_epoch: 0,
            proposer_stake_weight_bp: 0,
            cluster_support_lamports: 0,
            for_votes_lamports: 0,
            against_votes_lamports: 0,
            abstain_votes_lamports: 0,
            voting: false,
            finalized: false,
            proposal_bump: 0,
            creation_timestamp: 0,
            vote_count: 0,
            index: 0,
            snapshot_slot: 0,
            consensus_result: None,
            proposal_seed: 0,
            vote_account_pubkey: Pubkey::default(),
        }
    }
}

impl Proposal {
    pub fn add_vote_lamports(
        &mut self,
        for_votes: u64,
        against_votes: u64,
        abstain_votes: u64,
    ) -> Result<()> {
        self.for_votes_lamports = self
            .for_votes_lamports
            .checked_add(for_votes)
            .ok_or(GovernanceError::ArithmeticOverflow)?;

        self.against_votes_lamports = self
            .against_votes_lamports
            .checked_add(against_votes)
            .ok_or(GovernanceError::ArithmeticOverflow)?;

        self.abstain_votes_lamports = self
            .abstain_votes_lamports
            .checked_add(abstain_votes)
            .ok_or(GovernanceError::ArithmeticOverflow)?;

        Ok(())
    }

    pub fn sub_vote_lamports(
        &mut self,
        for_votes: u64,
        against_votes: u64,
        abstain_votes: u64,
    ) -> Result<()> {
        self.for_votes_lamports = self
            .for_votes_lamports
            .checked_sub(for_votes)
            .ok_or(GovernanceError::ArithmeticOverflow)?;

        self.against_votes_lamports = self
            .against_votes_lamports
            .checked_sub(against_votes)
            .ok_or(GovernanceError::ArithmeticOverflow)?;

        self.abstain_votes_lamports = self
            .abstain_votes_lamports
            .checked_sub(abstain_votes)
            .ok_or(GovernanceError::ArithmeticOverflow)?;

        Ok(())
    }

    pub fn add_cluster_support(&mut self, support_lamports: u64) -> Result<()> {
        self.cluster_support_lamports = self
            .cluster_support_lamports
            .checked_add(support_lamports)
            .ok_or(GovernanceError::ArithmeticOverflow)?;

        Ok(())
    }
}
