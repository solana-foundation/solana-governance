mod common;
use common::setup_server;

use reqwest::{
    multipart::{Form, Part},
    StatusCode,
};
use solana_sdk::{signature::Keypair, signer::Signer};

const NETWORK: &str = "testnet";

fn sign_upload_message(
    keypair: &Keypair,
    slot: u64,
    network: &str,
    merkle_root: &str,
    snapshot_hash: &str,
) -> String {
    let message = cli::upload_signature_message(slot, network, merkle_root, snapshot_hash);
    keypair.sign_message(&message).to_string()
}

#[tokio::test]
#[serial_test::serial]
async fn e2e_binary_endpoints() -> anyhow::Result<()> {
    let keypair = Keypair::new();
    let (base_url, _guard) = setup_server(&keypair).await?;

    // Load and parse the snapshot that will be uploaded
    let snapshot_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests/src/fixtures/meta_merkle_340850340.zip");
    let bytes = tokio::fs::read(&snapshot_path).await?;
    let (snapshot, snapshot_hash) =
        cli::MetaMerkleSnapshot::read_from_bytes_with_hash(bytes.clone(), true)?;
    let slot = snapshot.slot;
    let merkle_root = bs58::encode(snapshot.root).into_string();
    let encoded_hash = bs58::encode(snapshot_hash.to_bytes()).into_string();
    let signature = sign_upload_message(&keypair, slot, NETWORK, &merkle_root, &encoded_hash);

    // Test GET /healthz
    let client = reqwest::Client::new();
    let health = client.get(format!("{}/healthz", base_url)).send().await?;
    assert!(health.status().is_success());

    // Test POST /upload
    let form = Form::new()
        .text("slot", slot.to_string())
        .text("network", NETWORK)
        .text("merkle_root", merkle_root.clone())
        .text("snapshot_hash", encoded_hash.clone())
        .text("signature", signature)
        .part("file", Part::bytes(bytes).file_name("meta_merkle.bin"));

    let resp = client
        .post(format!("{}/upload", base_url))
        .multipart(form)
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "upload failed status={}",
        resp.status()
    );

    // Test GET /meta
    let meta: serde_json::Value = client
        .get(format!("{}/meta?network=testnet", base_url))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let expected_meta = serde_json::json!({
        "network": "testnet",
        "slot": slot,
        "merkle_root": merkle_root,
        "snapshot_hash": encoded_hash,
        "created_at": meta["created_at"],
    });
    assert_eq!(meta, expected_meta);

    // Test GET /voter
    let voter: serde_json::Value = client
        .get(format!(
            "{}/voter/AECaNinQ6ptWzZcD9WYFimvZuf37kuviUuNGGA4hgWDz?network=testnet&slot=340850340",
            base_url
        ))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let expected = serde_json::json!({
        "network": "testnet",
        "snapshot_slot": slot,
        "voting_wallet": "AECaNinQ6ptWzZcD9WYFimvZuf37kuviUuNGGA4hgWDz",
        "vote_accounts": [
            {
                "vote_account": "Mvrzoe3cvKFyY8WqVa7Y4ZGnH3KTdEAcez7esRYY67r",
                "active_stake": 32615567722979u64
            }
        ],
        "stake_accounts": [
            {
                "stake_account": "Fu12SHuZyaQ4B1or3hFRmx5gqLuGhxTWUjdH98oYRK2N",
                "vote_account": "Mvrzoe3cvKFyY8WqVa7Y4ZGnH3KTdEAcez7esRYY67r",
                "active_stake": 9997717120u64
            }
        ]
    });
    assert_eq!(voter, expected);

    // Test GET /proof/vote_account (compare full JSON)
    let vote_proof: serde_json::Value = client
        .get(format!(
            "{}/proof/vote_account/Mvrzoe3cvKFyY8WqVa7Y4ZGnH3KTdEAcez7esRYY67r?network=testnet&slot=340850340",
            base_url
        ))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let expected_vote_proof = serde_json::json!({
        "network": "testnet",
        "snapshot_slot": slot,
        "meta_merkle_leaf": {
            "voting_wallet": "AECaNinQ6ptWzZcD9WYFimvZuf37kuviUuNGGA4hgWDz",
            "vote_account": "Mvrzoe3cvKFyY8WqVa7Y4ZGnH3KTdEAcez7esRYY67r",
            "stake_merkle_root": "DkSTcvau7xpiZBHHtUSg52utSqEH2qa2NRfBEAAz5fya",
            "active_stake": 32615567722979u64
        },
        "meta_merkle_proof": [
          "ZVvsLpYErGY7dVZ9h5Wpugr5p5EJG31Jkv8NVo3ueYY",
          "obPoamwG5ixNRLisCdFEugYiFAaHqVScTUpLiwoizRt",
          "GJtfCth4kTFbRtgGqTMBUTt6r3RkQwZGpQ4nNj1HZSYF",
          "Fs2fTYw8MYwb4JqrDfpwgfuJ5DQrepEzexX1VQNBgLbk",
          "Fo5LHwsywxsa7yBm3ku9Cqiz3JrSgyzx7z8sRF5rYd2p",
          "xSdU8zuoLHykjN9r1wT5kygjamnWDQhiu4Nqj7feGM6",
          "BvaAe2fzv93BJgtUdEMtmgiuos5CDwv9rKk9Kk3gT4fM"
        ]
    });
    assert_eq!(vote_proof, expected_vote_proof);

    // Test GET /proof/stake_account
    let stake_proof: serde_json::Value = client
        .get(format!(
            "{}/proof/stake_account/Fu12SHuZyaQ4B1or3hFRmx5gqLuGhxTWUjdH98oYRK2N?network=testnet&slot=340850340",
            base_url
        ))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let expected_stake_proof = serde_json::json!({
        "network": "testnet",
        "snapshot_slot": slot,
        "stake_merkle_leaf": {
            "voting_wallet": "AECaNinQ6ptWzZcD9WYFimvZuf37kuviUuNGGA4hgWDz",
            "stake_account": "Fu12SHuZyaQ4B1or3hFRmx5gqLuGhxTWUjdH98oYRK2N",
            "active_stake": 9997717120u64
        },
        "stake_merkle_proof": [
            "2vQkMCm3ibpz8MMinkBPS8kt42TGgm6zdqUzrBG645iU",
            "CfeSpauiU21P7JPmXTvQXfiwFsGRQED551DJsRmeN5f6",
        ],
        "vote_account": "Mvrzoe3cvKFyY8WqVa7Y4ZGnH3KTdEAcez7esRYY67r"
    });
    assert_eq!(stake_proof, expected_stake_proof);

    // Test GET /admin/stats without header → 401
    let stats_no_hdr = client
        .get(format!("{}/admin/stats", base_url))
        .send()
        .await?;
    assert_eq!(stats_no_hdr.status(), StatusCode::UNAUTHORIZED);

    // Test GET /admin/stats with wrong token → 401
    let stats_bad = client
        .get(format!("{}/admin/stats", base_url))
        .header("x-metrics-token", "invalid")
        .send()
        .await?;
    assert_eq!(stats_bad.status(), StatusCode::UNAUTHORIZED);

    // Test GET /admin/stats with correct token → 200
    let stats_ok: serde_json::Value = client
        .get(format!("{}/admin/stats", base_url))
        .header("x-metrics-token", "test-token")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    // uploads should include one success entry with 1
    let uploads = stats_ok.get("upload_total").unwrap().as_array().unwrap();
    assert!(uploads[0].get("outcome").unwrap().as_str() == Some("success"));
    assert!(uploads[0].get("count").unwrap().as_u64() >= Some(1));

    // proofs_not_found_total should be empty
    let not_found = stats_ok
        .get("proofs_not_found_total")
        .unwrap()
        .as_array()
        .unwrap();
    assert!(not_found.is_empty());

    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn e2e_rejects_replayed_signature_and_incoherent_stake_root() -> anyhow::Result<()> {
    let keypair = Keypair::new();
    let (base_url, _guard) = setup_server(&keypair).await?;

    let snapshot_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests/src/fixtures/meta_merkle_340850340.zip");
    let honest_bytes = tokio::fs::read(&snapshot_path).await?;
    let (honest_snapshot, honest_snapshot_hash) =
        cli::MetaMerkleSnapshot::read_from_bytes_with_hash(honest_bytes.clone(), true)?;
    let slot = honest_snapshot.slot;
    let merkle_root = bs58::encode(honest_snapshot.root).into_string();
    let honest_hash = bs58::encode(honest_snapshot_hash.to_bytes()).into_string();
    let honest_signature = sign_upload_message(&keypair, slot, NETWORK, &merkle_root, &honest_hash);
    let client = reqwest::Client::new();

    let wrong_signature =
        sign_upload_message(&Keypair::new(), slot, NETWORK, &merkle_root, &honest_hash);
    let unauthorized_upload = Form::new()
        .text("slot", slot.to_string())
        .text("network", NETWORK)
        .text("merkle_root", merkle_root.clone())
        .text("snapshot_hash", honest_hash.clone())
        .text("signature", wrong_signature)
        .part(
            "file",
            Part::bytes(b"not a snapshot".to_vec()).file_name("invalid_snapshot.zip"),
        );
    let unauthorized_response = client
        .post(format!("{}/upload", base_url))
        .multipart(unauthorized_upload)
        .send()
        .await?;
    assert_eq!(unauthorized_response.status(), StatusCode::UNAUTHORIZED);

    let honest_upload = Form::new()
        .text("slot", slot.to_string())
        .text("network", NETWORK)
        .text("merkle_root", merkle_root.clone())
        .text("snapshot_hash", honest_hash.clone())
        .text("signature", honest_signature.clone())
        .part(
            "file",
            Part::bytes(honest_bytes).file_name("honest_snapshot.zip"),
        );
    let honest_response = client
        .post(format!("{}/upload", base_url))
        .multipart(honest_upload)
        .send()
        .await?;
    assert!(
        honest_response.status().is_success(),
        "honest upload failed with {}",
        honest_response.status()
    );

    let honest_meta: serde_json::Value = client
        .get(format!("{}/meta?network={}", base_url, NETWORK))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(honest_meta["snapshot_hash"], honest_hash);

    let mut tampered_snapshot = honest_snapshot;
    tampered_snapshot.leaf_bundles[0].stake_merkle_leaves[0].active_stake += 1;
    let tampered_bytes = encode_snapshot(&tampered_snapshot)?;
    let (_, tampered_snapshot_hash) =
        cli::MetaMerkleSnapshot::read_from_bytes_with_hash(tampered_bytes.clone(), true)?;
    let tampered_hash = bs58::encode(tampered_snapshot_hash.to_bytes()).into_string();

    let replay_upload = Form::new()
        .text("slot", slot.to_string())
        .text("network", NETWORK)
        .text("merkle_root", merkle_root.clone())
        .text("snapshot_hash", honest_hash)
        .text("signature", honest_signature)
        .part(
            "file",
            Part::bytes(tampered_bytes.clone()).file_name("replayed_snapshot.zip"),
        );
    let replay_response = client
        .post(format!("{}/upload", base_url))
        .multipart(replay_upload)
        .send()
        .await?;
    assert_eq!(replay_response.status(), StatusCode::BAD_REQUEST);

    let tampered_signature =
        sign_upload_message(&keypair, slot, NETWORK, &merkle_root, &tampered_hash);
    let incoherent_upload = Form::new()
        .text("slot", slot.to_string())
        .text("network", NETWORK)
        .text("merkle_root", merkle_root.clone())
        .text("snapshot_hash", tampered_hash)
        .text("signature", tampered_signature)
        .part(
            "file",
            Part::bytes(tampered_bytes).file_name("incoherent_snapshot.zip"),
        );
    let incoherent_response = client
        .post(format!("{}/upload", base_url))
        .multipart(incoherent_upload)
        .send()
        .await?;
    assert_eq!(incoherent_response.status(), StatusCode::BAD_REQUEST);

    let final_meta: serde_json::Value = client
        .get(format!("{}/meta?network={}", base_url, NETWORK))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(final_meta["snapshot_hash"], honest_meta["snapshot_hash"]);

    Ok(())
}

