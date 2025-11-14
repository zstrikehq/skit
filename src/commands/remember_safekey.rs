use crate::display::{print_info, print_success};
use crate::error::SkitError;
use crate::password::try_get_password_from_env;
use crate::types::Safe;
use std::fs;
use std::path::PathBuf;

pub fn remember_safekey(safe_path: &str) -> Result<(), SkitError> {
    // Load the safe to get the UUID
    let safe = Safe::load(safe_path)?;

    // Try to get password from environment first, otherwise prompt
    let password = match try_get_password_from_env(safe_path) {
        Some(pass) => {
            print_info("🌍 Using safe key from environment");
            pass
        }
        None => {
            println!("Enter the password for this safe to verify and save it:");
            crate::input::prompt_password_with_fallback("Password: ").map_err(SkitError::Io)?
        }
    };

    // Verify the password is correct
    if safe.verify_password(&password).is_err() {
        return Err(SkitError::InvalidPassword(
            "Invalid password provided".to_string(),
        ));
    }

    remember_safekey_with_password(&safe, &password).map(|_| ())
}

/// Save a safe key with a known password (used internally when we already have the password)
pub fn remember_safekey_with_password(safe: &Safe, password: &str) -> Result<(), SkitError> {
    remember_safekey_with_password_quiet(safe, password, false).map(|_| ())
}

/// Save a safe key with a known password, with optional quiet mode
pub fn remember_safekey_with_password_quiet(
    safe: &Safe,
    password: &str,
    quiet: bool,
) -> Result<String, SkitError> {
    // Verify the password is correct
    if safe.verify_password(password).is_err() {
        return Err(SkitError::InvalidPassword(
            "Invalid password provided".to_string(),
        ));
    }

    // Create the ~/.config/skit/keys directory
    let home_dir = dirs::home_dir().ok_or_else(|| {
        SkitError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find home directory",
        ))
    })?;

    let skit_keys_dir: PathBuf = home_dir.join(".config").join("skit").join("keys");
    fs::create_dir_all(&skit_keys_dir).map_err(SkitError::Io)?;

    // Save the password to ~/.config/skit/keys/<uuid>.key securely
    let key_file = skit_keys_dir.join(format!("{}.key", safe.uuid));
    crate::fs_utils::write_secret_file_secure(&key_file, password)?;

    if !quiet {
        print_success(&format!("Password saved to {}", key_file.display()));
        print_info(&format!("Safe UUID: {}", safe.uuid));
    }

    Ok(key_file.display().to_string())
}
