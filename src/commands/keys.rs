use crate::OutputFormat;
use crate::commands::template::CommandTemplate;
use crate::display::{format_json_output, print_keys_table};
use crate::error::SkitError;
use crate::types::{KeyItem, KeysOutput, Safe};

/// Arguments for the keys command (no arguments needed)
#[derive(Debug)]
pub struct KeysArgs;

/// Output for the keys command
#[derive(Debug)]
pub struct KeysCommandOutput {
    pub items: Vec<(String, bool)>, // (key, is_encrypted)
}

/// Template-based implementation of the keys command
pub struct KeysCommand;

impl CommandTemplate for KeysCommand {
    type Args = KeysArgs;
    type Output = KeysCommandOutput;

    fn requires_authentication(&self, _safe: &Safe, _args: &Self::Args) -> bool {
        // Keys command doesn't need authentication since it only shows key names and types
        false
    }

    fn execute_operation(
        &self,
        safe: &mut Safe,
        _password: Option<String>,
        _args: Self::Args,
    ) -> Result<Self::Output, SkitError> {
        if safe.items.is_empty() {
            return Ok(KeysCommandOutput { items: vec![] });
        }

        // Sort keys for consistent output
        let mut keys: Vec<_> = safe.items.keys().collect();
        keys.sort();

        let mut items = Vec::new();
        for key in keys {
            let item = &safe.items[key];
            items.push((item.key.clone(), item.is_encrypted));
        }

        Ok(KeysCommandOutput { items })
    }

    fn format_output(&self, output: Self::Output, format: &OutputFormat) -> Result<(), SkitError> {
        if output.items.is_empty() {
            match format {
                OutputFormat::Json => {
                    let keys_output = KeysOutput { keys: vec![] };
                    println!("{}", format_json_output(&keys_output)?);
                }
                _ => {
                    print_keys_table(&[]);
                }
            }
            return Ok(());
        }

        match format {
            OutputFormat::Json => {
                let keys: Vec<KeyItem> = output
                    .items
                    .iter()
                    .map(|(key, is_encrypted)| KeyItem {
                        key: key.clone(),
                        item_type: if *is_encrypted {
                            "ENC".to_string()
                        } else {
                            "PLAIN".to_string()
                        },
                    })
                    .collect();

                let keys_output = KeysOutput { keys };
                println!("{}", format_json_output(&keys_output)?);
            }
            _ => {
                print_keys_table(&output.items);
            }
        }

        Ok(())
    }
}

/// List all secret keys with their types
pub fn keys(safe_path: &str, format: &OutputFormat) -> Result<(), SkitError> {
    let command = KeysCommand;
    let args = KeysArgs;

    command.execute(safe_path, format, args)
}
