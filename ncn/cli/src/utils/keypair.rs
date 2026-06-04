use anchor_client::solana_sdk::signature::{read_keypair_file, Keypair};
use anyhow::{anyhow, Result};
use std::path::Path;

/// Read a keypair file used to sign on-chain instructions or off-chain
/// messages, surfacing actionable errors for the most common
/// misconfigurations before delegating to `read_keypair_file`.
///
/// Three preflight checks are performed in order:
///   1. The path is not the clap default placeholder `"/"`, which signals the
///      flag was never overridden by the user.
///   2. The path does not point at a directory (a common copy/paste mistake
///      that otherwise surfaces as a cryptic JSON parse error from
///      `read_keypair_file`).
///   3. `read_keypair_file` succeeds; if it does not, its message is wrapped
///      with the offending path and the next step the user should take.
///
/// `flag_name` should be the CLI flag the caller obtained the path from
/// (e.g. `"--payer-path"`) and is included in remediation hints so the user
/// knows which flag to set. The *role* of the keypair (payer vs authority) is
/// expected to be added by the caller via `anyhow::Context::with_context`,
/// keeping this helper agnostic to how the resulting key will be used.
pub fn read_signer_keypair(path: &Path, flag_name: &str) -> Result<Keypair> {
    // (1) The clap default; the CLI uses "/" as a placeholder so the args
    // remain optional at the parser level but still produce a clear error
    // when a subcommand actually needs the file.
    if path == Path::new("/") {
        return Err(anyhow!(
            "no keypair file specified (got the default placeholder `/`); \
             pass `{flag}` (or set the matching env var) pointing at a JSON \
             keypair file",
            flag = flag_name,
        ));
    }

    // (2) Detect directories explicitly; otherwise `read_keypair_file` emits
    // a confusing JSON-parse error several layers deep.
    if path.is_dir() {
        return Err(anyhow!(
            "keypair path `{path}` (from `{flag}`) is a directory; expected a \
             JSON keypair file",
            path = path.display(),
            flag = flag_name,
        ));
    }

    // (3) Defer to the SDK and wrap its error with the path and remediation.
    read_keypair_file(path).map_err(|err| {
        anyhow!(
            "failed to read keypair at `{path}` (from `{flag}`): {err}. \
             Verify the file exists, is readable, and is a JSON keypair file",
            path = path.display(),
            flag = flag_name,
            err = err,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn setup() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn rejects_default_placeholder() {
        let err = read_signer_keypair(Path::new("/"), "--payer-path").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("default placeholder"), "got: {msg}");
        assert!(msg.contains("--payer-path"), "got: {msg}");
    }

    #[test]
    fn rejects_directory() {
        let dir = setup();
        let err = read_signer_keypair(dir.path(), "--authority-path").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("is a directory"), "got: {msg}");
        assert!(msg.contains("--authority-path"), "got: {msg}");
    }

    #[test]
    fn rejects_missing_file() {
        let dir = setup();
        let missing = dir.path().join("does-not-exist.json");
        let err = read_signer_keypair(&missing, "--authority-path").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("failed to read keypair"), "got: {msg}");
        assert!(msg.contains("--authority-path"), "got: {msg}");
    }

    #[test]
    fn rejects_non_keypair_file() {
        let dir = setup();
        let path = dir.path().join("not-a-keypair.json");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "this is not a keypair").unwrap();

        let err = read_signer_keypair(&path, "--payer-path").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("failed to read keypair"), "got: {msg}");
        assert!(msg.contains("--payer-path"), "got: {msg}");
    }

    #[test]
    fn accepts_valid_keypair_file() {
        use anchor_client::solana_sdk::signature::write_keypair_file;

        let dir = setup();
        let path = dir.path().join("kp.json");
        let kp = Keypair::new();
        write_keypair_file(&kp, &path).unwrap();

        let loaded = read_signer_keypair(&path, "--authority-path").unwrap();
        assert_eq!(loaded.to_bytes(), kp.to_bytes());
    }
}
