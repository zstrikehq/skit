use crate::OutputFormat;
use crate::error::SkitError;
use crate::password::get_password_with_auth_chain_formatted;
use crate::types::Safe;

/// Template method trait for SKIT commands
pub trait CommandTemplate {
    /// Input arguments type for this command
    type Args;
    /// Output result type for this command  
    type Output;

    fn validate_args(&self, _args: &Self::Args) -> Result<(), SkitError> {
        Ok(())
    }

    fn requires_safe_loading(&self) -> bool {
        true
    }

    fn requires_authentication(&self, safe: &Safe, args: &Self::Args) -> bool;

    fn execute_operation(
        &self,
        safe: &mut Safe,
        password: Option<String>,
        args: Self::Args,
    ) -> Result<Self::Output, SkitError>;

    fn modifies_safe(&self) -> bool {
        false
    }

    fn format_output(&self, output: Self::Output, _format: &OutputFormat) -> Result<(), SkitError>
    where
        Self::Output: std::fmt::Debug,
    {
        println!("{:?}", output);
        Ok(())
    }

    fn execute(
        &self,
        safe_path: &str,
        format: &OutputFormat,
        args: Self::Args,
    ) -> Result<(), SkitError>
    where
        Self::Output: std::fmt::Debug,
    {
        self.validate_args(&args)?;

        let mut safe = if self.requires_safe_loading() {
            Safe::load(safe_path)?
        } else {
            return Err(SkitError::ParseError(
                "Safe loading required but not implemented".to_string(),
            ));
        };

        let password = if self.requires_authentication(&safe, &args) {
            Some(get_password_with_auth_chain_formatted(
                &safe,
                safe_path,
                "Enter safe password: ",
                Some(format),
            )?)
        } else {
            None
        };

        let output = self.execute_operation(&mut safe, password, args)?;

        if self.modifies_safe() {
            safe.save(safe_path)?;
        }

        self.format_output(output, format)?;

        Ok(())
    }
}

/// Result type for commands that just print a message
#[derive(Debug)]
pub struct MessageOutput {
    pub message: String,
}

impl MessageOutput {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
