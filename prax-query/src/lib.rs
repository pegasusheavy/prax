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
pub mod row;
pub mod sql;
pub mod static_filter;
pub mod tenant;
pub mod traits;
pub mod transaction;
pub mod typed_filter;
pub mod types;

pub use error::{ErrorCode, ErrorContext, QueryError, QueryResult, Suggestion};
pub use filter::{
    AndFilterBuilder, FieldName, Filter, FilterValue, FluentFilterBuilder, LargeValueList,
    OrFilterBuilder, ScalarFilter, SmallValueList, ValueList,
};
pub use nested::{NestedWrite, NestedWriteBuilder, NestedWriteOperations};
pub use operations::{
    CreateOperation, DeleteOperation, FindManyOperation, FindUniqueOperation, UpdateOperation,
};
pub use pagination::{Cursor, CursorDirection, Pagination};
pub use query::QueryBuilder;
pub use raw::{RawExecuteOperation, RawQueryOperation, Sql};
pub use relations::{Include, IncludeSpec, RelationLoader, RelationSpec, SelectSpec};
pub use traits::{Executable, IntoFilter, Model, QueryEngine};
pub use transaction::{IsolationLevel, Transaction, TransactionConfig};
pub use types::{
    NullsOrder, OrderBy, OrderByBuilder, OrderByField, Select, SortOrder, order_patterns,
};

// Re-export middleware types
pub use middleware::{
    LoggingMiddleware, MetricsMiddleware, Middleware, MiddlewareBuilder, MiddlewareChain,
    MiddlewareStack, QueryContext, QueryMetadata, QueryMetrics, QueryType, RetryMiddleware,
    TimingMiddleware,
};

// Re-export connection types
pub use connection::{
    ConnectionError, ConnectionOptions, ConnectionString, DatabaseConfig, Driver, EnvExpander,
    MultiDatabaseConfig, PoolConfig, PoolOptions, SslConfig, SslMode,
};

// Re-export data types
pub use data::{
    BatchCreate, ConnectData, CreateData, DataBuilder, FieldValue, IntoData, UpdateData,
};

// Re-export tenant types
pub use tenant::{
    DynamicResolver, IsolationStrategy, RowLevelConfig, SchemaConfig, StaticResolver, TenantConfig,
    TenantConfigBuilder, TenantContext, TenantId, TenantInfo, TenantMiddleware, TenantResolver,
};

// Re-export intern types
pub use intern::{clear_interned, fields, intern, intern_cow, interned_count};

// Re-export pool types
pub use pool::{FilterBuilder, FilterPool, IntoPooledValue, PooledFilter, PooledValue};

// Re-export SQL builder types
pub use sql::{DatabaseType, FastSqlBuilder, QueryCapacity, SqlBuilder, templates};

// Re-export cache types
pub use cache::{
    CacheStats, CachedQuery, ExecutionPlan, ExecutionPlanCache, PlanHint, QueryCache, QueryHash,
    QueryKey, SqlTemplate, SqlTemplateCache, get_global_template, global_template_cache,
    patterns as cache_patterns, precompute_query_hash,
    register_global_template,
};

// Re-export batch types
pub use batch::{
    Batch, BatchBuilder, BatchOperation, BatchResult, OperationResult, Pipeline, PipelineBuilder,
    PipelineQuery, PipelineResult, QueryResult as PipelineQueryResult,
};

// Re-export row deserialization types
pub use row::{FromColumn, FromRow, FromRowRef, RowData, RowError, RowRef, RowRefIter};

// Re-export lazy loading types
pub use lazy::{Lazy, LazyRelation, ManyToOneLoader, OneToManyLoader};

// Re-export static filter utilities
pub use static_filter::{
    CompactValue, StaticFilter, and2, and3, and4, and5, contains, ends_with, eq,
    fields as static_fields, gt, gte, in_list, is_not_null, is_null, lt, lte, ne, not, not_in_list,
    or2, or3, or4, or5, starts_with,
};

// Re-export typed filter utilities
pub use typed_filter::{
    And, AndN, Contains, DirectSql, EndsWith, Eq, Gt, Gte, InI64, InI64Slice, InStr, InStrSlice,
    IsNotNull, IsNull, LazyFilter, Lt, Lte, Maybe, Ne, Not as TypedNot, NotInI64Slice, Or, OrN,
    StartsWith, TypedFilter, and_n, eq as typed_eq, gt as typed_gt, gte as typed_gte,
    in_i64 as typed_in_i64, in_i64_slice, in_str as typed_in_str, in_str_slice,
    is_not_null as typed_is_not_null, is_null as typed_is_null, lazy, lt as typed_lt,
    lte as typed_lte, ne as typed_ne, not_in_i64_slice, or_n,
};

// Re-export memory optimization utilities
pub use memory::{
    BufferPool, CompactFilter, GLOBAL_BUFFER_POOL, GLOBAL_STRING_POOL, MemoryStats, PoolStats,
    PooledBuffer, StringPool, get_buffer, intern as memory_intern,
};

// Re-export logging utilities
pub use logging::{
    get_log_format, get_log_level, init as init_logging, init_debug, init_with_level,
    is_debug_enabled,
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
    pub use crate::raw::{RawExecuteOperation, RawQueryOperation, Sql};
    pub use crate::raw_query;
    pub use crate::relations::{Include, IncludeSpec, RelationSpec, SelectSpec};
    pub use crate::traits::{Executable, IntoFilter, Model, QueryEngine};
    pub use crate::transaction::{IsolationLevel, Transaction, TransactionConfig};
    pub use crate::types::{OrderBy, Select, SortOrder};

    // Tenant types
    pub use crate::tenant::{IsolationStrategy, TenantConfig, TenantContext, TenantMiddleware};
}
