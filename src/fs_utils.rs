use crate::error::SkitError;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

/// Securely create and write a secret file.
/// - Fails if the file already exists.
/// - Creates the file with 0o600 permissions on Unix.
/// - Refuses to operate on symlinks.
pub fn write_secret_file_secure(path: &Path, contents: &str) -> Result<(), SkitError> {
    // Ensure parent directory exists and is not a symlink
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(SkitError::Io)?;
        let meta = fs::symlink_metadata(parent).map_err(SkitError::Io)?;
        if !meta.is_dir() {
            return Err(SkitError::Io(std::io::Error::other(format!(
                "Parent is not a directory: {}",
                parent.display()
            ))));
        }
        if meta.file_type().is_symlink() {
            return Err(SkitError::Io(std::io::Error::other(format!(
                "Refusing to write through symlinked directory: {}",
                parent.display()
            ))));
        }
    }

    // Refuse if target exists (also mitigates symlink attacks)
    if path.exists() {
        return Err(SkitError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("Refusing to overwrite existing file: {}", path.display()),
        )));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(path)
            .map_err(SkitError::Io)?;
        file.write_all(contents.as_bytes()).map_err(SkitError::Io)?;
        file.flush().map_err(SkitError::Io)?;
        Ok(())
    }

    #[cfg(not(unix))]
    {
        // Note: On Windows, secure file permissions (mode 0o600) are not set
        // The file will be created with default Windows ACLs
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .map_err(SkitError::Io)?;
        file.write_all(contents.as_bytes()).map_err(SkitError::Io)?;
        file.flush().map_err(SkitError::Io)?;
        Ok(())
    }
}
