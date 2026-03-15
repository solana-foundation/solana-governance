mod common;
use common::setup_server;

use reqwest::{
    multipart::{Form, Part},
    StatusCode,
};
use solana_sdk::{signature::Keypair, signer::Signer};

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

    // Build signature over slot || merkle_root
    let mut message = Vec::new();
    message.extend_from_slice(&slot.to_le_bytes());
    message.extend_from_slice(merkle_root.as_bytes());
    let signature = keypair.sign_message(&message).to_string();

    // Test GET /healthz
    let client = reqwest::Client::new();
    let health = client.get(format!("{}/healthz", base_url)).send().await?;
    assert!(health.status().is_success());

    // Test POST /upload
    let form = Form::new()
        .text("slot", slot.to_string())
        .text("network", "testnet")
        .text("merkle_root", merkle_root.clone())
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
        "snapshot_hash": bs58::encode(snapshot_hash.to_bytes()).into_string(),
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
