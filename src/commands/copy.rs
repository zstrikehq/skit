use crate::crypto;
use crate::error::SkitError;
use crate::input;
use crate::password::{
    generate_secure_password, get_password_with_auth_chain, validate_password_strength,
};
use crate::types::Safe;
use std::fs;
use std::io::{self, Write};

pub fn copy(
    source_path: &str,
    dest_path: &str,
    remember: bool,
    description: Option<&str>,
) -> Result<(), SkitError> {
    // Check if destination already exists
    if fs::metadata(dest_path).is_ok() {
        return Err(SkitError::ParseError(format!(
            "Destination safe already exists at {}",
            dest_path
        )));
    }

    // Load the source safe
    let source_safe = Safe::load(source_path)?;

    // Get source password to decrypt secrets
    let source_password = get_password_with_auth_chain(
        &source_safe,
        source_path,
        "Enter password for source safe: ",
    )?;
    source_safe.verify_password(&source_password)?;

    println!("\n📋 Copying safe from {} to {}", source_path, dest_path);
    println!("\nPassword requirements for new safe:");
    println!("  - At least 12 characters");
    println!("  - At least one uppercase letter");
    println!("  - At least one lowercase letter");
    println!("  - At least one digit");
    println!("  - At least one special character. Allowed special characters: . _ @ # -");

    // Get new password for destination safe
    let dest_password = loop {
        let password = input::prompt_password_with_fallback(
            "Enter password for the new safe (or hit enter to generate one automatically): ",
        )
        .map_err(SkitError::Io)?;

        if password.is_empty() {
            let gen_password = generate_secure_password();
            println!("Generated password (keep this safe!): {}", gen_password);
            break gen_password;
        } else {
            match validate_password_strength(&password) {
                Ok(()) => {
                    let confirm = input::prompt_password_with_fallback("Confirm password: ")
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

    // Get description for new safe
    let new_description = if let Some(desc) = description {
        desc.to_string()
    } else {
        print!(
            "\nEnter a description for the new safe (optional, press enter to use source description):"
        );
        let _ = io::stdout().flush();

        let mut input_description = String::new();
        io::stdin()
            .read_line(&mut input_description)
            .map_err(SkitError::Io)?;
        let input_description = input_description.trim();

        if input_description.is_empty() {
            source_safe.description.clone()
        } else {
            input_description.to_string()
        }
    };

    // Create new safe with new password and UUID
    let mut dest_safe = Safe::new_with_password(&dest_password, &new_description)?;

    // Copy and re-encrypt all items
    let mut copied_encrypted = 0;
    let mut copied_plain = 0;

    for (key, item) in &source_safe.items {
        if item.is_encrypted {
            // Decrypt with source password and re-encrypt with destination password
            let decrypted_value = crypto::DecryptBuilder::new()
                .password(&source_password)
                .ciphertext(&item.value)
                .decrypt()?;
            let encrypted_value = crypto::EncryptBuilder::new()
                .password(&dest_password)
                .plaintext(&decrypted_value)
                .encrypt()?;
            dest_safe.add_or_update_item(key.clone(), encrypted_value, true);
            copied_encrypted += 1;
        } else {
            // Plain text values are copied as-is
            dest_safe.add_or_update_item(key.clone(), item.value.clone(), false);
            copied_plain += 1;
        }
    }

    // Save the new safe
    dest_safe.save(dest_path)?;

    tracing::info!(
        "✓ Copied safe to {} ({} encrypted, {} plain text)",
        dest_path,
        copied_encrypted,
        copied_plain
    );

    // Save the safe key if requested or if user chooses to
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
            .join(format!("{}.key", dest_safe.uuid));
        save_safe_key(&dest_safe, &dest_password)?;
        tracing::info!(
            "✓ Safe key saved for automatic authentication at {}",
            key_file.display()
        );
    } else {
        tracing::info!(
            "💡 Tip: Use 'skit --safe {} remember-safekey' to save your safe key securely for easy access",
            dest_path
        );
    }

    Ok(())
}

fn save_safe_key(safe: &Safe, password: &str) -> Result<(), SkitError> {
    // Create the ~/.config/skit/keys directory
    let home_dir = dirs::home_dir().ok_or_else(|| {
        SkitError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find home directory",
        ))
    })?;

    let skit_keys_dir = home_dir.join(".config").join("skit").join("keys");
    fs::create_dir_all(&skit_keys_dir).map_err(SkitError::Io)?;

    // Save the password to ~/.config/skit/keys/<uuid>.key with secure permissions
    let key_file = skit_keys_dir.join(format!("{}.key", safe.uuid));
    crate::fs_utils::write_secret_file_secure(&key_file, password)?;

    Ok(())
}
