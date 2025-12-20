//! Compile-time checked query macros.
//!
//! This module provides macros that leverage SQLx's compile-time query checking
//! to ensure SQL queries are valid at compile time.
//!
//! # Setup
//!
//! To use compile-time checked queries:
//!
//! 1. Set the `DATABASE_URL` environment variable to your database connection string
//! 2. For offline mode (CI), run `cargo sqlx prepare` to generate query metadata
//! 3. Set `SQLX_OFFLINE=true` in CI environments
//!
//! # Examples
//!
//! ## Basic Query
//!
//! ```ignore
//! use prax_sqlx::query;
//!
//! // This query is checked at compile time
//! let users = query!("SELECT id, name, email FROM users WHERE active = $1", true)
//!     .fetch_all(&pool)
//!     .await?;
//! ```
//!
//! ## Typed Query
//!
//! ```ignore
//! use prax_sqlx::query_as;
//!
//! #[derive(sqlx::FromRow)]
//! struct User {
//!     id: i32,
//!     name: String,
//!     email: String,
//! }
//!
//! let users: Vec<User> = query_as!(User, "SELECT id, name, email FROM users")
//!     .fetch_all(&pool)
//!     .await?;
//! ```
//!
//! ## Scalar Query
//!
//! ```ignore
//! use prax_sqlx::query_scalar;
//!
//! let count: i64 = query_scalar!("SELECT COUNT(*) FROM users")
//!     .fetch_one(&pool)
//!     .await?;
//! ```

/// Re-export SQLx's query macro for compile-time checked queries.
///
/// This macro validates SQL queries at compile time by connecting to the database
/// specified in the `DATABASE_URL` environment variable.
///
/// # Example
///
/// ```ignore
/// use prax_sqlx::query;
///
/// let rows = query!("SELECT id, name FROM users WHERE id = $1", user_id)
///     .fetch_all(&pool)
///     .await?;
///
/// for row in rows {
///     println!("User: {} - {}", row.id, row.name);
/// }
/// ```
#[macro_export]
macro_rules! query {
    ($($args:tt)*) => {
        sqlx::query!($($args)*)
    };
}

/// Re-export SQLx's query_as macro for compile-time checked typed queries.
///
/// This macro maps query results to a struct at compile time.
///
/// # Example
///
/// ```ignore
/// use prax_sqlx::query_as;
///
/// #[derive(sqlx::FromRow)]
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// let users: Vec<User> = query_as!(User, "SELECT id, name FROM users")
///     .fetch_all(&pool)
///     .await?;
/// ```
#[macro_export]
macro_rules! query_as {
    ($($args:tt)*) => {
        sqlx::query_as!($($args)*)
    };
}

/// Re-export SQLx's query_scalar macro for compile-time checked scalar queries.
///
/// This macro is used for queries that return a single value.
///
/// # Example
///
/// ```ignore
/// use prax_sqlx::query_scalar;
///
/// let count: i64 = query_scalar!("SELECT COUNT(*) FROM users")
///     .fetch_one(&pool)
///     .await?;
/// ```
#[macro_export]
macro_rules! query_scalar {
    ($($args:tt)*) => {
        sqlx::query_scalar!($($args)*)
    };
}

/// Re-export SQLx's query_file macro for compile-time checked queries from files.
///
/// This macro loads and validates SQL from a file at compile time.
///
/// # Example
///
/// ```ignore
/// use prax_sqlx::query_file;
///
/// // Loads and validates sql/get_users.sql at compile time
/// let rows = query_file!("sql/get_users.sql")
///     .fetch_all(&pool)
///     .await?;
/// ```
#[macro_export]
macro_rules! query_file {
    ($($args:tt)*) => {
        sqlx::query_file!($($args)*)
    };
}

/// Re-export SQLx's query_file_as macro for compile-time checked typed queries from files.
///
/// # Example
///
/// ```ignore
/// use prax_sqlx::query_file_as;
///
/// #[derive(sqlx::FromRow)]
/// struct User {
///     id: i32,
///     name: String,
/// }
///
/// let users: Vec<User> = query_file_as!(User, "sql/get_users.sql")
///     .fetch_all(&pool)
///     .await?;
/// ```
#[macro_export]
macro_rules! query_file_as {
    ($($args:tt)*) => {
        sqlx::query_file_as!($($args)*)
    };
}

/// Re-export SQLx's query_file_scalar macro for compile-time checked scalar queries from files.
///
/// # Example
///
/// ```ignore
/// use prax_sqlx::query_file_scalar;
///
/// let count: i64 = query_file_scalar!("sql/count_users.sql")
///     .fetch_one(&pool)
///     .await?;
/// ```
#[macro_export]
macro_rules! query_file_scalar {
    ($($args:tt)*) => {
        sqlx::query_file_scalar!($($args)*)
    };
}

/// A macro for building type-safe queries with compile-time validation.
///
/// This combines Prax's query builder with SQLx's compile-time checking.
///
/// # Example
///
/// ```ignore
/// use prax_sqlx::prax_query;
///
/// // Build a query with the query builder, then validate it
/// let query = prax_query!(User, find_many)
///     .r#where(user::active::equals(true))
///     .order_by(user::created_at::desc())
///     .take(10);
///
/// let users = query.exec(&engine).await?;
/// ```
#[macro_export]
macro_rules! prax_query {
    ($model:ty, $operation:ident) => {
        <$model>::$operation()
    };
}

/// Macro to define a model with SQLx row derivation.
///
/// # Example
///
/// ```ignore
/// use prax_sqlx::define_model;
///
/// define_model! {
///     pub struct User {
///         pub id: i32,
///         pub name: String,
///         pub email: String,
///         pub active: bool,
///         pub created_at: chrono::DateTime<chrono::Utc>,
///     }
/// }
/// ```
#[macro_export]
macro_rules! define_model {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                $field_vis:vis $field:ident : $ty:ty
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
        $vis struct $name {
            $(
                $(#[$field_meta])*
                $field_vis $field: $ty
            ),*
        }
    };
}

