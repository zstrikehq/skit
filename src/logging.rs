use std::env;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize the tracing subscriber for SKIT
///
/// This sets up colored output for terminals with automatic detection of:
/// - NO_COLOR environment variable (disables colors)
/// - TTY detection (no colors when piped)
/// - RUST_LOG environment variable for filtering
pub fn init_logging() {
    // Check if we should force colors or respect NO_COLOR
    let use_ansi = should_use_colors();

    // Set up the env filter - defaults to "info" if RUST_LOG is not set
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_ansi(use_ansi)
                .with_target(false) // Don't show module paths for cleaner output
                .with_thread_ids(false) // Don't show thread IDs for CLI tool
                .with_file(false) // Don't show file paths for cleaner output
                .with_line_number(false) // Don't show line numbers for cleaner output
                .without_time() // Remove timestamps for cleaner CLI output
                .compact(), // Use compact format for CLI tools
        )
        .with(env_filter)
        .init();
}

/// Determine if we should use ANSI colors based on environment and TTY detection
fn should_use_colors() -> bool {
    // Check NO_COLOR standard first
    if env::var("NO_COLOR").is_ok() {
        return false;
    }

    // Check SKIT-specific override
    if env::var("SKIT_NO_COLOR").is_ok() {
        return false;
    }

    // Check for force color
    if env::var("FORCE_COLOR").is_ok() || env::var("SKIT_FORCE_COLOR").is_ok() {
        return true;
    }

    // Default to true - tracing-subscriber will handle TTY detection
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_color_detection() {
        // Test NO_COLOR environment variable
        env::set_var("NO_COLOR", "1");
        assert_eq!(should_use_colors(), false);
        env::remove_var("NO_COLOR");

        // Test SKIT_NO_COLOR environment variable
        env::set_var("SKIT_NO_COLOR", "1");
        assert_eq!(should_use_colors(), false);
        env::remove_var("SKIT_NO_COLOR");

        // Test FORCE_COLOR environment variable
        env::set_var("FORCE_COLOR", "1");
        assert_eq!(should_use_colors(), true);
        env::remove_var("FORCE_COLOR");

        // Test SKIT_FORCE_COLOR environment variable
        env::set_var("SKIT_FORCE_COLOR", "1");
        assert_eq!(should_use_colors(), true);
        env::remove_var("SKIT_FORCE_COLOR");
    }
}
