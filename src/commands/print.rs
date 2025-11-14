use crate::OutputFormat;
use crate::commands::template::CommandTemplate;
use crate::crypto;
use crate::display::{format_json_output, print_grouped, print_terraform_output};
use crate::error::SkitError;
use crate::types::{PrintItem, PrintOutput, Safe};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct PostmanEnvironmentVariable {
    key: String,
    value: String,
    #[serde(rename = "type")]
    var_type: String,
    enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct PostmanEnvironment {
    id: String,
    name: String,
    values: Vec<PostmanEnvironmentVariable>,
    #[serde(rename = "_postman_variable_scope")]
    postman_variable_scope: String,
    #[serde(rename = "_postman_exported_at")]
    postman_exported_at: String,
    #[serde(rename = "_postman_exported_using")]
    postman_exported_using: String,
}

/// Arguments for the print command
#[derive(Debug)]
pub struct PrintArgs {
    pub plain_only: bool,
    pub enc_only: bool,
}

/// Output for the print command
#[derive(Debug)]
pub struct PrintCommandOutput {
    pub items: Vec<(String, String, bool)>, // (key, value, is_encrypted)
}

/// Template-based implementation of the print command
pub struct PrintCommand;

impl CommandTemplate for PrintCommand {
    type Args = PrintArgs;
    type Output = PrintCommandOutput;

    fn validate_args(&self, args: &Self::Args) -> Result<(), SkitError> {
        if args.plain_only && args.enc_only {
            return Err(SkitError::ParseError(
                "Cannot use both --plain and --enc flags together".to_string(),
            ));
        }
        Ok(())
    }

    fn requires_authentication(&self, safe: &Safe, args: &Self::Args) -> bool {
        // Only need password if we have encrypted items and we're not showing plain-only
        let has_encrypted = safe.items.values().any(|item| item.is_encrypted);
        has_encrypted && !args.plain_only
    }

    fn execute_operation(
        &self,
        safe: &mut Safe,
        password: Option<String>,
        args: Self::Args,
    ) -> Result<Self::Output, SkitError> {
        if safe.items.is_empty() {
            return Ok(PrintCommandOutput { items: vec![] });
        }

        // Sort keys for consistent output
        let mut keys: Vec<_> = safe.items.keys().collect();
        keys.sort();

        let mut output_data = Vec::new();

        for key in keys {
            let item = &safe.items[key];

            // Filter based on flags
            if args.plain_only && item.is_encrypted {
                continue; // Skip encrypted items when --plain is used
            }
            if args.enc_only && !item.is_encrypted {
                continue; // Skip plain items when --enc is used
            }

            let value = if item.is_encrypted {
                if let Some(ref pwd) = password {
                    match crypto::DecryptBuilder::new()
                        .ciphertext(&item.value)
                        .password(pwd)
                        .decrypt()
                    {
                        Ok(v) => v,
                        Err(_) => "[DECRYPTION_FAILED]".to_string(),
                    }
                } else {
                    "<Value hidden - encrypted>".to_string()
                }
            } else {
                item.value.clone()
            };

            output_data.push((item.key.clone(), value, item.is_encrypted));
        }

        Ok(PrintCommandOutput { items: output_data })
    }

    fn format_output(&self, output: Self::Output, format: &OutputFormat) -> Result<(), SkitError> {
        if output.items.is_empty() {
            match format {
                OutputFormat::Json => {
                    let print_output = PrintOutput { items: vec![] };
                    println!("{}", format_json_output(&print_output)?);
                }
                OutputFormat::Env => {
                    // No output for empty safe in env format
                }
                OutputFormat::Table => {
                    println!("No items in safe");
                }
                OutputFormat::Terraform => {
                    print_terraform_output(&output.items);
                }
                OutputFormat::Postman => {
                    let postman_env = PostmanEnvironment {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: "SKIT Environment".to_string(),
                        values: vec![],
                        postman_variable_scope: "environment".to_string(),
                        postman_exported_at: chrono::Utc::now().to_rfc3339(),
                        postman_exported_using: "SKIT".to_string(),
                    };
                    println!("{}", serde_json::to_string_pretty(&postman_env)?);
                }
            }
            return Ok(());
        }

        match format {
            OutputFormat::Json => {
                let items: Vec<PrintItem> = output
                    .items
                    .iter()
                    .map(|(key, value, is_encrypted)| PrintItem {
                        key: key.clone(),
                        value: value.clone(),
                        item_type: if *is_encrypted {
                            "ENC".to_string()
                        } else {
                            "PLAIN".to_string()
                        },
                    })
                    .collect();

                let print_output = PrintOutput { items };
                println!("{}", format_json_output(&print_output)?);
            }
            OutputFormat::Env => {
                for (key, value, _) in output.items {
                    println!("{}={}", key, value);
                }
            }
            OutputFormat::Table => {
                print_grouped(&output.items);
                let has_encrypted = output
                    .items
                    .iter()
                    .any(|(_, _, is_encrypted)| *is_encrypted);
                if has_encrypted {
                    use crate::display::print_info;
                    println!();
                    print_info(
                        "Encrypted values are stored securely in the file - only decrypted for display",
                    );
                }
            }
            OutputFormat::Terraform => {
                print_terraform_output(&output.items);
            }
            OutputFormat::Postman => {
                let values: Vec<PostmanEnvironmentVariable> = output
                    .items
                    .iter()
                    .map(|(key, value, is_encrypted)| PostmanEnvironmentVariable {
                        key: key.clone(),
                        value: value.clone(),
                        var_type: if *is_encrypted {
                            "secret".to_string()
                        } else {
                            "default".to_string()
                        },
                        enabled: true,
                    })
                    .collect();

                let postman_env = PostmanEnvironment {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "SKIT Environment".to_string(),
                    values,
                    postman_variable_scope: "environment".to_string(),
                    postman_exported_at: chrono::Utc::now().to_rfc3339(),
                    postman_exported_using: "SKIT".to_string(),
                };

                println!("{}", serde_json::to_string_pretty(&postman_env)?);
            }
        }

        Ok(())
    }
}

/// Display all secrets in organized format
pub fn print(
    safe_path: &str,
    format: &OutputFormat,
    plain_only: bool,
    enc_only: bool,
) -> Result<(), SkitError> {
    let command = PrintCommand;
    let args = PrintArgs {
        plain_only,
        enc_only,
    };

    command.execute(safe_path, format, args)
}
