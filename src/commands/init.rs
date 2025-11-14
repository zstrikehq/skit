use crate::error::SkitError;
use crate::password::{generate_secure_password, validate_password_strength};
use crate::types::Safe;
use std::fs;
use std::io::{self, Write};

pub fn init(
    safe_path: &str,
    remember: bool,
    description: Option<&str>,
    ssm_prefix: Option<&str>,
) -> Result<(), SkitError> {
    if fs::metadata(safe_path).is_ok() {
        tracing::info!("Safe already exists at {}", safe_path);
        return Ok(());
    }

    println!("Creating new safe.");
    println!("\nPassword requirements for new safe:");
    println!("  - At least 12 characters");
    println!("  - At least one uppercase letter");
    println!("  - At least one lowercase letter");
    println!("  - At least one digit");
    println!("  - At least one special character. Allowed special characters: . _ @ # -");

    let password = loop {
        let password = crate::input::prompt_password_with_fallback(
            "Enter password for the safe (or hit enter to generate one automatically): ",
        )
        .map_err(SkitError::Io)?;

        if password.is_empty() {
            let gen_password = generate_secure_password();
            println!("Generated password (keep this safe!): {}", gen_password);
            break gen_password;
        } else {
            match validate_password_strength(&password) {
                Ok(()) => {
                    let confirm = crate::input::prompt_password_with_fallback("Confirm password: ")
                        .map_err(SkitError::Io)?;

                    if password == confirm {
                        println!();
                        break password;
                    } else {
                        eprintln!("Error: Passwords do not match. Please try again.");
                        continue;
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    continue;
                }
            }
        }
    };

    let description = if let Some(desc) = description {
        desc.to_string()
    } else {
        print!("\nEnter a description for this safe (optional):");
        let _ = io::stdout().flush();

        let mut input_description = String::new();
        io::stdin()
            .read_line(&mut input_description)
            .map_err(SkitError::Io)?;
        let input_description = input_description.trim();

        if input_description.is_empty() {
            "Default safe".to_string()
        } else {
            input_description.to_string()
        }
    };

    let mut safe = Safe::new_with_password(&password, &description)?;

    if let Some(prefix) = ssm_prefix {
        let normalized_prefix = prefix.trim();
        if normalized_prefix.is_empty() {
            return Err(SkitError::ParseError(
                "SSM prefix cannot be empty when provided".to_string(),
            ));
        }

        if !normalized_prefix.starts_with('/') {
            tracing::warn!(
                "SSM prefix '{}' does not start with '/'. AWS SSM parameters typically start with '/'",
                normalized_prefix
            );
        }

        safe.ssm_prefix = Some(normalized_prefix.to_string());
        println!(
            "Associated this safe with default SSM prefix: {}",
            normalized_prefix
        );
    }
    safe.save(safe_path)?;
    tracing::info!("✓ Created new safe at {}", safe_path);

    let should_save = if remember {
        true
    } else {
        print!("\nWould you like to save the safe key for automatic authentication? (y/N):");
        let _ = io::stdout().flush();

        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(SkitError::Io)?;

        let input = input.trim().to_lowercase();
        input == "y" || input == "yes"
    };

    if should_save {
        let home_dir = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("~"));
        let key_file = home_dir
            .join(".config")
            .join("skit")
            .join("keys")
            .join(format!("{}.key", safe.uuid));
        save_safe_key(&safe, &password)?;
        tracing::info!(
            "✓ Safe key saved for automatic authentication at {}",
            key_file.display()
        );
    } else {
        tracing::info!(
            "💡 Tip: Use 'skit remember-safekey' to save your safe key securely for easy access",
        );
    }

    Ok(())
}

fn save_safe_key(safe: &Safe, password: &str) -> Result<(), SkitError> {
    let home_dir = dirs::home_dir().ok_or_else(|| {
        SkitError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find home directory",
        ))
    })?;

    let skit_keys_dir = home_dir.join(".config").join("skit").join("keys");
    fs::create_dir_all(&skit_keys_dir).map_err(SkitError::Io)?;

    let key_file = skit_keys_dir.join(format!("{}.key", safe.uuid));
    crate::fs_utils::write_secret_file_secure(&key_file, password)?;

    Ok(())
}
