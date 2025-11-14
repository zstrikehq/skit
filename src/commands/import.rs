use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::OutputFormat;
use crate::commands::template::{CommandTemplate, MessageOutput};
use crate::crypto;
use crate::display::{print_info, print_success};
use crate::error::SkitError;
use crate::types::Safe;
use crate::validation::is_valid_env_key;

/// Arguments for the import command
#[derive(Debug)]
pub struct ImportArgs {
    pub file_path: String,
    pub plain_keys: Option<HashSet<String>>,
}

/// Template-based implementation of the import command
pub struct ImportCommand;

impl CommandTemplate for ImportCommand {
    type Args = ImportArgs;
    type Output = MessageOutput;

    fn validate_args(&self, args: &Self::Args) -> Result<(), SkitError> {
        if !Path::new(&args.file_path).exists() {
            return Err(SkitError::ParseError(format!(
                "Input file '{}' does not exist",
                args.file_path
            )));
        }
        Ok(())
    }

    fn requires_safe_loading(&self) -> bool {
        false
    }

    fn requires_authentication(&self, _safe: &Safe, args: &Self::Args) -> bool {
        match &args.plain_keys {
            Some(_) => true,
            None => true,
        }
    }

    fn execute_operation(
        &self,
        safe: &mut Safe,
        password: Option<String>,
        args: Self::Args,
    ) -> Result<Self::Output, SkitError> {
        let file_content = fs::read_to_string(&args.file_path)
            .map_err(|e| SkitError::ParseError(format!("Failed to read file: {}", e)))?;

        let parsed_vars = parse_env_file(&file_content)?;

        if parsed_vars.is_empty() {
            return Err(SkitError::ParseError(
                "No valid key-value pairs found in input file".to_string(),
            ));
        }

        if let Some(plain_keys) = &args.plain_keys {
            let file_keys: HashSet<String> = parsed_vars.iter().map(|(k, _)| k.clone()).collect();
            let missing_keys: Vec<&String> = plain_keys.difference(&file_keys).collect();
            if !missing_keys.is_empty() {
                crate::display::print_info(&format!(
                    "⚠️  Warning: Plain keys not found in file: {}",
                    missing_keys
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }

        let mut encrypted_count = 0;
        let mut plain_count = 0;

        for (key, value) in parsed_vars {
            let should_encrypt = determine_encryption(&key, &args.plain_keys);

            if should_encrypt {
                let password = password.as_ref().ok_or_else(|| {
                    SkitError::InvalidPassword("Password required for encrypted values".to_string())
                })?;
                let encrypted_value = crypto::EncryptBuilder::new()
                    .plaintext(&value)
                    .password(password)
                    .encrypt()
                    .map_err(SkitError::Crypto)?;
                safe.add_or_update_item(key, encrypted_value, true);
                encrypted_count += 1;
            } else {
                safe.add_or_update_item(key, value, false);
                plain_count += 1;
            }
        }

        Ok(MessageOutput::new(format!(
            "Imported {} secrets: {} encrypted, {} plain text",
            encrypted_count + plain_count,
            encrypted_count,
            plain_count
        )))
    }

    fn modifies_safe(&self) -> bool {
        true
    }

    fn format_output(&self, output: Self::Output, _format: &OutputFormat) -> Result<(), SkitError> {
        print_success(&output.message);
        Ok(())
    }
}

/// Parse a .env style file into key-value pairs
fn parse_env_file(content: &str) -> Result<Vec<(String, String)>, SkitError> {
    let mut vars = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let value = line[eq_pos + 1..].trim();

            let value = if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                value[1..value.len() - 1].to_string()
            } else {
                value.to_string()
            };

            if key.is_empty() {
                return Err(SkitError::ParseError(format!(
                    "Empty key on line {}",
                    line_num + 1
                )));
            }
            if !is_valid_env_key(&key) {
                return Err(SkitError::ParseError(format!(
                    "Invalid key '{}' on line {} (must match [A-Za-z_][A-Za-z0-9_]*)",
                    key,
                    line_num + 1
                )));
            }

            vars.push((key, value));
        } else {
            return Err(SkitError::ParseError(format!(
                "Invalid format on line {}: expected KEY=VALUE",
                line_num + 1
            )));
        }
    }

    Ok(vars)
}

/// Determine if a key should be encrypted based on the command options
fn determine_encryption(key: &str, plain_keys: &Option<HashSet<String>>) -> bool {
    match plain_keys {
        Some(plain_set) => !plain_set.contains(key),
        None => true,
    }
}

