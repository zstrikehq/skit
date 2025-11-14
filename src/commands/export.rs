use crate::OutputFormat;
use crate::commands::template::CommandTemplate;
use crate::crypto;
use crate::error::SkitError;
use crate::types::Safe;

/// Arguments for the export command
#[derive(Debug)]
pub struct ExportArgs;

/// Output for the export command
#[derive(Debug)]
pub struct ExportOutput {
    pub entries: Vec<(String, String)>, // (key, value) pairs
}

/// Template-based implementation of the export command
pub struct ExportCommand;

impl CommandTemplate for ExportCommand {
    type Args = ExportArgs;
    type Output = ExportOutput;

    fn requires_authentication(&self, safe: &Safe, _args: &Self::Args) -> bool {
        // Need authentication if there are any encrypted items
        safe.items.values().any(|item| item.is_encrypted)
    }

    fn execute_operation(
        &self,
        safe: &mut Safe,
        password: Option<String>,
        _args: Self::Args,
    ) -> Result<Self::Output, SkitError> {
        if safe.items.is_empty() {
            return Ok(ExportOutput { entries: vec![] });
        }

        // Sort keys for consistent output
        let mut keys: Vec<_> = safe.items.keys().collect();
        keys.sort();

        let mut entries = Vec::new();

        for key in keys {
            let item = &safe.items[key];

            let value = if item.is_encrypted {
                if let Some(ref pwd) = password {
                    match crypto::DecryptBuilder::new()
                        .ciphertext(&item.value)
                        .password(pwd)
                        .decrypt()
                    {
                        Ok(v) => v,
                        Err(_) => {
                            eprintln!("# Warning: Failed to decrypt '{}'", item.key);
                            continue;
                        }
                    }
                } else {
                    eprintln!(
                        "# Warning: No password provided for encrypted key '{}'",
                        item.key
                    );
                    continue;
                }
            } else {
                item.value.clone()
            };

            entries.push((item.key.clone(), value));
        }

        Ok(ExportOutput { entries })
    }

    fn format_output(&self, output: Self::Output, _format: &OutputFormat) -> Result<(), SkitError> {
        // Output simple KEY=value format for piping to external commands
        for (key, value) in output.entries {
            println!("{}={}", key, value);
        }
        Ok(())
    }
}

/// Output secrets in KEY=value format for piping to external commands
pub fn export(safe_path: &str) -> Result<(), SkitError> {
    let command = ExportCommand;
    let args = ExportArgs;

    command.execute(safe_path, &OutputFormat::Env, args)
}
