use crate::OutputFormat;
use crate::commands::template::{CommandTemplate, MessageOutput};
use crate::crypto;
use crate::error::SkitError;
use crate::types::Safe;
use crate::validation::is_valid_env_key;

/// Arguments for the set command
#[derive(Debug)]
pub struct SetArgs {
    pub key: String,
    pub value: String,
    pub is_plain: bool,
}

/// Template-based implementation of the set command
pub struct SetCommand;

impl CommandTemplate for SetCommand {
    type Args = SetArgs;
    type Output = MessageOutput;

    fn validate_args(&self, args: &Self::Args) -> Result<(), SkitError> {
        if args.key.is_empty() {
            return Err(SkitError::ParseError("Key cannot be empty".to_string()));
        }
        if !is_valid_env_key(&args.key) {
            return Err(SkitError::ParseError(format!(
                "Invalid key '{}' (must match [A-Za-z_][A-Za-z0-9_]*)",
                args.key
            )));
        }
        Ok(())
    }

    fn requires_authentication(&self, _safe: &Safe, args: &Self::Args) -> bool {
        // Only require authentication if we're storing an encrypted value
        !args.is_plain
    }

    fn execute_operation(
        &self,
        safe: &mut Safe,
        password: Option<String>,
        args: Self::Args,
    ) -> Result<Self::Output, SkitError> {
        let stored_value = if args.is_plain {
            args.value.clone()
        } else {
            // For encrypted values, we must have a password at this point
            let password = password.ok_or_else(|| {
                SkitError::InvalidPassword("Password required for encrypted values".to_string())
            })?;
            crypto::EncryptBuilder::new()
                .plaintext(&args.value)
                .password(&password)
                .encrypt()
                .map_err(SkitError::Crypto)?
        };

        safe.add_or_update_item(args.key.clone(), stored_value, !args.is_plain);

        let type_str = if args.is_plain {
            "plain text"
        } else {
            "encrypted"
        };
        Ok(MessageOutput::new(format!(
            "Set {} ({}) in safe",
            args.key, type_str
        )))
    }

    fn modifies_safe(&self) -> bool {
        true
    }

    fn format_output(&self, output: Self::Output, _format: &OutputFormat) -> Result<(), SkitError> {
        tracing::info!("✓ {}", output.message);
        Ok(())
    }
}

/// Add or update a secret in the safe
pub fn set(safe_path: &str, key: &str, value: &str, is_plain: bool) -> Result<(), SkitError> {
    let command = SetCommand;
    let args = SetArgs {
        key: key.to_string(),
        value: value.to_string(),
        is_plain,
    };

    // Use Table format as default (format doesn't matter for set command output)
    command.execute(safe_path, &OutputFormat::Table, args)
}
