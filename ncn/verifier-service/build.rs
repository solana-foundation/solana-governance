fn main() {
    // Re-run if HEAD or this file changes
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");

    // Try to get git hash
    let git_hash = std::process::Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .and_then(|o| if o.status.success() { Some(String::from_utf8_lossy(&o.stdout).trim().to_string()) } else { None });

    if let Some(h) = git_hash {
        println!("cargo:rustc-env=VERIFIER_BUILD_GIT_HASH={}", h);
    }

    // Build time as unix seconds, allow override via SOURCE_DATE_EPOCH for reproducible builds
    let now = std::env::var("SOURCE_DATE_EPOCH").ok().or_else(|| {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
        Some(ts.to_string())
    }).unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=VERIFIER_BUILD_TIME_UNIX={}", now);
}
