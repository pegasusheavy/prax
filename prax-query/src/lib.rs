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
//! - Raw SQL escape hatch
//! - Middleware system
//! - Multi-tenant support
//!
//! ## Filters
//!
//! Build type-safe filters for queries:
//!
//! ```rust
//! use prax_query::{Filter, FilterValue};
//!
//! // Equality filter
//! let filter = Filter::Equals("email".into(), FilterValue::String("test@example.com".into()));
//!
//! // Greater than filter
//! let filter = Filter::Gt("age".into(), FilterValue::Int(18));
//!
//! // Contains filter (for strings)
//! let filter = Filter::Contains("name".into(), FilterValue::String("john".into()));
//!
//! // Combine filters with AND/OR
//! let combined = Filter::and([
//!     Filter::Equals("active".into(), FilterValue::Bool(true)),
//!     Filter::Gt("age".into(), FilterValue::Int(18)),
//! ]);
//!
//! let either = Filter::or([
//!     Filter::Equals("role".into(), FilterValue::String("admin".into())),
//!     Filter::Equals("role".into(), FilterValue::String("moderator".into())),
//! ]);
//! ```
//!
//! ## Filter Values
//!
//! Convert Rust types to filter values:
//!
//! ```rust
//! use prax_query::FilterValue;
//!
//! // Integer values
//! let val: FilterValue = 42.into();
//! assert!(matches!(val, FilterValue::Int(42)));
//!
//! // String values
//! let val: FilterValue = "hello".into();
//! assert!(matches!(val, FilterValue::String(_)));
//!
//! // Boolean values
//! let val: FilterValue = true.into();
//! assert!(matches!(val, FilterValue::Bool(true)));
//!
//! // Float values
//! let val: FilterValue = 3.14f64.into();
//! assert!(matches!(val, FilterValue::Float(_)));
//!
//! // Null values
//! let val = FilterValue::Null;
//! ```
//!
//! ## Sorting
//!
//! Build sort specifications:
//!
//! ```rust
//! use prax_query::{OrderBy, OrderByField, NullsOrder};
//!
//! // Ascending order
//! let order = OrderByField::asc("created_at");
//!
//! // Descending order
//! let order = OrderByField::desc("updated_at");
//!
//! // With NULLS FIRST/LAST
//! let order = OrderByField::asc("name").nulls(NullsOrder::First);
//! let order = OrderByField::desc("score").nulls(NullsOrder::Last);
//!
//! // Combine multiple orderings
//! let orders = OrderBy::Field(OrderByField::asc("name"))
//!     .then(OrderByField::desc("created_at"));
//! ```
//!
//! ## Raw SQL
//!
//! Build raw SQL queries with parameter binding:
//!
//! ```rust
//! use prax_query::Sql;
//!
//! // Simple query
//! let sql = Sql::new("SELECT * FROM users");
//! assert_eq!(sql.sql(), "SELECT * FROM users");
//!
//! // Query with parameter - bind appends placeholder
//! let sql = Sql::new("SELECT * FROM users WHERE id = ")
//!     .bind(42);
//! assert_eq!(sql.params().len(), 1);
//! ```
//!
//! ## Connection Strings
//!
//! Parse database connection strings:
//!
//! ```rust
//! use prax_query::ConnectionString;
//!
//! // PostgreSQL
//! let conn = ConnectionString::parse("postgres://user:pass@localhost:5432/mydb").unwrap();
//! assert_eq!(conn.host(), Some("localhost"));
//! assert_eq!(conn.port(), Some(5432));
//! assert_eq!(conn.database(), Some("mydb"));
//!
//! // MySQL
//! let conn = ConnectionString::parse("mysql://user:pass@localhost:3306/mydb").unwrap();
//! ```
//!
//! ## Transaction Config
//!
//! Configure transaction behavior:
//!
//! ```rust
//! use prax_query::IsolationLevel;
//!
//! let level = IsolationLevel::Serializable;
//! assert_eq!(level.as_sql(), "SERIALIZABLE");
//! ```
//!
//! ## Error Handling
//!
//! Work with query errors:
//!
//! ```rust
//! use prax_query::{QueryError, ErrorCode};
//!
//! // Create errors
//! let err = QueryError::not_found("User");
//! assert_eq!(err.code, ErrorCode::RecordNotFound);
//! ```

pub mod batch;
pub mod cache;
pub mod connection;
pub mod data;
pub mod error;
pub mod filter;
pub mod intern;
pub mod lazy;
pub mod logging;
#[macro_use]
pub mod macros;
pub mod memory;
pub mod middleware;
pub mod nested;
pub mod operations;
pub mod pagination;
pub mod pool;
pub mod query;
pub mod raw;
pub mod relations;
pub mod sql;
pub mod static_filter;
pub mod tenant;
pub mod traits;
pub mod transaction;
pub mod typed_filter;
pub mod types;

