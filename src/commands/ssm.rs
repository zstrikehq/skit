use crate::aws::{client, parameters};
use crate::commands::template::{CommandTemplate, MessageOutput};
use crate::crypto;
use crate::error::SkitError;
use crate::types::Safe;
use std::sync::mpsc;

/// Arguments for the SSM pull command
#[derive(Debug)]
pub struct SsmPullArgs {
    pub prefix: Option<String>,
    pub region: Option<String>,
    pub replace: bool,
    pub no_overwrite: bool,
    pub dry_run: bool,
}

/// Template-based implementation of the SSM pull command
pub struct SsmPullCommand;

impl CommandTemplate for SsmPullCommand {
    type Args = SsmPullArgs;
    type Output = MessageOutput;

    fn validate_args(&self, args: &Self::Args) -> Result<(), SkitError> {
        if let Some(prefix) = &args.prefix
            && prefix.trim().is_empty()
        {
            return Err(SkitError::ParseError(
                "SSM prefix cannot be empty when provided".to_string(),
            ));
        }
        Ok(())
    }

    fn requires_safe_loading(&self) -> bool {
        true
    }

    fn requires_authentication(&self, _safe: &Safe, _args: &Self::Args) -> bool {
        true // Required to re-encrypt SecureString parameters from AWS SSM
    }

    fn execute_operation(
        &self,
        safe: &mut Safe,
        password: Option<String>,
        args: Self::Args,
    ) -> Result<Self::Output, SkitError> {
        let SsmPullArgs {
            prefix,
            region,
            replace,
            no_overwrite,
            dry_run,
        } = args;

        let resolved_prefix = match prefix.as_ref() {
            Some(prefix) => {
                let trimmed = prefix.trim();
                if trimmed.is_empty() {
                    return Err(SkitError::ParseError(
                        "SSM prefix cannot be empty when provided".to_string(),
                    ));
                }
                trimmed.to_string()
            }
            None => safe
                .ssm_prefix
                .as_ref()
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .ok_or_else(|| {
                    SkitError::ParseError(
                        "No SSM prefix available. Provide --prefix or set one via `skit init --ssm-prefix ...` or a prior `skit ssm pull --prefix ...`."
                            .to_string(),
                    )
                })?,
        };

        let region_for_fetch = region.clone();
        let prefix_for_fetch = resolved_prefix.clone();
        let ssm_parameters = run_async_blocking(async move {
            let ssm_client = client::create_ssm_client(region_for_fetch.clone()).await?;
            parameters::fetch_parameters(&ssm_client, &prefix_for_fetch, true).await
        })?;

        if dry_run {
            let mut message = format!(
                "Dry run: Would pull {} parameters from SSM prefix '{}'\n\n",
                ssm_parameters.len(),
                resolved_prefix
            );

            for param in ssm_parameters.iter().take(10) {
                let param_type = if param.is_encrypted {
                    "SecureString (will be encrypted)"
                } else {
                    "String (will be plain text)"
                };
                message.push_str(&format!("  {} [{}]\n", param.key, param_type));
            }

            if ssm_parameters.len() > 10 {
                message.push_str(&format!("  ... and {} more\n", ssm_parameters.len() - 10));
            }

            return Ok(MessageOutput { message });
        }

        let mut added_count = 0;
        let mut updated_count = 0;
        let mut skipped_count = 0;
        let mut encrypted_count = 0;
        let mut plain_count = 0;

        if replace {
            safe.items.clear();
        }

        for param in ssm_parameters {
            if no_overwrite && safe.find_item(&param.key).is_some() {
                skipped_count += 1;
                continue;
            }

            let is_new = safe.find_item(&param.key).is_none();

            if param.is_encrypted {
                let password = password.as_ref().ok_or_else(|| {
                    SkitError::InvalidPassword(
                        "Password required to encrypt SecureString parameters".to_string(),
                    )
                })?;

                let encrypted_value = crypto::EncryptBuilder::new()
                    .plaintext(&param.value)
                    .password(password)
                    .encrypt()
                    .map_err(SkitError::Crypto)?;

                safe.add_or_update_item(param.key.clone(), encrypted_value, true);
                encrypted_count += 1;
            } else {
                safe.add_or_update_item(param.key.clone(), param.value, false);
                plain_count += 1;
            }

            if is_new {
                added_count += 1;
            } else {
                updated_count += 1;
            }
        }

        safe.ssm_prefix = Some(resolved_prefix.clone());
        safe.ssm_region = region.clone();

        let message = format!(
            "Successfully pulled {} parameters from SSM prefix '{}'\n\
             Added: {}, Updated: {}, Skipped: {}\n\
             Encrypted: {}, Plain text: {}",
            added_count + updated_count,
            resolved_prefix,
            added_count,
            updated_count,
            skipped_count,
            encrypted_count,
            plain_count
        );

        Ok(MessageOutput { message })
    }

    fn modifies_safe(&self) -> bool {
        true
    }

    fn format_output(
        &self,
        output: Self::Output,
        _format: &crate::OutputFormat,
    ) -> Result<(), SkitError> {
        crate::display::print_success(&output.message);
        Ok(())
    }
}

pub fn ssm_pull(
    safe_path: &str,
    prefix: Option<&str>,
    region: Option<String>,
    replace: bool,
    no_overwrite: bool,
    dry_run: bool,
) -> Result<(), SkitError> {
    use crate::display::print_info;

    print_info("Pulling parameters from AWS SSM Parameter Store...\n");

    let command = SsmPullCommand;
    let args = SsmPullArgs {
        prefix: prefix.map(|p| p.to_string()),
        region,
        replace,
        no_overwrite,
        dry_run,
    };

    command.validate_args(&args)?;
    command.execute(safe_path, &crate::OutputFormat::Table, args)?;

    Ok(())
}

fn run_async_blocking<T, F>(future: F) -> Result<T, SkitError>
where
    T: Send + 'static,
    F: std::future::Future<Output = Result<T, SkitError>> + Send + 'static,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        let (tx, rx) = mpsc::channel();
        handle.spawn(async move {
            let _ = tx.send(future.await);
        });

        match rx.recv() {
            Ok(result) => result,
            Err(e) => Err(SkitError::AwsError(format!(
                "Failed to receive result from async task: {}",
                e
            ))),
        }
    } else {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| SkitError::AwsError(format!("Failed to create async runtime: {}", e)))?;
        runtime.block_on(future)
    }
}
