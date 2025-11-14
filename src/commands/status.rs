use crate::OutputFormat;
use crate::commands::template::CommandTemplate;
use crate::crypto;
use crate::display::{format_json_output, print_info, print_success, print_warning};
use crate::error::SkitError;
use crate::types::{
    Safe, StatusIntegrity, StatusMetadata, StatusOutput, StatusStatistics,
    StatusVerificationDetails,
};

/// Arguments for the status command (no arguments needed)
#[derive(Debug)]
pub struct StatusArgs;

/// Output for the status command
#[derive(Debug)]
pub struct StatusCommandOutput {
    pub status_output: StatusOutput,
    pub encrypted_count: usize,
    pub verification_details: Option<StatusVerificationDetails>,
}

/// Template-based implementation of the status command
pub struct StatusCommand;

impl CommandTemplate for StatusCommand {
    type Args = StatusArgs;
    type Output = StatusCommandOutput;

    fn requires_authentication(&self, safe: &Safe, _args: &Self::Args) -> bool {
        // Status always requires authentication to verify integrity
        !safe.password_hash.is_empty()
    }

    fn execute_operation(
        &self,
        safe: &mut Safe,
        password: Option<String>,
        _args: Self::Args,
    ) -> Result<Self::Output, SkitError> {
        // Count statistics
        let total_items = safe.items.len();
        let encrypted_count = safe.items.values().filter(|item| item.is_encrypted).count();
        let plain_count = total_items - encrypted_count;

        let password_hash_ok;
        let mut verification_details = None;
        let encrypted_secrets_verified;

        match password {
            Some(password) => {
                // Password verification succeeded, so hash is definitely OK
                password_hash_ok = true;

                // If we have encrypted secrets, also test decryption
                if encrypted_count > 0 {
                    let mut verification_results = Vec::new();
                    let mut failed_count = 0;

                    for item in safe.items.values() {
                        if item.is_encrypted {
                            match crypto::DecryptBuilder::new()
                                .ciphertext(&item.value)
                                .password(&password)
                                .decrypt()
                            {
                                Ok(_) => {
                                    verification_results.push((item.key.clone(), true));
                                }
                                Err(_) => {
                                    verification_results.push((item.key.clone(), false));
                                    failed_count += 1;
                                }
                            }
                        }
                    }

                    let verified_count = encrypted_count - failed_count;
                    encrypted_secrets_verified = Some(failed_count == 0);

                    let failed_keys: Vec<String> = verification_results
                        .iter()
                        .filter_map(
                            |(key, success)| {
                                if !success { Some(key.clone()) } else { None }
                            },
                        )
                        .collect();

                    verification_details = Some(StatusVerificationDetails {
                        total_encrypted: encrypted_count,
                        verified: verified_count,
                        failed: failed_count,
                        failed_keys,
                    });
                } else {
                    encrypted_secrets_verified = Some(true); // No encrypted secrets, so verification is trivially successful
                }
            }
            None => {
                // Password verification failed - corrupted hash or wrong password
                password_hash_ok = false;
                encrypted_secrets_verified = Some(false);
            }
        }

        let output = StatusOutput {
            safe_path: "".to_string(), // Will be overridden in format_output
            metadata: StatusMetadata {
                version: safe.version.clone(),
                description: safe.description.clone(),
                created: safe.created.clone(),
                updated: safe.updated.clone(),
            },
            statistics: StatusStatistics {
                total_secrets: total_items,
                encrypted: encrypted_count,
                plain_text: plain_count,
            },
            integrity: StatusIntegrity {
                password_hash_ok,
                encrypted_secrets_verified,
                verification_details: verification_details.clone(),
            },
        };

        Ok(StatusCommandOutput {
            status_output: output,
            encrypted_count,
            verification_details,
        })
    }

    fn format_output(&self, output: Self::Output, format: &OutputFormat) -> Result<(), SkitError> {
        match format {
            OutputFormat::Json => {
                println!("{}", format_json_output(&output.status_output)?);
            }
            _ => {
                // Original text output with verification messages
                println!();
                print_info("Verifying password hash integrity...");

                if output.encrypted_count > 0 {
                    print_info("Verifying encrypted secrets...");
                }

                print_info(&format!("Safe: {}", &output.status_output.safe_path));
                println!();

                // Display metadata
                println!("Metadata:");
                println!("  Version: {}", output.status_output.metadata.version);
                println!(
                    "  Description: {}",
                    output.status_output.metadata.description
                );
                println!("  Created: {}", output.status_output.metadata.created);
                println!("  Last updated: {}", output.status_output.metadata.updated);
                println!();

                // Display statistics
                println!("Statistics:");
                println!(
                    "  Total secrets: {}",
                    output.status_output.statistics.total_secrets
                );
                println!(
                    "  Encrypted:     {}",
                    output.status_output.statistics.encrypted
                );
                println!(
                    "  Plain text:    {}",
                    output.status_output.statistics.plain_text
                );

                // Display integrity status
                println!();
                println!("Integrity:");
                if output.status_output.integrity.password_hash_ok {
                    println!("  Password hash: OK");
                } else {
                    println!("  Password hash: CORRUPTED (invalid format)");
                }

                if output.encrypted_count == 0 {
                    println!();
                    print_success("No encrypted secrets to verify");
                } else if let Some(details) = &output.verification_details {
                    println!();
                    if details.failed == 0 {
                        print_success(&format!(
                            "All {} encrypted secrets verified",
                            output.encrypted_count
                        ));
                    } else {
                        print_warning(&format!(
                            "{} of {} encrypted secrets failed verification",
                            details.failed, output.encrypted_count
                        ));
                        println!();
                        println!("Failed secrets:");
                        for key in &details.failed_keys {
                            println!("  - {}", key);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl StatusCommand {
    /// Custom execute method that properly sets the safe_path
    pub fn execute_with_path(
        &self,
        safe_path: &str,
        format: &OutputFormat,
        args: StatusArgs,
    ) -> Result<(), SkitError> {
        // Step 1: Validate arguments
        self.validate_args(&args)?;

        // Step 2: Load safe
        let mut safe = Safe::load(safe_path)?;

        // Step 3: Authenticate (if required)
        let password = if self.requires_authentication(&safe, &args) {
            Some(crate::password::get_password_with_auth_chain_formatted(
                &safe,
                safe_path,
                "Enter safe password: ",
                Some(format),
            )?)
        } else {
            None
        };

        // Step 4: Execute core operation
        let mut output = self.execute_operation(&mut safe, password, args)?;

        // Step 5: Set the safe_path in the output
        output.status_output.safe_path = safe_path.to_string();

        // Step 6: Save safe (if modified) - not needed for status

        // Step 7: Format and display output
        self.format_output(output, format)?;

        Ok(())
    }
}

/// Show safe metadata and integrity status
pub fn status(safe_path: &str, format: &OutputFormat) -> Result<(), SkitError> {
    let command = StatusCommand;
    let args = StatusArgs;

    command.execute_with_path(safe_path, format, args)
}
