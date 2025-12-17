//! # prax-schema
//!
//! Schema parser and AST for the Prax ORM.
//!
//! This crate provides:
//! - Schema Definition Language (SDL) parser for `.prax` files
//! - Configuration parser for `prax.toml` files
//! - Abstract Syntax Tree (AST) types for schema representation
//! - Schema validation and semantic analysis
//!
//! ## Example
//!
//! ```rust,ignore
//! use prax_schema::{parse_schema, validate_schema, PraxConfig};
//!
//! // Parse and validate a schema
//! let schema = validate_schema(r#"
//!     model User {
//!         id    Int    @id @auto
//!         email String @unique
//!         name  String?
//!     }
//! "#)?;
//!
//! // Parse configuration
//! let config = PraxConfig::from_file("prax.toml")?;
//! ```

pub mod ast;
pub mod config;
pub mod error;
pub mod parser;
pub mod validator;

pub use ast::*;
pub use config::PraxConfig;
pub use error::{SchemaError, SchemaResult};
pub use parser::{parse_schema, parse_schema_file};
pub use validator::{Validator, validate_schema};
