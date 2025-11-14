use crossterm::{
    cursor,
    event::{Event, KeyCode, KeyEvent, KeyModifiers, read},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::{self, Write, stdout};
use std::process;
use zeroize::Zeroizing;

/// Read a password with visual masking (shows asterisks) using crossterm
pub fn prompt_password_masked(prompt: &str) -> Result<String, io::Error> {
    print!("{}", prompt);
    stdout().flush()?;

    enable_raw_mode()?;

    let mut password = Zeroizing::new(String::new());
    let result = read_password_chars(&mut password);

    let _ = disable_raw_mode();

    match result {
        Ok(()) => {
            println!();
            // Extract the string to return, original will be zeroized on drop
            Ok(password.to_string())
        }
        Err(e) => Err(e),
    }
}

fn read_password_chars(password: &mut String) -> Result<(), io::Error> {
    loop {
        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = read()?
        {
            match code {
                KeyCode::Enter => break,

                KeyCode::Backspace => {
                    if !password.is_empty() {
                        password.pop();
                        execute!(
                            stdout(),
                            cursor::MoveLeft(1),
                            crossterm::style::Print(" "),
                            cursor::MoveLeft(1)
                        )?;
                    }
                }

                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                    // Ensure terminal is restored, then exit immediately
                    let _ = disable_raw_mode();
                    println!();
                    process::exit(130);
                }

                KeyCode::Char(c) => {
                    password.push(c);
                    print!("*");
                    stdout().flush()?;
                }

                _ => {}
            }
        }
    }

    Ok(())
}

/// Main function for password prompts - uses crossterm with fallback
pub fn prompt_password_with_fallback(prompt: &str) -> Result<String, io::Error> {
    match prompt_password_masked(prompt) {
        Ok(password) => Ok(password),
        Err(_e) => {
            // If crossterm fails, provide a basic fallback
            eprintln!("Note: Visual masking unavailable, using secure input mode");

            // Simple fallback without masking
            print!("{}", prompt);
            stdout().flush()?;

            let mut input = Zeroizing::new(String::new());
            io::stdin().read_line(&mut input)?;

            // Remove trailing newline
            if input.ends_with('\n') {
                input.pop();
                if input.ends_with('\r') {
                    input.pop();
                }
            }

            Ok(input.to_string())
        }
    }
}
