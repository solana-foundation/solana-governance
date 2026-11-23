use std::{process::Command, str::FromStr};

use anchor_client::solana_sdk::{
    bs58,
    signature::{write_keypair_file, Keypair, Signature, Signer},
};
use cli::MetaMerkleSnapshot;
use serde_json::Value;

#[test]
fn snapshot_hash_json_is_one_document_with_a_valid_upload_signature() {
    let temp = tempfile::tempdir().unwrap();
    let authority = Keypair::new();
    let keypair_path = temp.path().join("authority.json");
    write_keypair_file(&authority, &keypair_path).unwrap();
    let snapshot_path = temp.path().join("snapshot.zip");
    MetaMerkleSnapshot {
        root: [1; 32],
        leaf_bundles: vec![],
        slot: 100,
    }
    .save_compressed(snapshot_path.clone())
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_ncn-cli"))
        .arg("--authority-path")
        .arg(&keypair_path)
        .args(["log-meta-merkle-hash", "--read-path"])
        .arg(&snapshot_path)
        .args(["--network", "testnet", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let root = bs58::encode([1; 32]).into_string();
    let hash = MetaMerkleSnapshot::snapshot_hash(snapshot_path, true)
        .unwrap()
        .to_string();
    assert_eq!(value["slot"], 100);
    assert_eq!(value["network"], "testnet");
    assert_eq!(value["meta_merkle_root"], root);
    assert_eq!(value["snapshot_hash"], hash);
    let signature = Signature::from_str(value["signature"].as_str().unwrap()).unwrap();
    assert!(signature.verify(
        authority.pubkey().as_ref(),
        &cli::upload_signature_message(100, "testnet", &root, &hash)
    ));
}