/// Parse a comma-separated string into a HashSet
fn parse_key_list(keys_str: &str) -> HashSet<String> {
    keys_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Import secrets from an existing cleartext file into a safe
pub fn import(safe_path: &str, file_path: &str, plain_keys: Option<&str>) -> Result<(), SkitError> {
    println!("skit (Security Kit) - Finally safe to commit your secrets!");
    println!("Let's convert your cleartext secrets to a secure safe.\n");

    let command = ImportCommand;

    let plain_keys_set = plain_keys.map(parse_key_list);

    let args = ImportArgs {
        file_path: file_path.to_string(),
        plain_keys: plain_keys_set,
    };

    command.validate_args(&args)?;

    let file_content = fs::read_to_string(&args.file_path)
        .map_err(|e| SkitError::ParseError(format!("Failed to read file: {}", e)))?;
    let parsed_vars = parse_env_file(&file_content)?;
    if parsed_vars.is_empty() {
        return Err(SkitError::ParseError(
            "No valid key-value pairs found in input file".to_string(),
        ));
    }

    println!("📂 Found {} secrets in {}", parsed_vars.len(), file_path);

    if let Some(plain_keys) = &args.plain_keys {
        let keys_list: Vec<&String> = plain_keys.iter().collect();
        println!(
            "📋 {} keys will stay as plain text: {}",
            plain_keys.len(),
            keys_list
                .into_iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    if Path::new(safe_path).exists() {
        if safe_path == ".env.safe" {
            return Err(SkitError::ParseError(format!(
                "Safe file '{}' already exists.\nOptions:\n  • Use a different name: skit --safe myproject import -f {}\n  • Or remove existing file: rm {}",
                safe_path, args.file_path, safe_path
            )));
        } else {
            return Err(SkitError::ParseError(format!(
                "Safe file '{}' already exists.\nOptions:\n  • Choose a different name: skit --safe newname import -f {}\n  • Or remove existing file: rm {}",
                safe_path, args.file_path, safe_path
            )));
        }
    }

    println!("\n🔑 Creating your secure safe...");

    let password = crate::input::prompt_password_with_fallback(
        "Enter password for new safe (or hit enter to generate one automatically): ",
    )
    .map_err(SkitError::Io)?;
    println!();

    let password = if password.trim().is_empty() {
        let generated_password = crate::password::generate_secure_password();
        println!();
        print_success(&format!("🎲 Generated Password: {}", generated_password));
        print_info("Please save this password securely - you'll need it to access your safe!");
        println!();
        generated_password
    } else {
        password
    };

    let mut safe = Safe::new_with_password(&password, "Imported from file")?;

    let mut encrypted_count = 0;
    let mut plain_count = 0;

    for (key, value) in parsed_vars {
        let should_encrypt = determine_encryption(&key, &args.plain_keys);

        if should_encrypt {
            let encrypted_value = crypto::EncryptBuilder::new()
                .plaintext(&value)
                .password(&password)
                .encrypt()
                .map_err(SkitError::Crypto)?;
            safe.add_or_update_item(key, encrypted_value, true);
            encrypted_count += 1;
        } else {
            safe.add_or_update_item(key, value, false);
            plain_count += 1;
        }
    }

    safe.save(safe_path)?;

    println!();
    print_success("✅ Import complete!");
    println!(
        "   {} secrets imported ({} encrypted, {} plain text)",
        encrypted_count + plain_count,
        encrypted_count,
        plain_count
    );
    println!("   Safe created: {}", safe_path);

    println!();
    let save_key = prompt_yes_no("Save safe key for easy access? (y/N): ", false)?;
    if save_key {
        let key_path =
            crate::commands::remember_safekey_with_password_quiet(&safe, &password, true)?;
        println!(
            "✅ Safe key saved to {}! No more password prompts needed.",
            key_path
        );
        println!("   🔐 Keep this key \x1b[31m↑\x1b[0m  secure - never commit it to git!");
    }

    println!();
    print_info("🔐 Your secrets are now secure and safe to commit to git!");

    let usage_example = if safe_path == ".env.safe" {
        "🚀 Try: skit print".to_string()
    } else {
        format!(
            "🚀 Try: skit --safe {} print",
            Path::new(safe_path)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or(safe_path)
        )
    };
    print_info(&usage_example);

    Ok(())
}

/// Simple yes/no prompt
fn prompt_yes_no(prompt: &str, default: bool) -> Result<bool, SkitError> {
    print!("{}", prompt);
    io::stdout().flush().map_err(SkitError::Io)?;

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(SkitError::Io)?;

    let input = input.trim().to_lowercase();
    match input.as_str() {
        "y" | "yes" => Ok(true),
        "n" | "no" => Ok(false),
        "" => Ok(default),
        _ => Ok(default),
    }
}
