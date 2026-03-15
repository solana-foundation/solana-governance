use anchor_lang::prelude::*;

use gov_v1::{
    cpi::{accounts::VerifyMerkleProof, verify_merkle_proof},
    StakeMerkleLeaf,
};

/// Generic CPI function for merkle proof verification
/// Supports both validator and delegator verification
pub fn verify_merkle_proof_cpi<'info>(
    meta_merkle_proof_account: &AccountInfo<'info>,
    consensus_result_account: &AccountInfo<'info>,
    gov_v1_program: &AccountInfo<'info>,
    stake_merkle_proof: Option<Vec<[u8; 32]>>,
    stake_merkle_leaf: Option<StakeMerkleLeaf>,
) -> Result<()> {
    let cpi_accounts = VerifyMerkleProof {
        meta_merkle_proof: meta_merkle_proof_account.clone(),
        consensus_result: consensus_result_account.clone(),
    };

    let cpi_ctx = CpiContext::new(gov_v1_program.clone(), cpi_accounts);

    // Call verify_merkle_proof from gov-v1 program
    verify_merkle_proof(cpi_ctx, stake_merkle_proof, stake_merkle_leaf)?;

    Ok(())
}
