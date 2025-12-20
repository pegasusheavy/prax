//! Multi-tenant support for Prax.
//!
//! This module provides comprehensive multi-tenancy support with multiple isolation strategies:
//!
//! - **Row-Level Security (RLS)**: All tenants share tables, filtered by tenant_id column
//! - **Schema-Based**: Each tenant has their own database schema
//! - **Database-Based**: Each tenant has their own database
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use prax_query::tenant::{TenantContext, TenantConfig, IsolationStrategy};
//!
//! // Configure row-level isolation
//! let config = TenantConfig::row_level("tenant_id");
//!
//! // Create tenant context
//! let ctx = TenantContext::new("tenant-123");
//!
//! // Use with client
//! let client = PraxClient::new(db_config)
//!     .with_tenant(config)
//!     .set_tenant(ctx);
//!
//! // All queries automatically filtered by tenant
//! let users = client.user().find_many().exec().await?;
//! // SQL: SELECT * FROM users WHERE tenant_id = 'tenant-123'
//! ```
//!
//! # Isolation Strategies
//!
//! ## Row-Level Security
//!
//! The simplest approach where all tenants share the same tables:
//!
//! ```rust,ignore
//! let config = TenantConfig::row_level("tenant_id")
//!     .with_default_tenant("default")
//!     .with_bypass_for_superuser(true);
//! ```
//!
//! ## Schema-Based
//!
//! Each tenant gets their own schema (PostgreSQL/MySQL):
//!
//! ```rust,ignore
//! let config = TenantConfig::schema_based()
//!     .with_schema_prefix("tenant_")
//!     .with_shared_schema("shared");
//! ```
//!
//! ## Database-Based
//!
//! Each tenant gets their own database:
//!
//! ```rust,ignore
//! let config = TenantConfig::database_based()
//!     .with_resolver(|tenant_id| async move {
//!         // Return connection config for tenant
//!         DatabaseConfig::from_url(&format!("postgres://localhost/{}", tenant_id))
//!     });
//! ```

mod context;
mod config;
mod middleware;
mod resolver;
mod strategy;

pub use context::{TenantContext, TenantId, TenantInfo};
pub use config::{TenantConfig, TenantConfigBuilder};
pub use middleware::TenantMiddleware;
pub use resolver::{TenantResolver, StaticResolver, DynamicResolver, DatabaseResolver};
pub use strategy::{IsolationStrategy, RowLevelConfig, SchemaConfig, DatabaseConfig as TenantDatabaseConfig};


