use crate::error::SkitError;
use crate::types::Safe;
use std::fs;
use std::path::Path;
use zeroize::Zeroizing;

pub fn validate_password_strength(password: &str) -> Result<(), SkitError> {
    if password.len() < 12 {
        return Err(SkitError::ParseError(
            "Password must be at least 12 characters long".to_string(),
        ));
    }

    let allowed_chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._@#-";
    let has_invalid_chars = password.chars().any(|c| !allowed_chars.contains(c));

    if has_invalid_chars {
        return Err(SkitError::ParseError(
            "Password contains invalid characters. Use only: a-z A-Z 0-9 . _ @ # -".to_string(),
        ));
    }

    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| "._@#-".contains(c));

    if !has_lower {
        return Err(SkitError::ParseError(
            "Password must contain at least one lowercase letter".to_string(),
        ));
    }

    if !has_upper {
        return Err(SkitError::ParseError(
            "Password must contain at least one uppercase letter".to_string(),
        ));
    }

    if !has_digit {
        return Err(SkitError::ParseError(
            "Password must contain at least one digit".to_string(),
        ));
    }

    if !has_special {
        return Err(SkitError::ParseError(
            "Password must contain at least one special character (^ - _ . * + = : ,)".to_string(),
        ));
    }

    Ok(())
}

use rand::seq::SliceRandom;

pub fn generate_secure_password() -> String {
    let mut rng = rand::thread_rng();

    let lowercase = "abcdefghijklmnopqrstuvwxyz".chars().collect::<Vec<char>>();
    let uppercase = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().collect::<Vec<char>>();
    let digits = "0123456789".chars().collect::<Vec<char>>();
    let special = "._@#-".chars().collect::<Vec<char>>();

    let mut password = vec![
        *lowercase
            .choose(&mut rng)
            .expect("lowercase charset not empty"),
        *uppercase
            .choose(&mut rng)
            .expect("uppercase charset not empty"),
        *digits.choose(&mut rng).expect("digits charset not empty"),
        *special.choose(&mut rng).expect("special charset not empty"),
    ];

    let all_chars: Vec<char> = lowercase
        .iter()
        .chain(&uppercase)
        .chain(&digits)
        .chain(&special)
        .copied()
        .collect();

    while password.len() < 12 {
        match all_chars.choose(&mut rng) {
            Some(ch) => password.push(*ch),
            None => {
                eprintln!("Warning: Character set unexpectedly empty during password generation");
                break;
            }
        }
    }

    password.shuffle(&mut rng);

    password.into_iter().collect()
}

pub fn get_env_var_name_for_safe(_safe_path: &str) -> String {
    // Always use SKIT_SAFEKEY environment variable
    // The safe file is determined by the -s/--safe parameter
    "SKIT_SAFEKEY".to_string()
}

pub fn try_get_password_from_env(safe_path: &str) -> Option<String> {
    let env_var_name = get_env_var_name_for_safe(safe_path);
    std::env::var(&env_var_name).ok().filter(|p| !p.is_empty())
}

/// Touch a key file to update its modification time for cleanup tracking
fn touch_key_file(key_file: &Path) -> Result<(), SkitError> {
    use filetime::{FileTime, set_file_mtime};
    use std::time::SystemTime;

    // Set the modification time to now without changing file content
    let now = SystemTime::now();
    let filetime = FileTime::from_system_time(now);
    set_file_mtime(key_file, filetime).map_err(|e| {
        SkitError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to touch key file {}: {}", key_file.display(), e),
        ))
    })
}

pub fn try_get_password_from_keyfile(safe: &Safe) -> Result<Option<String>, SkitError> {
    let home_dir = match dirs::home_dir() {
        Some(dir) => dir,
        None => return Ok(None), // No home directory, skip key file lookup
    };

    let key_file = home_dir
        .join(".config")
        .join("skit")
        .join("keys")
        .join(format!("{}.key", safe.uuid));

    if !key_file.exists() {
        return Ok(None);
    }

    let password = Zeroizing::new(
        fs::read_to_string(&key_file)
            .map_err(|e| {
                SkitError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to read key file {}: {}", key_file.display(), e),
                ))
            })?
            .trim()
            .to_string(),
    );

    touch_key_file(&key_file)?;

    match safe.verify_password(&password) {
        Ok(()) => Ok(Some(password.to_string())),
        Err(_) => Err(SkitError::InvalidPassword(format!(
            "Password in key file {} is invalid",
            key_file.display()
        ))),
    }
}

pub fn get_password_with_auth_chain(
    safe: &Safe,
    safe_path: &str,
    prompt_message: &str,
) -> Result<String, SkitError> {
    get_password_with_auth_chain_formatted(safe, safe_path, prompt_message, None)
}

pub fn get_password_with_auth_chain_formatted(
    safe: &Safe,
    safe_path: &str,
    prompt_message: &str,
    format: Option<&crate::OutputFormat>,
) -> Result<String, SkitError> {
    let suppress_info = matches!(
        format,
        Some(crate::OutputFormat::Json)
            | Some(crate::OutputFormat::Env)
            | Some(crate::OutputFormat::Terraform)
            | Some(crate::OutputFormat::Postman)
    );

    let env_var_name = get_env_var_name_for_safe(safe_path);
    if let Ok(password_raw) = std::env::var(&env_var_name)
        && !password_raw.is_empty()
    {
        let password = Zeroizing::new(password_raw);
        match safe.verify_password(&password) {
            Ok(()) => {
                if !suppress_info {
                    tracing::info!("🌍 Using safe key from environment");
                }
                return Ok(password.to_string());
            }
            Err(_) => {
                return Err(SkitError::InvalidPassword(format!(
                    "Invalid password from environment variable {}",
                    env_var_name
                )));
            }
        }
    }

    if let Some(password) = try_get_password_from_keyfile(safe)? {
        if !suppress_info {
            tracing::info!("🔐 Using saved safe key");
        }
        return Ok(password);
    }

    // Finally, fall back to prompting with visual feedback
    let password =
        crate::input::prompt_password_with_fallback(prompt_message).map_err(SkitError::Io)?;
    println!(); // Add line break after password prompt

    match safe.verify_password(&password) {
        Ok(()) => Ok(password),
        Err(_) => Err(SkitError::InvalidPassword(
            "Invalid password from interactive prompt".to_string(),
        )),
    }
}
