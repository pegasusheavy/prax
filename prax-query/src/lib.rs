//! # prax-query
//!
//! Type-safe query builder for the Prax ORM.
//!
//! This crate provides the core query building functionality, including:
//! - Fluent API for building queries (`find_many`, `find_unique`, `create`, `update`, `delete`)
//! - Type-safe filtering with `where` clauses
//! - Sorting and pagination
//! - Relation loading (`include`, `select`)
//! - Transaction support
//!
//! ## Example
//!
//! ```rust,ignore
//! use prax_query::prelude::*;
//!
//! // Find many users with filtering
//! let users = client
//!     .user()
//!     .find_many()
//!     .where_(user::email::contains("@example.com"))
//!     .order_by(user::created_at::desc())
//!     .take(10)
//!     .exec()
//!     .await?;
//!
//! // Create a new user
//! let user = client
//!     .user()
//!     .create(user::Create {
//!         email: "new@example.com".into(),
//!         name: Some("New User".into()),
//!     })
//!     .exec()
//!     .await?;
//! ```

pub mod error;
pub mod filter;
pub mod operations;
pub mod pagination;
pub mod query;
pub mod relations;
pub mod sql;
pub mod traits;
pub mod transaction;
pub mod types;

pub use error::{QueryError, QueryResult};
pub use filter::{Filter, FilterValue, ScalarFilter};
pub use operations::{
    CreateOperation, DeleteOperation, FindManyOperation, FindUniqueOperation, UpdateOperation,
};
pub use pagination::{Cursor, CursorDirection, Pagination};
pub use query::QueryBuilder;
pub use relations::{Include, IncludeSpec, RelationLoader, RelationSpec, SelectSpec};
pub use traits::{Executable, IntoFilter, Model, QueryEngine};
pub use transaction::{Transaction, TransactionConfig, IsolationLevel};
pub use types::{OrderBy, Select, SortOrder};

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::error::{QueryError, QueryResult};
    pub use crate::filter::{Filter, FilterValue, ScalarFilter};
    pub use crate::operations::*;
    pub use crate::pagination::{Cursor, CursorDirection, Pagination};
    pub use crate::query::QueryBuilder;
    pub use crate::relations::{Include, IncludeSpec, RelationSpec, SelectSpec};
    pub use crate::traits::{Executable, IntoFilter, Model, QueryEngine};
    pub use crate::transaction::{Transaction, TransactionConfig, IsolationLevel};
    pub use crate::types::{OrderBy, Select, SortOrder};
}

