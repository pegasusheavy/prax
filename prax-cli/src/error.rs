//! CLI error types and result alias.

use miette::Diagnostic;
use thiserror::Error;

/// Result type alias for CLI operations
pub type CliResult<T> = Result<T, CliError>;

/// CLI error types
#[derive(Error, Debug, Diagnostic)]
pub enum CliError {
    /// IO error
    #[error("IO error: {0}")]
    #[diagnostic(code(prax::io))]
    Io(#[from] std::io::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    #[diagnostic(code(prax::config))]
    Config(String),

    /// Schema parsing error
    #[error("Schema error: {0}")]
    #[diagnostic(code(prax::schema))]
    Schema(String),

    /// Validation error
    #[error("Validation error: {0}")]
    #[diagnostic(code(prax::validation))]
    Validation(String),

    /// Migration error
    #[error("Migration error: {0}")]
    #[diagnostic(code(prax::migration))]
    Migration(String),

    /// Database error
    #[error("Database error: {0}")]
    #[diagnostic(code(prax::database))]
    Database(String),

    /// Command error
    #[error("Command error: {0}")]
    #[diagnostic(code(prax::command))]
    Command(String),

    /// Format error
    #[error("Format error: {0}")]
    #[diagnostic(code(prax::format))]
    Format(String),

    /// Code generation error
    #[error("Codegen error: {0}")]
    #[diagnostic(code(prax::codegen))]
    Codegen(String),
}

impl From<toml::de::Error> for CliError {
    fn from(err: toml::de::Error) -> Self {
        CliError::Config(format!("Failed to parse TOML: {}", err))
    }
}

impl From<toml::ser::Error> for CliError {
    fn from(err: toml::ser::Error) -> Self {
        CliError::Config(format!("Failed to serialize TOML: {}", err))
    }
}
