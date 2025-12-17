//! # Prax
//!
//! A next-generation, type-safe ORM for Rust inspired by Prisma.
//!
//! Prax provides:
//! - A schema definition language for defining your data models
//! - Type-safe query builders with compile-time guarantees
//! - Async-first design built on Tokio
//! - Support for PostgreSQL, MySQL, SQLite, and MongoDB
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use prax::prelude::*;
//!
//! #[derive(Model)]
//! #[prax(table = "users")]
//! pub struct User {
//!     #[prax(id, auto)]
//!     pub id: i32,
//!     #[prax(unique)]
//!     pub email: String,
//!     pub name: Option<String>,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), prax::Error> {
//!     let client = PraxClient::new("postgresql://localhost/mydb").await?;
//!
//!     let users = client
//!         .user()
//!         .find_many()
//!         .where_(user::email::contains("@example.com"))
//!         .exec()
//!         .await?;
//!
//!     Ok(())
//! }
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

/// Schema parsing and AST types.
pub mod schema {
    pub use prax_schema::*;
}

// Re-export proc macros
pub use prax_codegen::prax_schema;
pub use prax_codegen::Model;

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::schema::{PraxConfig, Schema, parse_schema, parse_schema_file};
    pub use crate::{Model, prax_schema};
}

// Re-export key types at the crate root
pub use schema::{Schema, SchemaError};
