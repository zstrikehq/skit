use crate::OutputFormat;
use crate::commands::template::{CommandTemplate, MessageOutput};
use crate::display::print_success;
use crate::error::SkitError;
use crate::types::Safe;

/// Arguments for the rm command
#[derive(Debug)]
pub struct RmArgs {
    pub key: String,
}

/// Template-based implementation of the rm command
pub struct RmCommand;

impl CommandTemplate for RmCommand {
    type Args = RmArgs;
    type Output = MessageOutput;

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
        _password: Option<String>,
        args: Self::Args,
    ) -> Result<Self::Output, SkitError> {
        // Check if key exists
        if safe.find_item(&args.key).is_none() {
            return Err(SkitError::KeyNotFound);
        }

        // Remove the item
        safe.items.remove(&args.key);

        Ok(MessageOutput::new(format!(
            "Removed '{}' from safe",
            args.key
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

/// Remove a secret from the safe
pub fn rm(safe_path: &str, key: &str) -> Result<(), SkitError> {
    let command = RmCommand;
    let args = RmArgs {
        key: key.to_string(),
    };

    command.execute(safe_path, &OutputFormat::Table, args)
}
