use crate::crypto;
use crate::display::{print_info, print_success};
use crate::error::SkitError;
use crate::password::{get_password_with_auth_chain, validate_password_strength};
use crate::types::Safe;
use std::io::{self, Write};

pub fn rotate(safe_path: &str) -> Result<(), SkitError> {
    let mut safe = Safe::load(safe_path)?;

    println!("Starting credential rotation for safe: {}", safe_path);
    println!();
    println!("⚠️  WARNING: This will rotate your salt and password.");
    println!("    All encrypted secrets will be re-encrypted with new credentials.");
    println!("    Make sure you have a backup before proceeding.");
    println!();

    // Confirmation prompt
    print!("Do you want to continue? (yes/no): ");
    let _ = io::stdout().flush();

    let mut confirmation = String::new();
    io::stdin()
        .read_line(&mut confirmation)
        .map_err(SkitError::Io)?;
    let confirmation = confirmation.trim().to_lowercase();

    if confirmation != "yes" && confirmation != "y" {
        print_info("Rotation cancelled");
        return Ok(());
    }

    println!();

    // Step 1: Verify current password and collect encrypted secrets
    let encrypted_secrets = safe
        .items
        .values()
        .filter(|item| item.is_encrypted)
        .cloned()
        .collect::<Vec<_>>();

    if encrypted_secrets.is_empty() {
        print_info("No encrypted secrets found. Only rotating salt and password hash.");
    } else {
        print_info(&format!(
            "Found {} encrypted secrets to re-encrypt",
            encrypted_secrets.len()
        ));
    }

    let old_password = if !encrypted_secrets.is_empty() {
        Some(get_password_with_auth_chain(
            &safe,
            safe_path,
            "Enter CURRENT password to decrypt existing secrets: ",
        )?)
    } else {
        None
    };

    // Step 2: Get new password
    println!();
    println!("Creating new credentials:");
    println!("Password requirements:");
    println!("  - At least 12 characters");
    println!("  - Uppercase and lowercase letters");
    println!("  - At least one digit");
    println!("  - At least one special character");

    let new_password = loop {
        let password = crate::input::prompt_password_with_fallback("Enter NEW password: ")
            .map_err(SkitError::Io)?;

        if password.is_empty() {
            eprintln!("Error: Password cannot be empty");
            continue;
        }

        match validate_password_strength(&password) {
            Ok(()) => {
                let confirm = crate::input::prompt_password_with_fallback("Confirm NEW password: ")
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
    };

    // Step 3: Decrypt all secrets with old credentials (if any)
    let mut decrypted_secrets: Vec<(String, String)> = Vec::new();
    if let Some(old_pwd) = &old_password {
        print_info("Decrypting secrets with current credentials...");

        for item in safe.items.values() {
            if item.is_encrypted {
                let decrypted = crypto::DecryptBuilder::new()
                    .ciphertext(&item.value)
                    .password(old_pwd)
                    .decrypt()
                    .map_err(SkitError::Crypto)?;
                decrypted_secrets.push((item.key.clone(), decrypted));
                print_info(&format!("Decrypted: {}", item.key));
            }
        }
    }

    // Step 4: Generate new password hash
    print_info("Generating new password hash...");
    safe.password_hash = crypto::hash_password(&new_password)?;

    // Step 5: Re-encrypt all secrets with new credentials
    if !decrypted_secrets.is_empty() {
        print_info("Re-encrypting secrets with new credentials...");

        for (key, decrypted_value) in decrypted_secrets {
            // Re-encrypt with new password and new per-secret salt
            let re_encrypted = crypto::EncryptBuilder::new()
                .plaintext(&decrypted_value)
                .password(&new_password)
                .encrypt()
                .map_err(SkitError::Crypto)?;

            // Update the item in the safe
            if let Some(item) = safe.items.get_mut(&key) {
                item.value = re_encrypted;
                print_info(&format!("Re-encrypted: {}", key));
            }
        }
    }

    // Step 6: Save the rotated safe
    safe.save(safe_path)?;

    println!();
    print_success("Credential rotation completed successfully!");
    print_info("New password is now active");
    if !encrypted_secrets.is_empty() {
        print_info(&format!(
            "Re-encrypted {} secrets with new per-secret salts",
            encrypted_secrets.len()
        ));
    }
    print_info(&format!("Safe UUID: {}", safe.uuid));
    print_info(
        "💡 Tip: Use 'skit remember-safekey' to save your new safe key securely for easy access",
    );

    Ok(())
}
