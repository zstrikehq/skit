use crate::error::SkitError;
use aws_sdk_ssm::{Client, types::ParameterType};

/// Represents a pulled SSM parameter with its key, value, and encryption status
#[derive(Debug, Clone)]
pub struct SsmParameter {
    pub key: String,
    pub value: String,
    pub is_encrypted: bool,
}

/// Fetch all parameters under a given prefix from AWS SSM Parameter Store
///
/// # Arguments
/// * `client` - AWS SSM client
/// * `prefix` - Parameter path prefix (e.g., "/myapp/dev/")
/// * `strip_prefix` - Whether to strip the prefix from parameter names
///
/// # Returns
/// Vector of SsmParameter structs with key, value, and encryption status
///
/// # SSM Type Mapping
/// - `String` → plain text (is_encrypted = false)
/// - `SecureString` → decrypted value for re-encryption (is_encrypted = true)
/// - `StringList` → treated as plain text, comma-separated
pub async fn fetch_parameters(
    client: &Client,
    prefix: &str,
    strip_prefix: bool,
) -> Result<Vec<SsmParameter>, SkitError> {
    let mut parameters = Vec::new();
    let mut next_token: Option<String> = None;

    let normalized_prefix = if prefix.starts_with('/') {
        prefix.to_string()
    } else {
        format!("/{}", prefix)
    };

    loop {
        let mut request = client
            .get_parameters_by_path()
            .path(&normalized_prefix)
            .with_decryption(true);

        if let Some(token) = next_token {
            request = request.next_token(token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| SkitError::AwsError(format!("Failed to fetch parameters: {}", e)))?;

        if let Some(params) = response.parameters {
            for param in params {
                let param_name = param.name().unwrap_or("");
                let param_value = param.value().unwrap_or("");
                let param_type = param.r#type();

                let is_encrypted = matches!(param_type, Some(ParameterType::SecureString));

                let key = if strip_prefix && param_name.starts_with(&normalized_prefix) {
                    param_name[normalized_prefix.len()..]
                        .trim_start_matches('/')
                        .to_string()
                } else {
                    param_name.to_string()
                };

                if key.is_empty() {
                    continue;
                }

                parameters.push(SsmParameter {
                    key,
                    value: param_value.to_string(),
                    is_encrypted,
                });
            }
        }

        if response.next_token.is_some() {
            next_token = response.next_token;
        } else {
            break;
        }
    }

    if parameters.is_empty() {
        return Err(SkitError::AwsError(format!(
            "No parameters found under prefix: {}",
            normalized_prefix
        )));
    }

    Ok(parameters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_stripping() {
        let prefix = "/myapp/dev/";
        let param_name = "/myapp/dev/API_KEY";

        let stripped = if param_name.starts_with(prefix) {
            param_name[prefix.len()..]
                .trim_start_matches('/')
                .to_string()
        } else {
            param_name.to_string()
        };

        assert_eq!(stripped, "API_KEY");
    }

    #[test]
    fn test_nested_key_stripping() {
        let prefix = "/myapp/dev/";
        let param_name = "/myapp/dev/database/host";

        let stripped = if param_name.starts_with(prefix) {
            param_name[prefix.len()..]
                .trim_start_matches('/')
                .to_string()
        } else {
            param_name.to_string()
        };

        assert_eq!(stripped, "database/host");
    }
}
