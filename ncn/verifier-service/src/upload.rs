//! Upload handling for snapshot files

use std::str::FromStr;

use anyhow::Result;
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::Json,
};
use cli::{upload_signature_message, MetaMerkleSnapshot};
use meta_merkle_tree::{merkle_tree::MerkleTree, utils::get_proof};
use serde_json::{json, Value};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use sqlx::sqlite::SqlitePool;
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
    let (slot, network, merkle_root, provided_snapshot_hash, signature) =
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

    // 3. Verify signature over slot || network || merkle_root || snapshot_hash before
    // touching the file body.
    verify_signature(
        &slot,
        &network,
        &merkle_root,
        &provided_snapshot_hash,
        &signature,
    )
    .map_err(|e| {
        info!("Signature verification failed: {}", e);
        metrics::record_upload_outcome(metrics::UploadOutcome::Unauthorized);
        StatusCode::UNAUTHORIZED
    })?;
    info!(
        "Verified upload request: slot={}, network={}, merkle_root={}, snapshot_hash={}",
        slot, network, merkle_root, provided_snapshot_hash
    );

    // 4. Load the file.
    let file_data = extract_remaining_file(&mut multipart).await.map_err(|e| {
        info!("Failed to extract file: {}", e);
        metrics::record_upload_outcome(metrics::UploadOutcome::BadRequest);
        StatusCode::BAD_REQUEST
    })?;
    info!("Processing upload file ({} bytes)", file_data.len());

    // 5. Parse snapshot file and verify it matches the signed request fields.
    let (snapshot, computed_snapshot_hash) =
        MetaMerkleSnapshot::read_from_bytes_with_hash(file_data, true).map_err(|e| {
            info!("Failed to read snapshot: {}", e);
            metrics::record_upload_outcome(metrics::UploadOutcome::BadRequest);
            StatusCode::BAD_REQUEST
        })?;
    let encoded_hash = bs58::encode(computed_snapshot_hash).into_string();

    if bs58::encode(snapshot.root).into_string() != merkle_root
        || snapshot.slot != slot
        || encoded_hash != provided_snapshot_hash
    {
        info!("Merkle root, slot, or snapshot hash mismatch");
        metrics::record_upload_outcome(metrics::UploadOutcome::BadRequest);
        return Err(StatusCode::BAD_REQUEST);
    }

    // 6. Reject snapshots whose bundled stake leaves do not match the signed meta leaves.
    validate_stake_merkle_roots(&snapshot).map_err(|e| {
        info!("Invalid stake merkle root: {}", e);
        metrics::record_upload_outcome(metrics::UploadOutcome::BadRequest);
        StatusCode::BAD_REQUEST
    })?;

    // 7. Reconstruct the meta merkle tree from the uploaded leaves and confirm it
    // reproduces the signed root. Every proof we later serve is derived from this
    // tree, so the uploaded `bundle.proof` bytes are never trusted or persisted.
    let meta_merkle_tree = reconstruct_meta_merkle_tree(&snapshot).map_err(|e| {
        info!("Failed to reconstruct meta merkle tree: {}", e);
        metrics::record_upload_outcome(metrics::UploadOutcome::BadRequest);
        StatusCode::BAD_REQUEST
    })?;

    // 8. Index data in database
    index_snapshot_data(
        &pool,
        &snapshot,
        &meta_merkle_tree,
        &network,
        &merkle_root,
        &encoded_hash,
    )
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

fn validate_stake_merkle_roots(snapshot: &MetaMerkleSnapshot) -> Result<()> {
    for (bundle_idx, bundle) in snapshot.leaf_bundles.iter().enumerate() {
        let derived_root = derive_stake_merkle_root(&bundle.stake_merkle_leaves)
            .ok_or_else(|| anyhow::anyhow!("bundle {} has no stake merkle root", bundle_idx))?;

        if derived_root != bundle.meta_merkle_leaf.stake_merkle_root {
            return Err(anyhow::anyhow!(
                "bundle {} stake root mismatch for vote account {}",
                bundle_idx,
                bundle.meta_merkle_leaf.vote_account
            ));
        }
    }

    Ok(())
}