fn encode_snapshot(snapshot: &cli::MetaMerkleSnapshot) -> anyhow::Result<Vec<u8>> {
    Ok(snapshot.to_compressed_bytes()?)
}

/// Reuploading the same `(network, slot)` with a different (subset) snapshot
/// body must fully replace the slot's rows, not merge into a hybrid snapshot.
/// Rows for accounts omitted from the reupload must be cleared so that `/meta`,
/// `/proof/*`, and `/voter/*` all describe the single snapshot identified by the
/// advertised `snapshot_hash`.
#[tokio::test]
#[serial_test::serial]
async fn e2e_same_slot_reupload_fully_replaces_rows() -> anyhow::Result<()> {
    let keypair = Keypair::new();
    let (base_url, _guard) = setup_server(&keypair).await?;
    let client = reqwest::Client::new();

    let snapshot_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests/src/fixtures/meta_merkle_340850340.zip");
    let full_bytes = tokio::fs::read(&snapshot_path).await?;
    let (full_snapshot, full_hash) =
        cli::MetaMerkleSnapshot::read_from_bytes_with_hash(full_bytes.clone(), true)?;

    assert!(
        full_snapshot.leaf_bundles.len() >= 2,
        "fixture must contain at least two bundles"
    );

    let slot = full_snapshot.slot;
    let merkle_root = bs58::encode(full_snapshot.root).into_string();
    let full_hash = bs58::encode(full_hash.to_bytes()).into_string();
    let full_signature = sign_upload_message(&keypair, slot, NETWORK, &merkle_root, &full_hash);

    // First upload: the complete fixture snapshot.
    let full_upload = Form::new()
        .text("slot", slot.to_string())
        .text("network", NETWORK)
        .text("merkle_root", merkle_root.clone())
        .text("snapshot_hash", full_hash.clone())
        .text("signature", full_signature)
        .part("file", Part::bytes(full_bytes).file_name("full_snapshot.zip"));
    let resp = client
        .post(format!("{}/upload", base_url))
        .multipart(full_upload)
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "full upload failed status={}",
        resp.status()
    );

    // Keep (and modify) the first bundle; omit the second on the reupload.
    let mut modified_bundle = full_snapshot.leaf_bundles[0].clone();
    let omitted_bundle = full_snapshot.leaf_bundles[1].clone();
    modified_bundle.meta_merkle_leaf.active_stake = modified_bundle
        .meta_merkle_leaf
        .active_stake
        .saturating_add(1);

    // The omitted account is queryable after the full upload.
    let omitted_vote_account = omitted_bundle.meta_merkle_leaf.vote_account.to_string();
    let omitted_before = client
        .get(format!(
            "{}/proof/vote_account/{}?network={}&slot={}",
            base_url, omitted_vote_account, NETWORK, slot
        ))
        .send()
        .await?;
    assert!(
        omitted_before.status().is_success(),
        "omitted account should be present after the full upload, got status={}",
        omitted_before.status()
    );

    // Second upload: same (network, slot), only the modified bundle. Signed with
    // its own snapshot hash so it passes the byte-binding check.
    let modified_snapshot = cli::MetaMerkleSnapshot {
        root: full_snapshot.root,
        leaf_bundles: vec![modified_bundle.clone()],
        slot,
    };
    let modified_bytes = modified_snapshot.to_compressed_bytes()?;
    let (_, modified_hash) =
        cli::MetaMerkleSnapshot::read_from_bytes_with_hash(modified_bytes.clone(), true)?;
    let modified_hash = bs58::encode(modified_hash.to_bytes()).into_string();
    assert_ne!(
        modified_hash, full_hash,
        "reupload must carry a distinct snapshot hash"
    );
    let modified_signature =
        sign_upload_message(&keypair, slot, NETWORK, &merkle_root, &modified_hash);

    let modified_upload = Form::new()
        .text("slot", slot.to_string())
        .text("network", NETWORK)
        .text("merkle_root", merkle_root.clone())
        .text("snapshot_hash", modified_hash.clone())
        .text("signature", modified_signature)
        .part(
            "file",
            Part::bytes(modified_bytes).file_name("modified_snapshot.zip"),
        );
    let resp = client
        .post(format!("{}/upload", base_url))
        .multipart(modified_upload)
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "modified upload failed status={}",
        resp.status()
    );

    // /meta now advertises the new snapshot hash.
    let meta: serde_json::Value = client
        .get(format!("{}/meta?network={}", base_url, NETWORK))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(meta["snapshot_hash"], modified_hash);

    // The modified account reflects the reuploaded data.
    let modified_vote_account = modified_bundle.meta_merkle_leaf.vote_account.to_string();
    let modified_proof: serde_json::Value = client
        .get(format!(
            "{}/proof/vote_account/{}?network={}&slot={}",
            base_url, modified_vote_account, NETWORK, slot
        ))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        modified_proof["meta_merkle_leaf"]["active_stake"].as_u64(),
        Some(modified_bundle.meta_merkle_leaf.active_stake)
    );

    // The omitted account is no longer queryable for this slot: the reupload
    // fully replaced the slot instead of leaving a hybrid snapshot behind.
    let omitted_after = client
        .get(format!(
            "{}/proof/vote_account/{}?network={}&slot={}",
            base_url, omitted_vote_account, NETWORK, slot
        ))
        .send()
        .await?;
    assert_eq!(
        omitted_after.status(),
        StatusCode::NOT_FOUND,
        "omitted vote account must be cleared on reupload"
    );

    // A stake row under the omitted bundle must be cleared too. Check a single
    // representative account (one not also present in the retained bundle) to
    // stay within the request rate limit.
    let retained_stake_accounts: std::collections::HashSet<String> = modified_bundle
        .stake_merkle_leaves
        .iter()
        .map(|leaf| leaf.stake_account.to_string())
        .collect();
    if let Some(stake_account) = omitted_bundle
        .stake_merkle_leaves
        .iter()
        .map(|leaf| leaf.stake_account.to_string())
        .find(|account| !retained_stake_accounts.contains(account))
    {
        let stake_after = client
            .get(format!(
                "{}/proof/stake_account/{}?network={}&slot={}",
                base_url, stake_account, NETWORK, slot
            ))
            .send()
            .await?;
        assert_eq!(
            stake_after.status(),
            StatusCode::NOT_FOUND,
            "omitted stake account {} must be cleared on reupload",
            stake_account
        );
    }

    Ok(())
}
