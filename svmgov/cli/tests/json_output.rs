use std::process::Command;

use serde_json::Value;

#[test]
fn config_json_stdout_is_one_document_with_effective_defaults() {
    let home = tempfile::tempdir().unwrap();
    for (args, expected) in [
        (vec!["config", "show", "--json"], "network"),
        (vec!["config", "get", "rpc-url", "--json"], "rpc-url"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_svmgov"))
            .args(args)
            .env("HOME", home.path())
            .env_remove("SVMGOV_KEY")
            .env_remove("SVMGOV_RPC")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(value[expected].is_string());
        if expected == "network" {
            assert_eq!(value["network"], "mainnet");
            assert!(value["rpc-url"].as_str().unwrap().starts_with("https://"));
            assert!(value["identity-keypair"].is_null());
            assert!(value["program-id"].is_string());
        }
    }
}
