use std::path::Path;

#[derive(Debug, Clone)]
pub struct ShellInfo {
    pub name: String,
}

pub fn detect_current_shell() -> String {
    // Method 1: Check shell-specific environment variables
    if std::env::var("BASH_VERSION").is_ok() || std::env::var("BASH").is_ok() {
        return "bash".to_string();
    }
    if std::env::var("ZSH_VERSION").is_ok() {
        return "zsh".to_string();
    }
    if std::env::var("FISH_VERSION").is_ok() {
        return "fish".to_string();
    }
    if std::env::var("NU_VERSION").is_ok() {
        return "nu".to_string();
    }

    // Method 2: Check parent process name via /proc/self/stat (Linux only)
    #[cfg(target_os = "linux")]
    {
        // Try to read parent process info, but don't panic if it fails
        match std::fs::read_to_string("/proc/self/stat") {
            Ok(stat_content) => {
                let fields: Vec<&str> = stat_content.split_whitespace().collect();
                if fields.len() > 3
                    && let Ok(ppid) = fields[3].parse::<u32>()
                {
                    let parent_comm_path = format!("/proc/{}/comm", ppid);
                    if let Ok(parent_name) = std::fs::read_to_string(&parent_comm_path) {
                        let parent_name = parent_name.trim().to_lowercase();
                        if parent_name == "bash"
                            || parent_name == "zsh"
                            || parent_name == "fish"
                            || parent_name == "csh"
                            || parent_name == "tcsh"
                            || parent_name == "ksh"
                            || parent_name == "nu"
                        {
                            return parent_name;
                        }
                    }
                }
            }
            Err(_) => {
                // /proc might not be available or accessible, ignore and continue
            }
        }
    }

    // Method 3: Fall back to $SHELL environment variable
    let shell_path = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    Path::new(&shell_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("sh")
        .to_string()
}

pub fn detect_shell() -> ShellInfo {
    // Check for Windows environment first
    if cfg!(target_os = "windows") {
        // Check if running in PowerShell
        if std::env::var("PSVersionTable").is_ok()
            || std::env::var("POWERSHELL_DISTRIBUTION_CHANNEL").is_ok()
        {
            return ShellInfo {
                name: "powershell".to_string(),
            };
        }
        // Default to cmd on Windows
        return ShellInfo {
            name: "cmd".to_string(),
        };
    }

    // Unix-like systems: Try to detect current shell, not default shell
    let current_shell = detect_current_shell();
    let shell_name = current_shell.to_lowercase();

    match shell_name.as_str() {
        "bash" => ShellInfo {
            name: "bash".to_string(),
        },
        "zsh" => ShellInfo {
            name: "zsh".to_string(),
        },
        "fish" => ShellInfo {
            name: "fish".to_string(),
        },
        "nu" => ShellInfo {
            name: "nu".to_string(),
        },
        "csh" | "tcsh" => ShellInfo { name: shell_name },
        "ksh" => ShellInfo {
            name: "ksh".to_string(),
        },
        _ => ShellInfo {
            name: "sh".to_string(),
        },
    }
}
