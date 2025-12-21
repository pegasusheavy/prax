//! Styled terminal output utilities.

use owo_colors::OwoColorize;

/// Print a header/title
pub fn header(text: &str) {
    println!();
    println!("{}", text.bold().cyan());
    println!("{}", "─".repeat(text.len()).dimmed());
    println!();
}

/// Print the Prax logo
pub fn logo() {
    let logo = r#"
    ██████╗ ██████╗  █████╗ ██╗  ██╗
    ██╔══██╗██╔══██╗██╔══██╗╚██╗██╔╝
    ██████╔╝██████╔╝███████║ ╚███╔╝
    ██╔═══╝ ██╔══██╗██╔══██║ ██╔██╗
    ██║     ██║  ██║██║  ██║██╔╝ ██╗
    ╚═╝     ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝
    "#;
    println!("{}", logo.bright_cyan().bold());
}

/// Print a section header
pub fn section(text: &str) {
    println!("{}", text.bold().white());
}

/// Print a key-value pair
pub fn kv(key: &str, value: &str) {
    println!("  {}: {}", key.dimmed(), value);
}

/// Print a success message
pub fn success(text: &str) {
    println!("{} {}", "✔".green().bold(), text.green());
}

/// Print an info message
pub fn info(text: &str) {
    println!("{} {}", "ℹ".blue().bold(), text);
}

/// Print a warning message
pub fn warn(text: &str) {
    println!("{} {}", "⚠".yellow().bold(), text.yellow());
}

/// Print an error message
pub fn error(text: &str) {
    eprintln!("{} {}", "✖".red().bold(), text.red());
}

/// Print a step indicator
pub fn step(current: usize, total: usize, text: &str) {
    println!("{} {}", format!("[{}/{}]", current, total).dimmed(), text);
}

/// Print a list header
pub fn list(text: &str) {
    println!("{}", text);
}

/// Print a list item
pub fn list_item(text: &str) {
    println!("  {} {}", "•".dimmed(), text);
}

/// Print a numbered list item
pub fn numbered_item(number: usize, text: &str) {
    println!("  {}. {}", number.to_string().dimmed(), text);
}

/// Print a newline
pub fn newline() {
    println!();
}

/// Print dimmed text
pub fn dim(text: &str) {
    println!("{}", text.dimmed());
}

/// Print code block with syntax highlighting hint
pub fn code(code: &str, _language: &str) {
    println!();
    for line in code.lines() {
        println!("  {}", line.bright_white());
    }
    println!();
}

/// Style text as success (green)
pub fn style_success(text: &str) -> String {
    text.green().to_string()
}

/// Style text as pending (yellow)
pub fn style_pending(text: &str) -> String {
    text.yellow().to_string()
}

/// Style text as error (red)
pub fn style_error(text: &str) -> String {
    text.red().to_string()
}

/// Ask for confirmation
pub fn confirm(prompt: &str) -> bool {
    use std::io::{self, Write};

    print!("{} {} ", prompt, "[y/N]".dimmed());
    io::stdout().flush().ok();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }

    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Ask for text input
pub fn input(prompt: &str) -> Option<String> {
    use std::io::{self, Write};

    print!("{}: ", prompt);
    io::stdout().flush().ok();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return None;
    }

    let trimmed = input.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Ask user to select from options
pub fn select(prompt: &str, options: &[&str]) -> Option<usize> {
    use std::io::{self, Write};

    println!("{}", prompt);
    for (i, option) in options.iter().enumerate() {
        println!("  {} {}", format!("{})", i + 1).dimmed(), option);
    }

    print!("{}: ", "Select".dimmed());
    io::stdout().flush().ok();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return None;
    }

    input
        .trim()
        .parse::<usize>()
        .ok()
        .filter(|&n| n > 0 && n <= options.len())
        .map(|n| n - 1)
}
