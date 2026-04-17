#![allow(unexpected_cfgs, unused_variables, clippy::too_many_arguments)]
mod constants;
mod error;
mod events;
mod instructions;
mod merkle_helpers;
mod state;
mod utils;
use anchor_lang::prelude::*;
use instructions::*;

use ncn_snapshot::StakeMerkleLeaf;

declare_id!("EKwRPoyRactBV2z2XhUSVU1YbZuyTVq4kU5U5dM2JyZY");

#[program]
pub mod svmgov_program {
    use super::*;

    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        max_title_length: u16,
        max_description_length: u16,
        max_support_epochs: u64,
        min_proposal_stake_lamports: u64,
        cluster_support_pct_min_bps: u64,
        discussion_epochs: u64,
        voting_epochs: u64,
        snapshot_epoch_extension: u64,
        snapshot_slot_offset: i64,
    ) -> Result<()> {
        ctx.accounts.initialize_config(
            max_title_length,
            max_description_length,
            max_support_epochs,
            min_proposal_stake_lamports,
            cluster_support_pct_min_bps,
            discussion_epochs,
            voting_epochs,
            snapshot_epoch_extension,
            snapshot_slot_offset,
            &ctx.bumps,
        )?;
        Ok(())
    }

    pub fn update_config(
        ctx: Context<UpdateConfig>,
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
        ctx.accounts.update_config(
            max_title_length,
            max_description_length,
            max_support_epochs,
            min_proposal_stake_lamports,
            cluster_support_pct_min_bps,
            discussion_epochs,
            voting_epochs,
            snapshot_epoch_extension,
            snapshot_slot_offset,
        )?;
        Ok(())
    }

    pub fn initialize_index(ctx: Context<InitializedIndex>) -> Result<()> {
        ctx.accounts.init_index(&ctx.bumps)?;
        Ok(())
    }

    pub fn create_proposal(
        ctx: Context<CreateProposal>,
        seed: u64,
        title: String,
        description: String,
    ) -> Result<()> {
        ctx.accounts
            .create_proposal(seed, title, description, &ctx.bumps)?;
        Ok(())
    }

    pub fn support_proposal(ctx: Context<SupportProposal>) -> Result<()> {
        ctx.accounts.support_proposal(&ctx.bumps)?;
        Ok(())
    }

    pub fn cast_vote(
        ctx: Context<CastVote>,
        for_votes_bp: u64,
        against_votes_bp: u64,
        abstain_votes_bp: u64,
    ) -> Result<()> {
        ctx.accounts
            .cast_vote(for_votes_bp, against_votes_bp, abstain_votes_bp, &ctx.bumps)?;
        Ok(())
    }

    pub fn modify_vote(
        ctx: Context<ModifyVote>,
        for_votes_bp: u64,
        against_votes_bp: u64,
        abstain_votes_bp: u64,
    ) -> Result<()> {
        ctx.accounts
            .modify_vote(for_votes_bp, against_votes_bp, abstain_votes_bp)?;
        Ok(())
    }

    pub fn cast_vote_override(
        ctx: Context<CastVoteOverride>,
        for_votes_bp: u64,
        against_votes_bp: u64,
        abstain_votes_bp: u64,
        stake_merkle_proof: Vec<[u8; 32]>,
        stake_merkle_leaf: StakeMerkleLeaf,
    ) -> Result<()> {
        ctx.accounts.cast_vote_override(
            for_votes_bp,
            against_votes_bp,
            abstain_votes_bp,
            stake_merkle_proof,
            stake_merkle_leaf,
            &ctx.bumps,
        )?;
        Ok(())
    }

    pub fn modify_vote_override(
        ctx: Context<ModifyVoteOverride>,
        for_votes_bp: u64,
        against_votes_bp: u64,
        abstain_votes_bp: u64,
        stake_merkle_proof: Vec<[u8; 32]>,
        stake_merkle_leaf: StakeMerkleLeaf,
    ) -> Result<()> {
        ctx.accounts.modify_vote_override(
            for_votes_bp,
            against_votes_bp,
            abstain_votes_bp,
            stake_merkle_proof,
            stake_merkle_leaf,
            &ctx.bumps,
        )?;
        Ok(())
    }

    pub fn finalize_proposal(ctx: Context<FinalizeProposal>) -> Result<()> {
        ctx.accounts.finalize_proposal()?;

        Ok(())
    }

    pub fn flush_merkle_root(ctx: Context<FlushMerkleRoot>) -> Result<()> {
        ctx.accounts.flush_merkle_root()?;
        Ok(())
    }
}
