use crate::crypto;
#[cfg(windows)]
use crate::display::print_warning;
use crate::error::SkitError;
use crate::password::get_password_with_auth_chain;
use crate::types::Safe;
use std::collections::HashMap;
use std::process::Command;

pub fn exec(safe_path: &str, command_args: &[String]) -> Result<(), SkitError> {
    if command_args.is_empty() {
        return Err(SkitError::EmptyCommand);
    }

    // Show Windows warning
    #[cfg(windows)]
    {
        print_warning("For optimal experience on Windows, consider using 'skit env' instead:");
        print_warning("  PowerShell: skit env | Invoke-Expression");
        print_warning("  Command: for /f \"tokens=*\" %i in ('skit env') do %i");
    }

    let env_vars = prepare_environment(safe_path)?;

    #[cfg(unix)]
    {
        exec_replace_process(command_args, &env_vars); // Never returns
    }

    #[cfg(not(unix))]
    {
        exec_spawn_and_wait(command_args, &env_vars); // Never returns
    }
}

fn prepare_environment(safe_path: &str) -> Result<HashMap<String, String>, SkitError> {
    let safe = Safe::load(safe_path)?;

    if safe.items.is_empty() {
        return Ok(HashMap::new());
    }

    let mut env_vars = HashMap::new();
    let mut has_encrypted = false;

    // First pass: check if we have any encrypted secrets
    for item in safe.items.values() {
        if item.is_encrypted {
            has_encrypted = true;
            break;
        }
    }

    // Only prompt for password if we have encrypted secrets
    let password = if has_encrypted {
        Some(get_password_with_auth_chain(
            &safe,
            safe_path,
            "Enter safe password: ",
        )?)
    } else {
        None
    };

    // Second pass: decrypt and collect all values
    for item in safe.items.values() {
        let value = if item.is_encrypted {
            if let Some(ref pwd) = password {
                match crypto::DecryptBuilder::new()
                    .ciphertext(&item.value)
                    .password(pwd)
                    .decrypt()
                {
                    Ok(v) => v,
                    Err(_) => {
                        eprintln!("Warning: Failed to decrypt '{}', skipping", item.key);
                        continue;
                    }
                }
            } else {
                eprintln!(
                    "Warning: No password provided for encrypted key '{}', skipping",
                    item.key
                );
                continue;
            }
        } else {
            item.value.clone()
        };
        env_vars.insert(item.key.clone(), value);
    }

    Ok(env_vars)
}

#[cfg(unix)]
fn exec_replace_process(command_args: &[String], env_vars: &HashMap<String, String>) -> ! {
    use std::os::unix::process::CommandExt;

    let program = &command_args[0];
    let args = &command_args[1..];

    let mut cmd = Command::new(program);
    cmd.args(args);

    // Inherit current environment and add/override with safe variables
    for (key, value) in std::env::vars() {
        cmd.env(key, value);
    }

    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    // Use exec() to replace the current process entirely
    let err = cmd.exec(); // This never returns on success

    // If we get here, exec failed
    eprintln!("Failed to execute '{}': {}", program, err);
    std::process::exit(127); // Standard exit code for "command not found"
}

#[cfg(not(unix))]
fn exec_spawn_and_wait(command_args: &[String], env_vars: &HashMap<String, String>) -> ! {
    let program = &command_args[0];
    let args = &command_args[1..];

    let mut cmd = Command::new(program);
    cmd.args(args);

    // Inherit current environment and add/override with safe variables
    for (key, value) in std::env::vars() {
        cmd.env(key, value);
    }

    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    match cmd.status() {
        Ok(status) => {
            // Exit with the same code as the child process
            std::process::exit(status.code().unwrap_or(1));
        }
        Err(e) => {
            eprintln!("Failed to execute '{}': {}", program, e);
            std::process::exit(127); // Standard exit code for "command not found"
        }
    }
}
