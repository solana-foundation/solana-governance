#![allow(ambiguous_glob_reexports)]
#![allow(unexpected_cfgs)] // See: https://solana.stackexchange.com/a/19845

pub mod error;
pub mod instructions;
pub mod merkle_helper;
pub mod state;

use anchor_lang::prelude::*;

pub use instructions::*;
pub use state::*;

declare_id!("8qua2VAnWaNFDAhUakryMuRdNRVuUehkjnhLECvueGSV");

#[program]
pub mod ncn_snapshot {
    use super::*;

    pub fn init_program_config(ctx: Context<InitProgramConfig>) -> Result<()> {
        init_program_config::handler(ctx)
    }

    pub fn update_operator_whitelist(
        ctx: Context<UpdateOperatorWhitelist>,
        operators_to_add: Option<Vec<Pubkey>>,
        operators_to_remove: Option<Vec<Pubkey>>,
    ) -> Result<()> {
        update_operator_whitelist::handler(ctx, operators_to_add, operators_to_remove)
    }

    pub fn update_program_config(
        ctx: Context<UpdateProgramConfig>,
        proposed_authority: Option<Pubkey>,
        min_consensus_threshold_bps: Option<u16>,
        tie_breaker_admin: Option<Pubkey>,
        vote_duration: Option<i64>,
    ) -> Result<()> {
        update_program_config::handler(
            ctx,
            proposed_authority,
            min_consensus_threshold_bps,
            tie_breaker_admin,
            vote_duration,
        )
    }

    pub fn finalize_proposed_authority(ctx: Context<FinalizeProposedAuthority>) -> Result<()> {
        finalize_proposed_authority::handler(ctx)
    }

    pub fn init_ballot_box(
        ctx: Context<InitBallotBox>,
        snapshot_slot: u64,
        proposal_seed: u64,
        spl_vote_account: Pubkey,
    ) -> Result<()> {
        init_ballot_box::handler(ctx, snapshot_slot, proposal_seed, spl_vote_account)
    }

    pub fn cast_vote(ctx: Context<CastVote>, ballot: Ballot) -> Result<()> {
        cast_vote::handler(ctx, ballot)
    }

    pub fn remove_vote(ctx: Context<RemoveVote>) -> Result<()> {
        remove_vote::handler(ctx)
    }

    pub fn set_tie_breaker(ctx: Context<SetTieBreaker>, ballot: Ballot) -> Result<()> {
        set_tie_breaker::handler(ctx, ballot)
    }

    pub fn reset_ballot_box(ctx: Context<ResetBallotBox>) -> Result<()> {
        reset_ballot_box::handler(ctx)
    }

    pub fn finalize_ballot(ctx: Context<FinalizeBallot>) -> Result<()> {
        finalize_ballot::handler(ctx)
    }

    pub fn init_meta_merkle_proof(
        ctx: Context<InitMetaMerkleProof>,
        meta_merkle_leaf: MetaMerkleLeaf,
        meta_merkle_proof: Vec<[u8; 32]>,
        close_timestamp: i64,
    ) -> Result<()> {
        init_meta_merkle_proof::handler(ctx, meta_merkle_leaf, meta_merkle_proof, close_timestamp)
    }

    pub fn close_meta_merkle_proof(ctx: Context<CloseMetaMerkleProof>) -> Result<()> {
        close_meta_merkle_proof::handler(ctx)
    }

    pub fn verify_merkle_proof(
        ctx: Context<VerifyMerkleProof>,
        stake_merkle_proof: Option<Vec<[u8; 32]>>,
        stake_merkle_leaf: Option<StakeMerkleLeaf>,
    ) -> Result<()> {
        verify_merkle_proof::handler(ctx, stake_merkle_proof, stake_merkle_leaf)
    }
}
