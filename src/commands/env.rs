use crate::OutputFormat;
use crate::commands::template::CommandTemplate;
use crate::crypto;
use crate::display::shell_quote;
use crate::error::SkitError;
use crate::shell::detect_shell;
use crate::types::Safe;
use crate::validation::is_valid_env_key;

/// Arguments for the env command (no arguments needed)
#[derive(Debug)]
pub struct EnvArgs;

/// Output for the env command
#[derive(Debug)]
pub struct EnvOutput {
    pub entries: Vec<(String, String)>, // (key, value) pairs
    pub shell_name: String,
}

/// Template-based implementation of the env command
pub struct EnvCommand;

impl CommandTemplate for EnvCommand {
    type Args = EnvArgs;
    type Output = EnvOutput;

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
        let shell = detect_shell();

        if safe.items.is_empty() {
            return Ok(EnvOutput {
                entries: vec![],
                shell_name: shell.name,
            });
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

        Ok(EnvOutput {
            entries,
            shell_name: shell.name,
        })
    }

    fn format_output(&self, output: Self::Output, _format: &OutputFormat) -> Result<(), SkitError> {
        // Use shell-appropriate syntax
        for (key, value) in output.entries {
            if !is_valid_env_key(&key) {
                eprintln!("# Warning: Skipping invalid environment key: {}", key);
                continue;
            }
            match output.shell_name.as_str() {
                "fish" => {
                    println!("set -x {} {}", key, shell_quote(&value));
                }
                "powershell" => {
                    println!("$env:{} = {}", key, shell_quote(&value));
                }
                "cmd" => {
                    println!("set {}={}", key, value); // cmd doesn't need quoting like Unix
                }
                "csh" | "tcsh" => {
                    println!("setenv {} {}", key, shell_quote(&value));
                }
                "nu" => {
                    println!("let-env {} = {}", key, shell_quote(&value));
                }
                _ => {
                    println!("export {}={}", key, shell_quote(&value));
                }
            }
        }
        Ok(())
    }
}

/// Output secrets for shell sourcing
pub fn env(safe_path: &str) -> Result<(), SkitError> {
    let command = EnvCommand;
    let args = EnvArgs;

    command.execute(safe_path, &OutputFormat::Env, args)
}
