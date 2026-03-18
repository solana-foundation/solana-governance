use anyhow::{bail, Result};
use std::{fs, path::Path};

pub fn validate_db_path(db_path: &str) -> Result<()> {
    if db_path == ":memory:" {
        return Ok(());
    }

    if db_path.is_empty() {
        bail!("Empty database path");
    }

    if db_path.contains('\0') || db_path.contains(['\n', '\r', '\t']) {
        bail!("Invalid control characters in database path");
    }

    let path = Path::new(db_path);

    // Disallow parent directory traversal components
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            bail!("Parent directory traversal is not allowed in database path");
        }
    }

    // Require a terminal file name (avoid paths ending with a directory separator)
    if path.file_name().is_none() {
        bail!("Database path must include a file name");
    }

    // If an entry already exists at the path, reject symlinks and directories
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            bail!("Symlink path is not allowed for database path");
        }
        if meta.is_dir() {
            bail!("Database path points to a directory");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_db_path;
    use rand::{distributions::Alphanumeric, Rng};
    use std::{env, fs, path::PathBuf};

    fn random_name(prefix: &str) -> String {
        let suffix: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();
        format!("{}{}", prefix, suffix)
    }

    #[test]
    fn allows_memory() {
        assert!(validate_db_path(":memory:").is_ok());
    }

    #[test]
    fn allows_normal_relative_file() {
        assert!(validate_db_path("data/app.db").is_ok());
    }

    #[test]
    fn rejects_empty() {
        assert!(validate_db_path("").is_err());
    }

    #[test]
    fn rejects_control_chars() {
        assert!(validate_db_path("bad\nname.db").is_err());
        assert!(validate_db_path("bad\tname.db").is_err());
        assert!(validate_db_path("bad\0name.db").is_err());
    }

    #[test]
    fn rejects_parent_traversal() {
        assert!(validate_db_path("../bad.db").is_err());
        assert!(validate_db_path("dir/../bad.db").is_err());
    }

    #[test]
    fn rejects_directory_path() {
        let tmp = env::temp_dir();
        let dir_name = random_name("vs_dir_");
        let dir_path = tmp.join(dir_name);
        fs::create_dir_all(&dir_path).unwrap();
        // Passing a path to an existing directory should be rejected
        assert!(validate_db_path(dir_path.to_str().unwrap()).is_err());
        fs::remove_dir_all(&dir_path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_path() {
        use std::os::unix::fs::symlink;

        let tmp = env::temp_dir();
        let base = tmp.join(random_name("vs_sym_"));
        fs::create_dir_all(&base).unwrap();

        let target = base.join("target.db");
        fs::write(&target, b"test").unwrap();

        let link = base.join("link.db");
        symlink(&target, &link).unwrap();

        assert!(validate_db_path(link.to_str().unwrap()).is_err());

        // cleanup
        fs::remove_file(&link).ok();
        fs::remove_file(&target).ok();
        fs::remove_dir_all(&base).ok();
    }
}


