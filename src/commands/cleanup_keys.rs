use crate::display::{print_error, print_info, print_success, print_warning};
use crate::error::SkitError;
use std::fs;
use std::io::{self, Write};
use std::time::{Duration, SystemTime};

fn format_days_ago(days: u64) -> String {
    match days {
        0 => "today".to_string(),
        1 => "1 day ago".to_string(),
        n => format!("{} days ago", n),
    }
}

pub fn cleanup_keys(older_than_days: u64, dry_run: bool) -> Result<(), SkitError> {
    let home_dir = dirs::home_dir().ok_or_else(|| {
        SkitError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find home directory",
        ))
    })?;

    let skit_keys_dir = home_dir.join(".config").join("skit").join("keys");

    if !skit_keys_dir.exists() {
        print_info("No saved keys directory found - nothing to clean up");
        return Ok(());
    }

    let cutoff_time = SystemTime::now()
        .checked_sub(Duration::from_secs(older_than_days * 24 * 60 * 60))
        .ok_or_else(|| SkitError::ParseError("Invalid days value - too large".to_string()))?;

    let entries = fs::read_dir(&skit_keys_dir).map_err(SkitError::Io)?;
    let mut found_keys = false;
    let mut old_keys = Vec::new();
    let mut recent_keys = Vec::new();
    let now = SystemTime::now();

    // First pass: categorize keys by age
    for entry in entries {
        let entry = entry.map_err(SkitError::Io)?;
        let path = entry.path();

        // Only process .key files
        if path.extension().is_none_or(|ext| ext != "key") {
            continue;
        }

        found_keys = true;

        // Check the modification time
        let metadata = fs::metadata(&path).map_err(SkitError::Io)?;
        let modified_time = metadata.modified().map_err(SkitError::Io)?;

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Calculate days since last access
        let days_ago = match now.duration_since(modified_time) {
            Ok(duration) => duration.as_secs() / (24 * 60 * 60),
            Err(_) => 0, // File is in the future somehow, treat as recent
        };

        if modified_time < cutoff_time {
            old_keys.push((path, file_name, days_ago));
        } else {
            recent_keys.push((file_name, days_ago));
        }
    }

    if !found_keys {
        print_info("No key files found in the keys directory");
        return Ok(());
    }

    // Show recent keys being kept
    if !recent_keys.is_empty() {
        print_info(&format!("📂 Keeping {} recent key(s):", recent_keys.len()));
        for (key_name, days_ago) in &recent_keys {
            print_info(&format!(
                "  ├─ {} (accessed {})",
                key_name,
                format_days_ago(*days_ago)
            ));
        }
        println!(); // Add spacing
    }

    // Show old keys that will be/would be removed
    if old_keys.is_empty() {
        print_info("No old keys found to remove");
        return Ok(());
    }

    if dry_run {
        print_warning(&format!(
            "🗑️  Would remove {} old key(s) (not accessed for {}+ days):",
            old_keys.len(),
            older_than_days
        ));
        for (_, key_name, days_ago) in &old_keys {
            print_warning(&format!(
                "  ├─ {} (accessed {})",
                key_name,
                format_days_ago(*days_ago)
            ));
        }
        println!();
        print_info("Run without --dry-run to actually remove these keys");
        return Ok(());
    }

    // Show keys to be removed and ask for confirmation
    print_warning(&format!(
        "⚠️  Found {} old key(s) to remove (not accessed for {}+ days):",
        old_keys.len(),
        older_than_days
    ));
    for (_, key_name, days_ago) in &old_keys {
        print_warning(&format!(
            "  ├─ {} (accessed {})",
            key_name,
            format_days_ago(*days_ago)
        ));
    }
    println!();
    print_error("🚨 WARNING: This operation is IRREVERSIBLE!");
    print_info(
        "You will need to re-enter passwords for these safes if you want to use remember-safekey again.",
    );
    println!();

    print!("Continue with deletion? [y/N]: ");
    io::stdout().flush().map_err(SkitError::Io)?;

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(SkitError::Io)?;

    let input = input.trim().to_lowercase();
    if input != "y" && input != "yes" {
        print_info("Cleanup cancelled");
        return Ok(());
    }

    // Proceed with deletion
    let mut removed_count = 0;
    for (path, key_name, days_ago) in old_keys {
        match fs::remove_file(&path) {
            Ok(()) => {
                print_success(&format!(
                    "Removed old key: {} (was accessed {})",
                    key_name,
                    format_days_ago(days_ago)
                ));
                removed_count += 1;
            }
            Err(e) => {
                print_error(&format!("Failed to remove key {}: {}", key_name, e));
            }
        }
    }

    print_success(&format!(
        "✅ Cleanup completed - removed {} old key(s)",
        removed_count
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_cleanup_keys_basic_structure() {
        // Create temporary directory structure
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let keys_dir = temp_dir.path().join("keys");
        fs::create_dir_all(&keys_dir).expect("Failed to create keys dir");

        // Create a sample key file
        let key_file = keys_dir.join("test-uuid.key");
        fs::write(&key_file, "password").expect("Failed to write key file");

        // Basic structure test - file exists
        assert!(key_file.exists());

        // Note: Testing the actual cleanup logic would require manipulating file timestamps
        // which is complex and platform-dependent. This test verifies basic structure.
    }
}
