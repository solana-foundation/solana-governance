use reqwest::Client;
use solana_sdk::{signature::Keypair, signer::Signer};
use std::process::{Command, Stdio};
use std::{
    net::TcpListener,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::time::sleep;

/// Get an available ephemeral port on localhost.
pub fn find_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

/// Resolve the verifier-service binary path from env or common target dirs.
pub fn resolve_binary_path() -> String {
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_verifier-service") {
        return p;
    }
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_verifier_service") {
        return p;
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest.parent().unwrap_or(&manifest).to_path_buf();
    let candidates = [
        manifest.join("target/debug/verifier-service"),
        manifest.join("target/release/verifier-service"),
        workspace_root.join("target/debug/verifier-service"),
        workspace_root.join("target/release/verifier-service"),
    ];
    for cand in candidates.iter() {
        if Path::new(&cand).exists() {
            return cand.to_string_lossy().to_string();
        }
    }

    "verifier-service".to_string()
}

/// Poll /healthz until the server responds OK or timeout.
pub async fn wait_ready(base: &str, timeout_ms: u64) -> anyhow::Result<()> {
    let client = Client::new();
    let mut waited = 0u64;
    loop {
        if waited >= timeout_ms {
            anyhow::bail!("server not ready after {}ms", timeout_ms);
        }
        if let Ok(resp) = client.get(format!("{}/healthz", base)).send().await {
            if resp.status().is_success() {
                return Ok(());
            }
        }
        sleep(Duration::from_millis(50)).await;
        waited += 50;
    }
}

// Struct that ensures the child process is killed on drop
pub struct ChildGuard(std::process::Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

pub async fn setup_server(keypair: &Keypair) -> anyhow::Result<(String, ChildGuard)> {
    // Resolve binary path from Cargo or fallbacks
    let bin = resolve_binary_path();
    let bin_path = Path::new(&bin);
    assert!(bin_path.exists(), "binary not found at {}", bin);

    // Test config
    let port = find_free_port();
    let base_url = format!("http://127.0.0.1:{}", port);

    // Signing key and operator env
    let operator_pubkey = keypair.pubkey().to_string();

    // Start the binary
    let child = Command::new(&bin)
        .env("OPERATOR_PUBKEY", &operator_pubkey)
        .env("METRICS_AUTH_TOKEN", "test-token")
        .env("DB_PATH", ":memory:")
        .env("PORT", port.to_string())
        .env("RUST_LOG", "info")
        .env("UPLOAD_BODY_LIMIT", (512 * 1024 * 1024).to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Ensure we always try to kill the child on exit
    let guard = ChildGuard(child);

    // Wait until server is ready
    wait_ready(&base_url, 10_000).await?;

    Ok((base_url, guard))
}