fn derive_stake_merkle_root(
    stake_merkle_leaves: &[ncn_snapshot::StakeMerkleLeaf],
) -> Option<[u8; 32]> {
    let hashed_nodes: Vec<[u8; 32]> = stake_merkle_leaves
        .iter()
        .map(|n| n.hash().to_bytes())
        .collect();
    let stake_merkle = MerkleTree::new(&hashed_nodes[..], true);
    stake_merkle.get_root().map(|root| root.to_bytes())
}

/// Rebuild the meta merkle tree from the uploaded meta leaves and confirm it
/// reproduces the snapshot's signed root.
///
/// The bundles are stored in the same order in which the canonical tree was
/// built, so leaf `i` corresponds to tree index `i`. Rehashing the leaves in
/// that order yields the exact tree, which lets us (a) reject any snapshot whose
/// leaves do not hash up to the signed root and (b) derive each vote account's
/// proof ourselves instead of trusting the unsigned `bundle.proof` bytes. The
/// returned tree is consumed by [`index_snapshot_data`] to compute proofs that
/// are guaranteed to verify against the on-chain consensus root.
fn reconstruct_meta_merkle_tree(snapshot: &MetaMerkleSnapshot) -> Result<MerkleTree> {
    let hashed_nodes: Vec<[u8; 32]> = snapshot
        .leaf_bundles
        .iter()
        .map(|bundle| bundle.meta_merkle_leaf.hash().to_bytes())
        .collect();
    let meta_merkle = MerkleTree::new(&hashed_nodes[..], true);

    let derived_root = meta_merkle
        .get_root()
        .ok_or_else(|| anyhow::anyhow!("meta merkle tree has no root"))?
        .to_bytes();

    if derived_root != snapshot.root {
        return Err(anyhow::anyhow!(
            "derived meta merkle root {} does not match signed root {}",
            bs58::encode(derived_root).into_string(),
            bs58::encode(snapshot.root).into_string()
        ));
    }

    Ok(meta_merkle)
}

