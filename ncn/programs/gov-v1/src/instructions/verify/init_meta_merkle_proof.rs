use anchor_lang::prelude::*;

use crate::{verify_shared_handler, ConsensusResult, MetaMerkleLeaf, MetaMerkleProof};

#[derive(Accounts)]
#[instruction(meta_merkle_leaf: MetaMerkleLeaf, meta_merkle_proof: Vec<[u8; 32]>)]
pub struct InitMetaMerkleProof<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        seeds = [
            b"MetaMerkleProof".as_ref(),
            &consensus_result.key().as_ref(),
            meta_merkle_leaf.vote_account.as_ref(),
        ],
        bump,
        payer = payer,
        space = 8 + MetaMerkleProof::init_space(meta_merkle_proof)
    )]
    pub merkle_proof: Box<Account<'info, MetaMerkleProof>>,
    pub consensus_result: Box<Account<'info, ConsensusResult>>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<InitMetaMerkleProof>,
    meta_merkle_leaf: MetaMerkleLeaf,
    meta_merkle_proof: Vec<[u8; 32]>,
    close_timestamp: i64,
) -> Result<()> {
    let merkle_proof = &mut ctx.accounts.merkle_proof;
    merkle_proof.payer = ctx.accounts.payer.key();
    merkle_proof.consensus_result = ctx.accounts.consensus_result.key();
    merkle_proof.meta_merkle_leaf = meta_merkle_leaf;
    merkle_proof.meta_merkle_proof = meta_merkle_proof;
    merkle_proof.close_timestamp = close_timestamp;

    // Verify using the provided proof that the leaf exists in consensus result root.
    verify_shared_handler(
        &ctx.accounts.merkle_proof,
        &ctx.accounts.consensus_result,
        None,
        None,
    )?;

    Ok(())
}
