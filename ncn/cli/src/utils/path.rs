use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;

/// Validates that a path points to a regular, executable file (not a symlink).
pub fn validate_executable_path(path: &Path) -> Result<()> {
    let path_str = path.to_string_lossy();

    if path_str.starts_with('-') {
        return Err(anyhow!(
            "Executable path must not start with '-': {}",
            path_str
        ));
    }

    let canonical = fs::canonicalize(path)
        .map_err(|e| anyhow!("Failed to resolve executable path '{}': {}", path_str, e))?;

    let metadata = fs::metadata(&canonical)
        .map_err(|e| anyhow!("Failed to read metadata for '{}': {}", canonical.display(), e))?;

    if !metadata.is_file() {
        return Err(anyhow!(
            "Executable path is not a regular file: {}",
            canonical.display()
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(anyhow!(
                "File is not executable: {}",
                canonical.display()
            ));
        }
    }

    Ok(())
}

/// Validates that a path points to an existing directory (not a symlink to one).
pub fn validate_directory_path(path: &Path) -> Result<()> {
    let path_str = path.to_string_lossy();

    if path_str.starts_with('-') {
        return Err(anyhow!(
            "Directory path must not start with '-': {}",
            path_str
        ));
    }

    let canonical = fs::canonicalize(path)
        .map_err(|e| anyhow!("Failed to resolve directory path '{}': {}", path_str, e))?;

    let metadata = fs::metadata(&canonical)
        .map_err(|e| anyhow!("Failed to read metadata for '{}': {}", canonical.display(), e))?;

    if !metadata.is_dir() {
        return Err(anyhow!(
            "Path is not a directory: {}",
            canonical.display()
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::path::Path;
    use tempfile::TempDir;

    fn setup() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    // --- validate_executable_path ---

    #[test]
    fn executable_rejects_dash_prefix() {
        let result = validate_executable_path(Path::new("-malicious"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must not start with '-'"));
    }

    #[test]
    fn executable_rejects_nonexistent() {
        let result = validate_executable_path(Path::new("/no/such/binary"));
        assert!(result.is_err());
    }

    #[test]
    fn executable_rejects_directory() {
        let dir = setup();
        let result = validate_executable_path(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a regular file"));
    }

    #[test]
    fn executable_rejects_non_executable_file() {
        let dir = setup();
        let file_path = dir.path().join("not_exec");
        File::create(&file_path).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&file_path, fs::Permissions::from_mode(0o644)).unwrap();
        }

        let result = validate_executable_path(&file_path);
        assert!(result.is_err());
    }

    #[test]
    fn executable_accepts_valid() {
        let dir = setup();
        let file_path = dir.path().join("good_bin");
        File::create(&file_path).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&file_path, fs::Permissions::from_mode(0o755)).unwrap();
        }

        assert!(validate_executable_path(&file_path).is_ok());
    }

    // --- validate_directory_path ---

    #[test]
    fn directory_rejects_dash_prefix() {
        let result = validate_directory_path(Path::new("-bad"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must not start with '-'"));
    }

    #[test]
    fn directory_rejects_nonexistent() {
        let result = validate_directory_path(Path::new("/no/such/dir"));
        assert!(result.is_err());
    }

    #[test]
    fn directory_rejects_file() {
        let dir = setup();
        let file_path = dir.path().join("a_file");
        File::create(&file_path).unwrap();

        let result = validate_directory_path(&file_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a directory"));
    }

    #[test]
    fn directory_accepts_valid() {
        let dir = setup();
        assert!(validate_directory_path(dir.path()).is_ok());
    }
}
