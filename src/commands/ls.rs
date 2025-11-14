use crate::display::{format_json_output, print_info};
use crate::error::SkitError;
use crate::types::{Safe, SafeInfo, SafeStatistics, SafesListOutput};
use std::fs;
use std::path::Path;

pub fn ls(format: &crate::OutputFormat) -> Result<(), SkitError> {
    // Find all .safe files in current directory
    let current_dir = std::env::current_dir().map_err(SkitError::Io)?;

    let entries = fs::read_dir(&current_dir).map_err(SkitError::Io)?;

    let mut safe_files = Vec::new();

    for entry in entries {
        let entry = entry.map_err(SkitError::Io)?;
        let path = entry.path();

        if let Some(filename_str) = path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| name.ends_with(".safe"))
        {
            safe_files.push(filename_str.to_string());
        }
    }

    if safe_files.is_empty() {
        match format {
            crate::OutputFormat::Json => {
                let output = SafesListOutput { safes: vec![] };
                println!("{}", format_json_output(&output)?);
            }
            _ => {
                println!("No safes found in current directory");
            }
        }
        return Ok(());
    }

    safe_files.sort();

    // Collect safe information
    let mut safe_infos = Vec::new();

    for safe_file in &safe_files {
        let safe_path = Path::new(&safe_file);

        match safe_path.to_str().and_then(|path| Safe::load(path).ok()) {
            Some(safe) => {
                let total = safe.items.len();
                let encrypted = safe.items.values().filter(|item| item.is_encrypted).count();
                let plain = total - encrypted;
                let status = if total == 0 {
                    "Empty".to_string()
                } else {
                    "OK".to_string()
                };

                safe_infos.push(SafeInfo {
                    file: safe_file.clone(),
                    description: safe.description,
                    statistics: SafeStatistics {
                        total,
                        encrypted,
                        plain,
                    },
                    updated: safe.updated,
                    status,
                });
            }
            None => {
                safe_infos.push(SafeInfo {
                    file: safe_file.clone(),
                    description: "Error loading safe".to_string(),
                    statistics: SafeStatistics {
                        total: 0,
                        encrypted: 0,
                        plain: 0,
                    },
                    updated: "?".to_string(),
                    status: "Error".to_string(),
                });
            }
        }
    }

    match format {
        crate::OutputFormat::Json => {
            let output = SafesListOutput { safes: safe_infos };
            println!("{}", format_json_output(&output)?);
        }
        _ => {
            print_info(&format!(
                "Found {} safe(s) in current directory:",
                safe_files.len()
            ));
            println!();

            for (i, safe_info) in safe_infos.iter().enumerate() {
                if i > 0 {
                    println!(); // Add blank line between safes
                }

                println!("{}", safe_info.file);
                println!("  Description: {}", safe_info.description);
                println!(
                    "  Secrets: {} total ({} encrypted, {} plain)",
                    safe_info.statistics.total,
                    safe_info.statistics.encrypted,
                    safe_info.statistics.plain
                );
                println!("  Updated: {}", safe_info.updated);
                println!("  Status: {}", safe_info.status);
            }
        }
    }

    Ok(())
}
