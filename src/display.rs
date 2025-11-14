use crate::error::SkitError;

// Legacy print functions that now use tracing
// These are kept for backward compatibility but redirect to tracing macros
pub fn print_success(message: &str) {
    tracing::info!("✓ {}", message);
}

pub fn print_info(message: &str) {
    tracing::info!("{}", message);
}

pub fn print_warning(message: &str) {
    tracing::warn!("{}", message);
}

pub fn print_error(message: &str) {
    tracing::error!("{}", message);
}

pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if text.len() <= max_width {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + word.len() < max_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }

        if current_line.len() > max_width {
            let word = current_line;
            current_line = String::new();

            for chunk in word.chars().collect::<Vec<_>>().chunks(max_width) {
                let chunk_str: String = chunk.iter().collect();
                lines.push(chunk_str);
            }
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(text.to_string());
    }

    lines
}

pub fn print_grouped(items: &[(String, String, bool)]) {
    if items.is_empty() {
        println!("No items in safe");
        return;
    }

    let encrypted_items: Vec<_> = items
        .iter()
        .filter(|(_, _, is_encrypted)| *is_encrypted)
        .collect();
    let plain_items: Vec<_> = items
        .iter()
        .filter(|(_, _, is_encrypted)| !*is_encrypted)
        .collect();

    if !encrypted_items.is_empty() {
        println!("🔒 ENCRYPTED SECRETS ({})", encrypted_items.len());
        for (i, (key, value, _)) in encrypted_items.iter().enumerate() {
            let is_last = i == encrypted_items.len() - 1;
            let prefix = if is_last { "└─" } else { "├─" };

            let wrapped_lines = wrap_text(value, 80);
            if wrapped_lines.len() == 1 {
                println!("{} {}: {}", prefix, key, wrapped_lines[0]);
            } else {
                println!("{} {}:", prefix, key);
                for line in wrapped_lines.iter() {
                    let line_prefix = if is_last { "    " } else { "│   " };
                    let bullet = "  ";
                    println!("{}{}{}", line_prefix, bullet, line);
                }
            }
        }
        if !plain_items.is_empty() {
            println!();
        }
    }

    if !plain_items.is_empty() {
        println!("📝 PLAIN TEXT VALUES ({})", plain_items.len());
        for (i, (key, value, _)) in plain_items.iter().enumerate() {
            let is_last = i == plain_items.len() - 1;
            let prefix = if is_last { "└─" } else { "├─" };

            let wrapped_lines = wrap_text(value, 80);
            if wrapped_lines.len() == 1 {
                println!("{} {}: {}", prefix, key, wrapped_lines[0]);
            } else {
                println!("{} {}:", prefix, key);
                for line in wrapped_lines.iter() {
                    let line_prefix = if is_last { "    " } else { "│   " };
                    let bullet = "  ";
                    println!("{}{}{}", line_prefix, bullet, line);
                }
            }
        }
    }
}

pub fn print_keys_table(items: &[(String, bool)]) {
    if items.is_empty() {
        println!("No keys in safe");
        return;
    }

    let key_width = items.iter().map(|(k, _)| k.len()).max().unwrap_or(3).max(3);
    let type_width = 4; // "Type" header width

    println!(
        "{:-<width$}-+-{:-<twidth$}-",
        "",
        "",
        width = key_width,
        twidth = type_width
    );

    println!(
        " {:^width$} | {:^twidth$} ",
        "Key",
        "Type",
        width = key_width,
        twidth = type_width
    );

    println!(
        "{:-<width$}-+-{:-<twidth$}-",
        "",
        "",
        width = key_width,
        twidth = type_width
    );

    for (key, is_encrypted) in items {
        let type_str = if *is_encrypted { "ENC" } else { "PLAIN" };
        println!(
            " {:width$} | {:^twidth$} ",
            key,
            type_str,
            width = key_width,
            twidth = type_width
        );
    }

    println!(
        "{:-<width$}-+-{:-<twidth$}-",
        "",
        "",
        width = key_width,
        twidth = type_width
    );
}

pub fn wrap_with_quotes(value: &str) -> String {
    if value.is_empty() {
        return "\"\"".to_string();
    }

    // Use double quotes and escape any double quotes in the value
    let escaped = value.replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

pub fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    // Check if quoting is needed
    let needs_quoting = value.chars().any(|c| {
        matches!(
            c,
            ' ' | '\t'
                | '\n'
                | '\r'
                | '"'
                | '\''
                | '\\'
                | '$'
                | '`'
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | '|'
                | '&'
                | ';'
                | '<'
                | '>'
                | '*'
                | '?'
                | '~'
        )
    });

    if !needs_quoting {
        return value.to_string();
    }

    // Use single quotes and escape any single quotes in the value
    let escaped = value.replace('\'', "'\"'\"'");
    format!("'{}'", escaped)
}

pub fn format_json_output<T: serde::Serialize>(data: &T) -> Result<String, SkitError> {
    serde_json::to_string_pretty(data)
        .map_err(|e| SkitError::ParseError(format!("JSON serialization error: {}", e)))
}

pub fn print_terraform_output(items: &[(String, String, bool)]) {
    if items.is_empty() {
        println!("No items in safe");
        return;
    }

    for (key, value, _) in items.iter() {
        println!("{} = {}", key, wrap_with_quotes(value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_text_short() {
        let result = wrap_text("short text", 100);
        assert_eq!(result, vec!["short text"]);
    }

    #[test]
    fn test_wrap_text_word_boundaries() {
        let text = "This is a very long line that should be wrapped at word boundaries for better readability";
        let result = wrap_text(text, 30);

        assert!(result.len() > 1);
        for line in &result {
            assert!(line.len() <= 30);
        }

        // Verify all words are preserved when joined back
        let joined = result.join(" ");
        assert_eq!(joined, text);
    }

    #[test]
    fn test_wrap_text_long_word() {
        let text = "supercalifragilisticexpialidocious";
        let result = wrap_text(text, 10);

        assert!(result.len() > 1);
        for line in &result {
            assert!(line.len() <= 10);
        }
    }

    #[test]
    fn test_wrap_text_exact_length() {
        let text = format!("exactly100chars{}", "0".repeat(85)); // 15 + 85 = 100 chars
        let result = wrap_text(&text, 100);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], text);
    }
}
