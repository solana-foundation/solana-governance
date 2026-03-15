//! Upload handling for snapshot files

use std::str::FromStr;

use anyhow::Result;
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::Json,
};
use cli::MetaMerkleSnapshot;
use meta_merkle_tree::{merkle_tree::MerkleTree, utils::get_proof};
use serde_json::{json, Value};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use sqlx::sqlite::SqlitePool;
use sqlx::Acquire;
use tracing::{debug, info};

use crate::database::models::{SnapshotMetaRecord, StakeAccountRecord, VoteAccountRecord};
use crate::metrics;
use crate::utils::validate_network;

/// Handle POST /upload endpoint
pub async fn handle_upload(
    State(pool): State<SqlitePool>,
    mut multipart: Multipart,
) -> Result<Json<Value>, StatusCode> {
    info!("POST /upload - Snapshot upload requested");

    // 1. Extract metadata fields first.
    let (slot, network, merkle_root, signature) =
        extract_metadata_only(&mut multipart).await.map_err(|e| {
            info!("Failed to extract metadata: {}", e);
            metrics::record_upload_outcome(metrics::UploadOutcome::BadRequest);
            StatusCode::BAD_REQUEST
        })?;

    // 2. Validate network is supported
    if let Err(e) = validate_network(&network) {
        metrics::record_upload_outcome(metrics::UploadOutcome::BadRequest);
        return Err(e);
    }

    // 3: Verify signature over slot || merkle_root_bs58_bytes
    verify_signature(&slot, &merkle_root, &signature).map_err(|e| {
        info!("Signature verification failed: {}", e);
        metrics::record_upload_outcome(metrics::UploadOutcome::Unauthorized);
        StatusCode::UNAUTHORIZED
    })?;
    info!(
        "Verified upload request: slot={}, merkle_root={}, signature={}",
        slot, merkle_root, signature
    );

    // 4. Load the file.
    let file_data = extract_remaining_file(&mut multipart).await.map_err(|e| {
        info!("Failed to extract file: {}", e);
        metrics::record_upload_outcome(metrics::UploadOutcome::BadRequest);
        StatusCode::BAD_REQUEST
    })?;
    info!(
        "Signature verified, processing file ({} bytes)",
        file_data.len()
    );

    // 5. Parse snapshot file, verify merkle_root and slot from request fields.
    let (snapshot, snapshot_hash) = MetaMerkleSnapshot::read_from_bytes_with_hash(file_data, true)
        .map_err(|e| {
            info!("Failed to read snapshot: {}", e);
            metrics::record_upload_outcome(metrics::UploadOutcome::BadRequest);
            StatusCode::BAD_REQUEST
        })?;
    let encoded_hash = bs58::encode(snapshot_hash).into_string();

    if bs58::encode(snapshot.root).into_string() != merkle_root || snapshot.slot != slot {
        info!("Merkle root or slot in snapshot mismatch");
        metrics::record_upload_outcome(metrics::UploadOutcome::BadRequest);
        return Err(StatusCode::BAD_REQUEST);
    }

    // 6. Index data in database
    index_snapshot_data(&pool, &snapshot, &network, &merkle_root, &encoded_hash)
        .await
        .map_err(|e| {
            info!("Failed to index snapshot data: {}", e);
            metrics::record_upload_outcome(metrics::UploadOutcome::Internal);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    metrics::record_upload_outcome(metrics::UploadOutcome::Success);

    Ok(Json(json!({
        "status": "success",
        "slot": slot,
        "merkle_root": merkle_root,
    })))
}

/// Index snapshot data in the database
async fn index_snapshot_data(
    pool: &SqlitePool,
    snapshot: &MetaMerkleSnapshot,
    network: &str,
    merkle_root: &str,
    snapshot_hash: &str,
) -> Result<()> {
    // Begin transaction for atomic indexing
    let mut tx = pool.begin().await?;

    // Index vote accounts and stake accounts
    for (bundle_idx, bundle) in snapshot.leaf_bundles.iter().enumerate() {
        if bundle_idx % 100 == 0 {
            info!(
                "Indexing bundle {} / {}",
                bundle_idx,
                snapshot.leaf_bundles.len()
            );
        }
        let meta_leaf = &bundle.meta_merkle_leaf;

        // Convert meta merkle proof to base58 strings
        let meta_merkle_proof: Vec<String> = bundle
            .proof
            .as_ref()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|hash| bs58::encode(hash).into_string())
            .collect();

        // Create vote account record
        let vote_account_record = VoteAccountRecord {
            network: network.to_string(),
            snapshot_slot: snapshot.slot,
            vote_account: meta_leaf.vote_account.to_string(),
            voting_wallet: meta_leaf.voting_wallet.to_string(),
            stake_merkle_root: bs58::encode(meta_leaf.stake_merkle_root).into_string(),
            active_stake: meta_leaf.active_stake,
            meta_merkle_proof,
        };
        vote_account_record.insert_exec(&mut *tx).await?;

        // Generate stake merkle tree under vote account
        let hashed_nodes: Vec<[u8; 32]> = bundle
            .stake_merkle_leaves
            .iter()
            .map(|n| n.hash().to_bytes())
            .collect();
        let stake_merkle = MerkleTree::new(&hashed_nodes[..], true);

        // Create stake account records for each stake leaf
        for (idx, stake_leaf) in bundle.stake_merkle_leaves.iter().enumerate() {
            let stake_merkle_proof = get_proof(&stake_merkle, idx)
                .iter()
                .map(|hash| bs58::encode(hash).into_string())
                .collect();

            let stake_account_record = StakeAccountRecord {
                network: network.to_string(),
                snapshot_slot: snapshot.slot,
                stake_account: stake_leaf.stake_account.to_string(),
                vote_account: meta_leaf.vote_account.to_string(),
                voting_wallet: stake_leaf.voting_wallet.to_string(),
                active_stake: stake_leaf.active_stake,
                stake_merkle_proof,
            };

            stake_account_record.insert_exec(&mut *tx).await?;
        }

        debug!(
            "Indexed bundle {}: vote_account={}, {} stake accounts",
            bundle_idx,
            meta_leaf.vote_account,
            bundle.stake_merkle_leaves.len()
        );
    }

    let snapshot_meta = SnapshotMetaRecord {
        network: network.to_string(),
        slot: snapshot.slot,
        merkle_root: merkle_root.to_string(),
        snapshot_hash: snapshot_hash.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    snapshot_meta.insert_exec(&mut *tx).await?;

    tx.commit().await?;

    info!(
        "Successfully indexed snapshot for slot {} with {} vote accounts",
        snapshot.slot,
        snapshot.leaf_bundles.len()
    );

    Ok(())
}

/// Extract metadata fields in sequence.
async fn extract_metadata_only(multipart: &mut Multipart) -> Result<(u64, String, String, String)> {
    macro_rules! extract_field {
        ($name:expr) => {
            multipart
                .next_field()
                .await?
                .ok_or_else(|| anyhow::anyhow!("Next field is missing {}", $name))?
                .text()
                .await?
        };
    }

    let slot = extract_field!("slot").parse()?;
    let network = extract_field!("network");
    let merkle_root = extract_field!("merkle_root");
    let signature = extract_field!("signature");
    Ok((slot, network, merkle_root, signature))
}

/// Extract the remaining file field (after metadata extraction).
async fn extract_remaining_file(multipart: &mut Multipart) -> Result<Vec<u8>> {
    Ok(multipart
        .next_field()
        .await?
        .ok_or_else(|| anyhow::anyhow!("Missing file"))?
        .bytes()
        .await?
        .to_vec())
}

/// Verify Ed25519 signature over slot || merkle_root_bs58_bytes
fn verify_signature(slot: &u64, merkle_root: &str, signature: &str) -> Result<()> {
    // Get operator pubkey from environment variable
    let operator_pubkey_str = std::env::var("OPERATOR_PUBKEY")
        .map_err(|_| anyhow::anyhow!("OPERATOR_PUBKEY env not set"))?;
    let operator_pubkey = Pubkey::from_str(&operator_pubkey_str)?;

    let mut message = Vec::new();
    message.extend_from_slice(&slot.to_le_bytes());
    message.extend_from_slice(merkle_root.as_bytes());

    let signature = Signature::from_str(signature)?;
    if !signature.verify(&operator_pubkey.to_bytes(), &message) {
        return Err(anyhow::anyhow!("Verification failed"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{signature::Keypair, signer::Signer};
    use std::env;

    const SLOT1: u64 = 12345;
    const ROOT1: &str = "test_merkle_root_hash";
    const ROOT2: &str = "different_merkle_root_hash";

    /// Helper to set up environment
    fn setup_env(pubkey: &str) {
        env::set_var("OPERATOR_PUBKEY", pubkey);
    }

    /// Helper to create keypair and sign message
    fn create_signed_message(slot: u64, merkle_root: &str) -> (Keypair, String) {
        let keypair = Keypair::new();

        let mut message = Vec::new();
        message.extend_from_slice(&slot.to_le_bytes());
        message.extend_from_slice(merkle_root.as_bytes());

        let signature = keypair.sign_message(&message);
        (keypair, signature.to_string())
    }

    #[test]
    #[serial_test::serial]
    fn test_verify_signature_success() {
        let (keypair, signature) = create_signed_message(SLOT1, ROOT1);
        setup_env(&keypair.pubkey().to_string());

        let result = verify_signature(&SLOT1, ROOT1, &signature);
        assert!(result.is_ok(), "Verification should succeed");
    }

    #[test]
    #[serial_test::serial]
    fn test_verify_signature_invalid_signature() {
        let (keypair, _) = create_signed_message(SLOT1, ROOT1);
        let (_, wrong_signature) = create_signed_message(SLOT1, ROOT1);
        setup_env(&keypair.pubkey().to_string());

        let result = verify_signature(&SLOT1, ROOT1, &wrong_signature);
        assert!(
            result.is_err(),
            "Verification should fail with wrong signature"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_verify_signature_missing_env_var() {
        env::remove_var("OPERATOR_PUBKEY");

        let result = verify_signature(&SLOT1, ROOT1, "dummy");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("OPERATOR_PUBKEY env not set"));
    }

    #[test]
    #[serial_test::serial]
    fn test_verify_signature_different_message() {
        let (keypair, signature) = create_signed_message(SLOT1, ROOT1);
        setup_env(&keypair.pubkey().to_string());

        let result = verify_signature(&SLOT1, ROOT2, &signature);
        assert!(result.is_err(), "Should fail with different message");
    }
}
