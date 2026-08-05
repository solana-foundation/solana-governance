use std::process::Command;

// Basic integration test to verify the CLI can run
#[test]
fn test_cli_runs_without_error() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run CLI");

    assert!(output.status.success(), "CLI should run successfully");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("svmgov"), "CLI should show program name");
}

// Test that CLI shows help information
#[test]
fn test_cli_help_contains_expected_commands() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run CLI");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("create-proposal"),
        "Help should show create-proposal command"
    );
    assert!(
        stdout.contains("cast-vote"),
        "Help should show cast-vote command"
    );
    assert!(
        stdout.contains("support-proposal"),
        "Help should show support-proposal command"
    );
}

// A pull request link must be rejected before the CLI touches a keypair or an RPC endpoint,
// which is what makes this test hermetic.
#[test]
fn test_create_proposal_rejects_pull_request_link() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "create-proposal",
            "--title",
            "Test proposal",
            "--description",
            "https://github.com/solana-foundation/solana-governance-proposals/pull/3",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run CLI");

    assert!(
        !output.status.success(),
        "CLI should reject a pull request link"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("pull request"),
        "error should name the problem, got: {stderr}"
    );
    assert!(
        stderr.contains("Files changed"),
        "error should explain how to get the right link, got: {stderr}"
    );
}

// Test CLI error handling for missing arguments
#[test]
fn test_cli_error_on_invalid_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "invalid-command"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run CLI");

    // Should fail with an error
    assert!(
        !output.status.success(),
        "CLI should fail on invalid command"
    );
}
