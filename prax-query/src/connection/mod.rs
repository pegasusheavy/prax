//! Connection string parsing and multi-database configuration.
//!
//! This module provides utilities for parsing database connection URLs and
//! managing configurations for multiple database backends.
//!
//! # Supported URL Formats
//!
//! ## PostgreSQL
//! ```text
//! postgres://user:password@host:port/database?options
//! postgresql://user:password@host:port/database?options
//! ```
//!
//! ## MySQL
//! ```text
//! mysql://user:password@host:port/database?options
//! mariadb://user:password@host:port/database?options
//! ```
//!
//! ## SQLite
//! ```text
//! sqlite://path/to/database.db?options
//! sqlite::memory:
//! file:path/to/database.db?options
//! ```
//!
//! # Parsing Connection URLs
//!
//! ```rust
//! use prax_query::ConnectionString;
//!
//! // PostgreSQL URL
//! let conn = ConnectionString::parse("postgres://user:pass@localhost:5432/mydb").unwrap();
//! assert_eq!(conn.host(), Some("localhost"));
//! assert_eq!(conn.port(), Some(5432));
//! assert_eq!(conn.database(), Some("mydb"));
//!
//! // MySQL URL
//! let conn = ConnectionString::parse("mysql://user:pass@localhost:3306/mydb").unwrap();
//!
//! // SQLite URL (note: uses :// prefix)
//! let conn = ConnectionString::parse("sqlite://./data.db").unwrap();
//! ```
//!
//! # Driver Types
//!
//! ```rust
//! use prax_query::Driver;
//!
//! // Default ports
//! assert_eq!(Driver::Postgres.default_port(), Some(5432));
//! assert_eq!(Driver::MySql.default_port(), Some(3306));
//! assert_eq!(Driver::Sqlite.default_port(), None);
//!
//! // Parse from scheme
//! assert_eq!(Driver::from_scheme("postgres").unwrap(), Driver::Postgres);
//! assert_eq!(Driver::from_scheme("postgresql").unwrap(), Driver::Postgres);
//! assert_eq!(Driver::from_scheme("mysql").unwrap(), Driver::MySql);
//! assert_eq!(Driver::from_scheme("mariadb").unwrap(), Driver::MySql);
//! assert_eq!(Driver::from_scheme("sqlite").unwrap(), Driver::Sqlite);
//! ```
//!
//! # SSL Modes
//!
//! ```rust
//! use prax_query::connection::SslMode;
//!
//! // Available SSL modes
//! let mode = SslMode::Disable;   // No SSL
//! let mode = SslMode::Prefer;    // Use SSL if available
//! let mode = SslMode::Require;   // Require SSL
//! let mode = SslMode::VerifyCa;  // Verify CA certificate
//! let mode = SslMode::VerifyFull; // Verify CA and hostname
//! ```

mod config;
mod env;
mod options;
mod parser;
mod pool;

pub use config::{DatabaseConfig, DatabaseConfigBuilder, MultiDatabaseConfig};
pub use env::{EnvExpander, EnvSource};
pub use options::{
    ConnectionOptions, PoolOptions, PostgresOptions, MySqlOptions, SqliteOptions,
    SslMode, SslConfig,
};
pub use parser::{ConnectionString, Driver, ParsedUrl};
pub use pool::PoolConfig;

use thiserror::Error;

/// Errors that can occur during connection string parsing.
#[derive(Error, Debug)]
pub enum ConnectionError {
    /// Invalid URL format.
    #[error("Invalid connection URL: {0}")]
    InvalidUrl(String),

    /// Unknown database driver.
    #[error("Unknown database driver: {0}")]
    UnknownDriver(String),

    /// Missing required field.
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Invalid option value.
    #[error("Invalid option '{key}': {message}")]
    InvalidOption { key: String, message: String },

    /// Environment variable not found.
    #[error("Environment variable not found: {0}")]
    EnvNotFound(String),

    /// Invalid environment variable value.
    #[error("Invalid environment variable '{name}': {message}")]
    InvalidEnvValue { name: String, message: String },

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),
}

/// Result type for connection operations.
pub type ConnectionResult<T> = Result<T, ConnectionError>;

