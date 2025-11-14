use crate::OutputFormat;
use crate::commands::template::CommandTemplate;
use crate::crypto;
use crate::error::SkitError;
use crate::types::Safe;

/// Arguments for the get command
#[derive(Debug)]
pub struct GetArgs {
    pub key: String,
}

/// Output for the get command
#[derive(Debug)]
pub struct GetOutput {
    pub value: String,
}

/// Template-based implementation of the get command
pub struct GetCommand;

impl CommandTemplate for GetCommand {
    type Args = GetArgs;
    type Output = GetOutput;

    fn validate_args(&self, args: &Self::Args) -> Result<(), SkitError> {
        if args.key.is_empty() {
            return Err(SkitError::ParseError("Key cannot be empty".to_string()));
        }
        Ok(())
    }

    fn requires_authentication(&self, safe: &Safe, args: &Self::Args) -> bool {
        // Only require authentication if the key exists and is encrypted
        if let Some(item) = safe.find_item(&args.key) {
            item.is_encrypted
        } else {
            false // If key doesn't exist, we'll handle that in execute_operation
        }
    }

    fn execute_operation(
        &self,
        safe: &mut Safe,
        password: Option<String>,
        args: Self::Args,
    ) -> Result<Self::Output, SkitError> {
        let item = safe.find_item(&args.key).ok_or(SkitError::KeyNotFound)?;

        let value = if item.is_encrypted {
            // For encrypted values, we must have a password at this point
            let password = password.ok_or_else(|| {
                SkitError::InvalidPassword("Password required for encrypted values".to_string())
            })?;
            crypto::DecryptBuilder::new()
                .ciphertext(&item.value)
                .password(&password)
                .decrypt()
                .map_err(SkitError::Crypto)?
        } else {
            item.value.clone()
        };

        Ok(GetOutput { value })
    }

    fn format_output(&self, output: Self::Output, _format: &OutputFormat) -> Result<(), SkitError> {
        println!("{}", output.value);
        Ok(())
    }
}

/// Get a secret value from the safe
pub fn get(safe_path: &str, key: &str) -> Result<(), SkitError> {
    let command = GetCommand;
    let args = GetArgs {
        key: key.to_string(),
    };

    // Use Table format as default (format doesn't matter for get command output)
    command.execute(safe_path, &OutputFormat::Table, args)
}