pub use error::{QueryError, QueryResult, ErrorCode, ErrorContext, Suggestion};
pub use filter::{
    Filter, FilterValue, ScalarFilter, FieldName, ValueList, SmallValueList, LargeValueList,
    AndFilterBuilder, OrFilterBuilder, FluentFilterBuilder,
};
pub use nested::{NestedWrite, NestedWriteBuilder, NestedWriteOperations};
pub use operations::{
    CreateOperation, DeleteOperation, FindManyOperation, FindUniqueOperation, UpdateOperation,
};
pub use pagination::{Cursor, CursorDirection, Pagination};
pub use query::QueryBuilder;
pub use raw::{Sql, RawQueryOperation, RawExecuteOperation};
pub use relations::{Include, IncludeSpec, RelationLoader, RelationSpec, SelectSpec};
pub use traits::{Executable, IntoFilter, Model, QueryEngine};
pub use transaction::{Transaction, TransactionConfig, IsolationLevel};
pub use types::{NullsOrder, OrderBy, OrderByBuilder, OrderByField, Select, SortOrder, order_patterns};

// Re-export middleware types
pub use middleware::{
    LoggingMiddleware, MetricsMiddleware, TimingMiddleware, RetryMiddleware,
    Middleware, MiddlewareStack, MiddlewareChain, MiddlewareBuilder,
    QueryContext, QueryMetadata, QueryType, QueryMetrics,
};

// Re-export connection types
pub use connection::{
    ConnectionString, DatabaseConfig, Driver, MultiDatabaseConfig,
    ConnectionOptions, PoolOptions, PoolConfig, SslMode, SslConfig,
    EnvExpander, ConnectionError,
};

// Re-export data types
pub use data::{
    CreateData, UpdateData, DataBuilder, FieldValue, ConnectData,
    BatchCreate, IntoData,
};

// Re-export tenant types
pub use tenant::{
    TenantContext, TenantId, TenantInfo, TenantConfig, TenantConfigBuilder,
    TenantMiddleware, TenantResolver, StaticResolver, DynamicResolver,
    IsolationStrategy, RowLevelConfig, SchemaConfig,
};

// Re-export intern types
pub use intern::{intern, intern_cow, clear_interned, interned_count, fields};

// Re-export pool types
pub use pool::{FilterPool, FilterBuilder, PooledFilter, PooledValue, IntoPooledValue};

// Re-export SQL builder types
pub use sql::{DatabaseType, SqlBuilder, FastSqlBuilder, QueryCapacity, templates};

// Re-export cache types
pub use cache::{
    QueryCache, QueryKey, CachedQuery, CacheStats, QueryHash, patterns as cache_patterns,
    SqlTemplateCache, SqlTemplate, global_template_cache, register_global_template,
    get_global_template, precompute_query_hash,
};

// Re-export batch types
pub use batch::{Batch, BatchBuilder, BatchOperation, BatchResult, OperationResult};

// Re-export lazy loading types
pub use lazy::{Lazy, LazyRelation, OneToManyLoader, ManyToOneLoader};

// Re-export static filter utilities
pub use static_filter::{
    eq, ne, lt, lte, gt, gte, is_null, is_not_null,
    contains, starts_with, ends_with, in_list, not_in_list,
    and2, and3, and4, and5, or2, or3, or4, or5, not,
    StaticFilter, CompactValue,
    fields as static_fields,
};

// Re-export typed filter utilities
pub use typed_filter::{
    TypedFilter, DirectSql, And, Or, Not as TypedNot,
    Eq, Ne, Lt, Lte, Gt, Gte,
    IsNull, IsNotNull, Contains, StartsWith, EndsWith,
    AndN, OrN, LazyFilter, Maybe,
    and_n, or_n, lazy,
    eq as typed_eq, ne as typed_ne, lt as typed_lt, lte as typed_lte,
    gt as typed_gt, gte as typed_gte,
    is_null as typed_is_null, is_not_null as typed_is_not_null,
};

// Re-export memory optimization utilities
pub use memory::{
    StringPool, BufferPool, PooledBuffer, PoolStats,
    CompactFilter, MemoryStats,
    GLOBAL_STRING_POOL, GLOBAL_BUFFER_POOL,
    intern as memory_intern, get_buffer,
};

// Re-export logging utilities
pub use logging::{
    is_debug_enabled, get_log_level, get_log_format,
    init as init_logging, init_with_level, init_debug,
};

// Re-export smallvec for macros
pub use smallvec;

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::error::{QueryError, QueryResult};
    pub use crate::filter::{Filter, FilterValue, ScalarFilter};
    pub use crate::nested::{NestedWrite, NestedWriteBuilder, NestedWriteOperations};
    pub use crate::operations::*;
    pub use crate::pagination::{Cursor, CursorDirection, Pagination};
    pub use crate::query::QueryBuilder;
    pub use crate::raw::{Sql, RawQueryOperation, RawExecuteOperation};
    pub use crate::relations::{Include, IncludeSpec, RelationSpec, SelectSpec};
    pub use crate::traits::{Executable, IntoFilter, Model, QueryEngine};
    pub use crate::transaction::{Transaction, TransactionConfig, IsolationLevel};
    pub use crate::types::{OrderBy, Select, SortOrder};
    pub use crate::raw_query;

    // Tenant types
    pub use crate::tenant::{TenantContext, TenantConfig, TenantMiddleware, IsolationStrategy};
}

