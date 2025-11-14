use crate::error::SkitError;
use aws_sdk_ssm::Client;

/// Initialize AWS SSM client with the default credential provider chain
///
/// This will try to load credentials from:
/// 1. Environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
/// 2. ~/.aws/credentials file
/// 3. IAM role (when running on EC2, ECS, Lambda, etc.)
pub async fn create_ssm_client(region: Option<String>) -> Result<Client, SkitError> {
    let config = if let Some(region) = region {
        // Use explicit region if provided
        let region_provider = aws_sdk_ssm::config::Region::new(region);
        aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await
    } else {
        // Use default region from AWS config or environment
        aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await
    };

    Ok(Client::new(&config))
}