/// Index snapshot data in the database
async fn index_snapshot_data(
    pool: &SqlitePool,
    snapshot: &MetaMerkleSnapshot,
    meta_merkle_tree: &MerkleTree,
    network: &str,
    merkle_root: &str,
    snapshot_hash: &str,
) -> Result<()> {
    // Begin transaction for atomic indexing
    let mut tx = pool.begin().await?;

    // Treat a same-slot reupload as a full replacement, not a merge. Clear any
    // rows left by a previous upload of this `(network, slot)` before
    // repopulating, so accounts that are omitted from the new snapshot cannot
    // survive. Without this, an upsert-only path leaves stale rows behind while
    // `snapshot_meta` advertises the new `snapshot_hash`, yielding a hybrid
    // snapshot where `/meta` and `/proof`/`/voter` disagree. The deletes and
    // inserts share one transaction, so readers never observe a partial state.
    VoteAccountRecord::delete_by_slot(&mut *tx, network, snapshot.slot).await?;
    StakeAccountRecord::delete_by_slot(&mut *tx, network, snapshot.slot).await?;

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

        // Derive the meta merkle proof from the reconstructed tree rather than
        // trusting the unsigned `bundle.proof` bytes carried in the upload. Bundle
        // `bundle_idx` maps to tree index `bundle_idx` by construction, so this
        // proof is guaranteed to verify against the signed root.
        let meta_merkle_proof: Vec<String> = get_proof(meta_merkle_tree, bundle_idx)
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
async fn extract_metadata_only(
    multipart: &mut Multipart,
) -> Result<(u64, String, String, String, String)> {
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
    let snapshot_hash = extract_field!("snapshot_hash");
    let signature = extract_field!("signature");
    Ok((slot, network, merkle_root, snapshot_hash, signature))
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

/// Verify Ed25519 signature over slot || network || merkle_root || snapshot_hash.
fn verify_signature(
    slot: &u64,
    network: &str,
    merkle_root: &str,
    snapshot_hash: &str,
    signature: &str,
) -> Result<()> {
    // Get operator pubkey from environment variable
    let operator_pubkey_str = std::env::var("OPERATOR_PUBKEY")
        .map_err(|_| anyhow::anyhow!("OPERATOR_PUBKEY env not set"))?;
    let operator_pubkey = Pubkey::from_str(&operator_pubkey_str)?;

    let message = upload_signature_message(*slot, network, merkle_root, snapshot_hash);

    let signature = Signature::from_str(signature)?;
    if !signature.verify(&operator_pubkey.to_bytes(), &message) {
        return Err(anyhow::anyhow!("Verification failed"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::prelude::Pubkey as AnchorPubkey;
    use solana_sdk::{signature::Keypair, signer::Signer};
    use std::env;

    const SLOT1: u64 = 12345;
    const NETWORK1: &str = "testnet";
    const NETWORK2: &str = "mainnet";
    const ROOT1: &str = "test_merkle_root_hash";
    const ROOT2: &str = "different_merkle_root_hash";
    const HASH1: &str = "test_snapshot_hash";
    const HASH2: &str = "different_snapshot_hash";

    /// Helper to set up environment
    fn setup_env(pubkey: &str) {
        env::set_var("OPERATOR_PUBKEY", pubkey);
    }

    /// Helper to create keypair and sign message
    fn create_signed_message(
        slot: u64,
        network: &str,
        merkle_root: &str,
        snapshot_hash: &str,
    ) -> (Keypair, String) {
        let keypair = Keypair::new();

        let message = upload_signature_message(slot, network, merkle_root, snapshot_hash);
        let signature = keypair.sign_message(&message);
        (keypair, signature.to_string())
    }

    #[test]
    #[serial_test::serial]
    fn test_verify_signature_success() {
        let (keypair, signature) = create_signed_message(SLOT1, NETWORK1, ROOT1, HASH1);
        setup_env(&keypair.pubkey().to_string());

        let result = verify_signature(&SLOT1, NETWORK1, ROOT1, HASH1, &signature);
        assert!(result.is_ok(), "Verification should succeed");
    }

    #[test]
    #[serial_test::serial]
    fn test_verify_signature_invalid_signature() {
        let (keypair, _) = create_signed_message(SLOT1, NETWORK1, ROOT1, HASH1);
        let (_, wrong_signature) = create_signed_message(SLOT1, NETWORK1, ROOT1, HASH1);
        setup_env(&keypair.pubkey().to_string());

        let result = verify_signature(&SLOT1, NETWORK1, ROOT1, HASH1, &wrong_signature);
        assert!(
            result.is_err(),
            "Verification should fail with wrong signature"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_verify_signature_missing_env_var() {
        env::remove_var("OPERATOR_PUBKEY");

        let result = verify_signature(&SLOT1, NETWORK1, ROOT1, HASH1, "dummy");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("OPERATOR_PUBKEY env not set"));
    }

    #[test]
    #[serial_test::serial]
    fn test_verify_signature_different_message() {
        let (keypair, signature) = create_signed_message(SLOT1, NETWORK1, ROOT1, HASH1);
        setup_env(&keypair.pubkey().to_string());

        let result = verify_signature(&SLOT1, NETWORK1, ROOT2, HASH1, &signature);
        assert!(result.is_err(), "Should fail with different merkle root");

        let result = verify_signature(&SLOT1, NETWORK2, ROOT1, HASH1, &signature);
        assert!(result.is_err(), "Should fail with different network");

        let result = verify_signature(&SLOT1, NETWORK1, ROOT1, HASH2, &signature);
        assert!(result.is_err(), "Should fail with different snapshot hash");
    }

    #[test]
    fn test_validate_stake_merkle_roots_rejects_incoherent_bundle() {
        let stake_leaf = ncn_snapshot::StakeMerkleLeaf {
            voting_wallet: AnchorPubkey::new_unique(),
            stake_account: AnchorPubkey::new_unique(),
            active_stake: 10,
        };
        let stake_merkle_root = derive_stake_merkle_root(std::slice::from_ref(&stake_leaf))
            .expect("stake root should be derived");
        let snapshot = MetaMerkleSnapshot {
            root: [0; 32],
            leaf_bundles: vec![cli::MetaMerkleLeafBundle {
                meta_merkle_leaf: ncn_snapshot::MetaMerkleLeaf {
                    voting_wallet: AnchorPubkey::new_unique(),
                    vote_account: AnchorPubkey::new_unique(),
                    stake_merkle_root,
                    active_stake: stake_leaf.active_stake,
                },
                stake_merkle_leaves: vec![ncn_snapshot::StakeMerkleLeaf {
                    active_stake: stake_leaf.active_stake + 1,
                    ..stake_leaf
                }],
                proof: None,
            }],
            slot: SLOT1,
        };

        let result = validate_stake_merkle_roots(&snapshot);
        assert!(result.is_err(), "incoherent stake roots must be rejected");
    }

    /// Build a coherent snapshot from arbitrary meta leaves, mirroring how the CLI
    /// generates one: hash the leaves in order, build the tree, and store each
    /// bundle's canonical proof alongside the derived root.
    fn build_snapshot(slot: u64, count: u8) -> MetaMerkleSnapshot {
        let leaves: Vec<ncn_snapshot::MetaMerkleLeaf> = (0..count)
            .map(|seed| ncn_snapshot::MetaMerkleLeaf {
                voting_wallet: AnchorPubkey::new_unique(),
                vote_account: AnchorPubkey::new_unique(),
                stake_merkle_root: [seed; 32],
                active_stake: u64::from(seed) + 1,
            })
            .collect();

        let hashed_nodes: Vec<[u8; 32]> = leaves.iter().map(|l| l.hash().to_bytes()).collect();
        let tree = MerkleTree::new(&hashed_nodes[..], true);
        let root = tree.get_root().expect("root").to_bytes();

        let leaf_bundles = leaves
            .into_iter()
            .enumerate()
            .map(|(i, meta_merkle_leaf)| cli::MetaMerkleLeafBundle {
                meta_merkle_leaf,
                stake_merkle_leaves: vec![],
                proof: Some(get_proof(&tree, i)),
            })
            .collect();

        MetaMerkleSnapshot {
            root,
            leaf_bundles,
            slot,
        }
    }

    #[test]
    fn test_reconstruct_meta_merkle_tree_accepts_coherent_snapshot() {
        let snapshot = build_snapshot(SLOT1, 5);
        let tree = reconstruct_meta_merkle_tree(&snapshot).expect("coherent snapshot reconstructs");

        // Every proof derived from the reconstructed tree must verify against the
        // signed root using the exact logic the program runs on-chain.
        for (i, bundle) in snapshot.leaf_bundles.iter().enumerate() {
            let proof = get_proof(&tree, i);
            assert!(
                ncn_snapshot::merkle_helper::verify_helper(
                    &bundle.meta_merkle_leaf.hash().to_bytes(),
                    &proof,
                    snapshot.root.into(),
                )
                .is_ok(),
                "derived proof for leaf {} should verify against signed root",
                i
            );
        }
    }

    #[test]
    fn test_reconstruct_meta_merkle_tree_rejects_root_mismatch() {
        let mut snapshot = build_snapshot(SLOT1, 5);
        // Claimed root no longer matches the leaves it is supposed to commit to.
        snapshot.root[0] ^= 0xff;

        assert!(
            reconstruct_meta_merkle_tree(&snapshot).is_err(),
            "snapshot whose leaves do not hash to the signed root must be rejected"
        );
    }

    #[test]
    fn test_reconstruct_meta_merkle_tree_rejects_tampered_leaf() {
        let mut snapshot = build_snapshot(SLOT1, 5);
        // Mutate a leaf while leaving the signed root untouched: the derived root
        // now diverges from the signed root, so the upload must be rejected.
        snapshot.leaf_bundles[0].meta_merkle_leaf.active_stake += 1;

        assert!(
            reconstruct_meta_merkle_tree(&snapshot).is_err(),
            "tampered leaf must break root reconstruction"
        );
    }

    #[test]
    fn test_reconstruct_meta_merkle_tree_ignores_uploaded_proof_bytes() {
        let mut snapshot = build_snapshot(SLOT1, 5);
        // Poison the uploaded proof bytes for one bundle. Reconstruction works off
        // the leaves alone, so the derived proof stays canonical and verifiable.
        snapshot.leaf_bundles[0].proof = Some(vec![[0xaa; 32], [0xbb; 32]]);
        snapshot.leaf_bundles[2].proof = None;

        let tree = reconstruct_meta_merkle_tree(&snapshot)
            .expect("poisoned proof bytes do not affect reconstruction");

        let derived = get_proof(&tree, 0);
        assert_ne!(
            derived,
            vec![[0xaa; 32], [0xbb; 32]],
            "derived proof must not echo the poisoned upload bytes"
        );
        assert!(
            ncn_snapshot::merkle_helper::verify_helper(
                &snapshot.leaf_bundles[0].meta_merkle_leaf.hash().to_bytes(),
                &derived,
                snapshot.root.into(),
            )
            .is_ok(),
            "derived proof must verify even when uploaded proof bytes are poisoned"
        );
    }
}
