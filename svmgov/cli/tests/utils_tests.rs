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
