use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Safe {
    pub version: String,
    pub uuid: String,
    pub description: String,
    pub created: String,
    pub updated: String,
    pub password_hash: String,
    pub ssm_prefix: Option<String>,
    pub ssm_region: Option<String>,
    pub items: HashMap<String, SafeItem>,
}

#[derive(Debug, Clone)]
pub struct SafeItem {
    pub key: String,
    pub value: String,
    pub is_encrypted: bool,
}

// JSON output structures
#[derive(Serialize)]
pub struct PrintOutput {
    pub items: Vec<PrintItem>,
}

#[derive(Serialize)]
pub struct PrintItem {
    pub key: String,
    pub value: String,
    #[serde(rename = "type")]
    pub item_type: String,
}

#[derive(Serialize)]
pub struct KeysOutput {
    pub keys: Vec<KeyItem>,
}

#[derive(Serialize)]
pub struct KeyItem {
    pub key: String,
    #[serde(rename = "type")]
    pub item_type: String,
}

#[derive(Serialize, Debug)]
pub struct StatusOutput {
    pub safe_path: String,
    pub metadata: StatusMetadata,
    pub statistics: StatusStatistics,
    pub integrity: StatusIntegrity,
}

#[derive(Serialize, Debug)]
pub struct StatusMetadata {
    pub version: String,
    pub description: String,
    pub created: String,
    pub updated: String,
}

#[derive(Serialize, Debug)]
pub struct StatusStatistics {
    pub total_secrets: usize,
    pub encrypted: usize,
    pub plain_text: usize,
}

#[derive(Serialize, Debug)]
pub struct StatusIntegrity {
    pub password_hash_ok: bool,
    pub encrypted_secrets_verified: Option<bool>,
    pub verification_details: Option<StatusVerificationDetails>,
}

#[derive(Serialize, Debug, Clone)]
pub struct StatusVerificationDetails {
    pub total_encrypted: usize,
    pub verified: usize,
    pub failed: usize,
    pub failed_keys: Vec<String>,
}

#[derive(Serialize)]
pub struct SafesListOutput {
    pub safes: Vec<SafeInfo>,
}

#[derive(Serialize)]
pub struct SafeInfo {
    pub file: String,
    pub description: String,
    pub statistics: SafeStatistics,
    pub updated: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct SafeStatistics {
    pub total: usize,
    pub encrypted: usize,
    pub plain: usize,
}
