use crate::crypto;
use crate::error::SkitError;
use crate::types::{Safe, SafeItem};
use std::collections::HashMap;
use std::fs;
use std::io;

impl Safe {
    pub fn load(path: &str) -> Result<Self, SkitError> {
        let content = fs::read_to_string(path).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                SkitError::SafeNotFound(path.to_string())
            } else {
                SkitError::Io(e)
            }
        })?;

        Self::parse(&content)
    }

    pub fn new_with_password(password: &str, description: &str) -> Result<Self, SkitError> {
        use chrono::prelude::*;
        use uuid::Uuid;

        let now = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let uuid = Uuid::new_v4().to_string();

        Ok(Safe {
            version: "1.0".to_string(),
            uuid,
            description: description.to_string(),
            created: now.clone(),
            updated: now,
            password_hash: crypto::hash_password(password)?,
            ssm_prefix: None,
            ssm_region: None,
            items: HashMap::new(),
        })
    }

    pub fn parse(content: &str) -> Result<Self, SkitError> {
        let mut version = String::new();
        let mut uuid = String::new();
        let mut description = String::new();
        let mut created = String::new();
        let mut updated = String::new();
        let mut password_hash = String::new();
        let mut ssm_prefix: Option<String> = None;
        let mut ssm_region: Option<String> = None;
        let mut items = HashMap::new();

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            if line.starts_with("#@") {
                if let Some(eq_pos) = line.find('=') {
                    let field = &line[2..eq_pos];
                    let value = &line[eq_pos + 1..];

                    match field {
                        "VERSION" => version = value.to_string(),
                        "UUID" => uuid = value.to_string(),
                        "DESCRIPTION" => description = value.to_string(),
                        "CREATED" => created = value.to_string(),
                        "UPDATED" => updated = value.to_string(),
                        "PASS_HASH" => password_hash = value.to_string(),
                        "SSM_PREFIX" => ssm_prefix = Some(value.to_string()),
                        "SSM_REGION" => ssm_region = Some(value.to_string()),
                        _ => {}
                    }
                }
                continue;
            }

            if line.starts_with('#') {
                continue;
            }

            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim().to_string();
                let value = line[eq_pos + 1..].trim().to_string();

                if key.is_empty() {
                    return Err(SkitError::ParseError(format!(
                        "Empty key on line {}",
                        line_num + 1
                    )));
                }

                // Handle encryption format versioning: v1 (current), legacy, and very old formats
                let (is_encrypted, _salt, stored_value) =
                    if let Some(content) = value.strip_prefix("ENC~") {
                        if content.starts_with("v1~") {
                            (true, None, value.to_string())
                        } else if let Some(salt_end) = content.find('~') {
                            let salt = content[..salt_end].to_string();
                            let encrypted_data = content[salt_end + 1..].to_string();
                            (true, Some(salt), format!("ENC~{}", encrypted_data))
                        } else {
                            (true, None, value.to_string())
                        }
                    } else {
                        (false, None, value)
                    };

                items.insert(
                    key.clone(),
                    SafeItem {
                        key,
                        value: stored_value,
                        is_encrypted,
                    },
                );
            } else {
                return Err(SkitError::ParseError(format!(
                    "Invalid line format on line {}: {}",
                    line_num + 1,
                    line
                )));
            }
        }

        if password_hash.is_empty() {
            return Err(SkitError::ParseError(
                "No password hash found in file. Expected #@PASS_HASH=<value>".to_string(),
            ));
        }

        if version.is_empty() {
            return Err(SkitError::ParseError(
                "No version found in file. Expected #@VERSION=<value>".to_string(),
            ));
        }
        if uuid.is_empty() {
            return Err(SkitError::ParseError(
                "No UUID found in file. Expected #@UUID=<value>".to_string(),
            ));
        }
        if description.is_empty() {
            return Err(SkitError::ParseError(
                "No description found in file. Expected #@DESCRIPTION=<value>".to_string(),
            ));
        }
        if created.is_empty() {
            return Err(SkitError::ParseError(
                "No creation date found in file. Expected #@CREATED=<value>".to_string(),
            ));
        }
        if updated.is_empty() {
            return Err(SkitError::ParseError(
                "No update date found in file. Expected #@UPDATED=<value>".to_string(),
            ));
        }

        Ok(Safe {
            version,
            uuid,
            description,
            created,
            updated,
            password_hash,
            ssm_prefix,
            ssm_region,
            items,
        })
    }

    pub fn save(&mut self, path: &str) -> Result<(), SkitError> {
        use chrono::prelude::*;
        self.updated = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

        let mut content = String::new();
        content.push_str("# ========================================\n");
        content.push_str("# SKIT SAFE METADATA - DO NOT EDIT\n");
        content.push_str("# ========================================\n");
        content.push_str(&format!("#@VERSION={}\n", self.version));
        content.push_str(&format!("#@UUID={}\n", self.uuid));
        content.push_str(&format!("#@DESCRIPTION={}\n", self.description));
        content.push_str(&format!("#@CREATED={}\n", self.created));
        content.push_str(&format!("#@UPDATED={}\n", self.updated));
        content.push_str(&format!("#@PASS_HASH={}\n", self.password_hash));

        if let Some(ref prefix) = self.ssm_prefix {
            content.push_str(&format!("#@SSM_PREFIX={}\n", prefix));
        }
        if let Some(ref region) = self.ssm_region {
            content.push_str(&format!("#@SSM_REGION={}\n", region));
        }

        content.push_str("# ========================================\n");
        content.push_str("# SECRETS (KEY=VALUE or KEY=ENC~<data>)\n");
        content.push_str("# ========================================\n");

        let mut keys: Vec<_> = self.items.keys().collect();
        keys.sort();

        for key in keys {
            let item = &self.items[key];
            let output_value = item.value.clone();
            content.push_str(&format!("{}={}\n", item.key, output_value));
        }

        fs::write(path, content)?;
        Ok(())
    }

    pub fn find_item(&self, key: &str) -> Option<&SafeItem> {
        self.items.get(key)
    }

    pub fn add_or_update_item(&mut self, key: String, value: String, is_encrypted: bool) {
        self.items.insert(
            key.clone(),
            SafeItem {
                key,
                value,
                is_encrypted,
            },
        );
    }

    pub fn verify_password(&self, password: &str) -> Result<(), SkitError> {
        crypto::verify_password(password, &self.password_hash)
            .map_err(|_| SkitError::InvalidPassword("Invalid password".to_string()))
    }
}
