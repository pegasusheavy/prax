//! Tenant configuration.

use super::strategy::{IsolationStrategy, RowLevelConfig, SchemaConfig, DatabaseConfig};
use super::resolver::TenantResolver;
use std::sync::Arc;

/// Configuration for multi-tenant support.
#[derive(Clone)]
pub struct TenantConfig {
    /// The isolation strategy.
    pub strategy: IsolationStrategy,
    /// Whether tenant context is required for all queries.
    pub require_tenant: bool,
    /// Default tenant ID for queries without context.
    pub default_tenant: Option<String>,
    /// Allow superuser to bypass tenant filtering.
    pub allow_bypass: bool,
    /// Tenant resolver for dynamic tenant lookup.
    pub resolver: Option<Arc<dyn TenantResolver>>,
    /// Whether to enforce tenant on write operations.
    pub enforce_on_writes: bool,
    /// Whether to log tenant context with queries.
    pub log_tenant_context: bool,
}

impl std::fmt::Debug for TenantConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TenantConfig")
            .field("strategy", &self.strategy)
            .field("require_tenant", &self.require_tenant)
            .field("default_tenant", &self.default_tenant)
            .field("allow_bypass", &self.allow_bypass)
            .field("enforce_on_writes", &self.enforce_on_writes)
            .field("log_tenant_context", &self.log_tenant_context)
            .finish()
    }
}

impl TenantConfig {
    /// Create a row-level isolation config.
    pub fn row_level(column: impl Into<String>) -> Self {
        Self {
            strategy: IsolationStrategy::row_level(column),
            require_tenant: true,
            default_tenant: None,
            allow_bypass: true,
            resolver: None,
            enforce_on_writes: true,
            log_tenant_context: false,
        }
    }

    /// Create a schema-based isolation config.
    pub fn schema_based() -> Self {
        Self {
            strategy: IsolationStrategy::schema_based(),
            require_tenant: true,
            default_tenant: None,
            allow_bypass: true,
            resolver: None,
            enforce_on_writes: true,
            log_tenant_context: false,
        }
    }

    /// Create a database-based isolation config.
    pub fn database_based() -> Self {
        Self {
            strategy: IsolationStrategy::database_based(),
            require_tenant: true,
            default_tenant: None,
            allow_bypass: true,
            resolver: None,
            enforce_on_writes: true,
            log_tenant_context: false,
        }
    }

    /// Create a builder for advanced configuration.
    pub fn builder() -> TenantConfigBuilder {
        TenantConfigBuilder::default()
    }

    /// Set the default tenant.
    pub fn with_default_tenant(mut self, tenant: impl Into<String>) -> Self {
        self.default_tenant = Some(tenant.into());
        self
    }

    /// Don't require tenant context (use default if missing).
    pub fn optional(mut self) -> Self {
        self.require_tenant = false;
        self
    }

    /// Disable superuser bypass.
    pub fn without_bypass(mut self) -> Self {
        self.allow_bypass = false;
        self
    }

    /// Set the tenant resolver.
    pub fn with_resolver<R: TenantResolver + 'static>(mut self, resolver: R) -> Self {
        self.resolver = Some(Arc::new(resolver));
        self
    }

    /// Enable tenant context logging.
    pub fn with_logging(mut self) -> Self {
        self.log_tenant_context = true;
        self
    }

    /// Get the row-level config.
    pub fn row_level_config(&self) -> Option<&RowLevelConfig> {
        self.strategy.row_level_config()
    }

    /// Get the schema config.
    pub fn schema_config(&self) -> Option<&SchemaConfig> {
        self.strategy.schema_config()
    }

    /// Get the database config.
    pub fn database_config(&self) -> Option<&DatabaseConfig> {
        self.strategy.database_config()
    }
}

/// Builder for advanced tenant configuration.
#[derive(Default)]
pub struct TenantConfigBuilder {
    strategy: Option<IsolationStrategy>,
    require_tenant: bool,
    default_tenant: Option<String>,
    allow_bypass: bool,
    resolver: Option<Arc<dyn TenantResolver>>,
    enforce_on_writes: bool,
    log_tenant_context: bool,
}

impl TenantConfigBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            require_tenant: true,
            allow_bypass: true,
            enforce_on_writes: true,
            ..Default::default()
        }
    }

    /// Set the isolation strategy.
    pub fn strategy(mut self, strategy: IsolationStrategy) -> Self {
        self.strategy = Some(strategy);
        self
    }

    /// Use row-level isolation.
    pub fn row_level(mut self, config: RowLevelConfig) -> Self {
        self.strategy = Some(IsolationStrategy::RowLevel(config));
        self
    }

    /// Use schema-based isolation.
    pub fn schema(mut self, config: SchemaConfig) -> Self {
        self.strategy = Some(IsolationStrategy::Schema(config));
        self
    }

    /// Use database-based isolation.
    pub fn database(mut self, config: DatabaseConfig) -> Self {
        self.strategy = Some(IsolationStrategy::Database(config));
        self
    }

    /// Require tenant context.
    pub fn require_tenant(mut self, require: bool) -> Self {
        self.require_tenant = require;
        self
    }

    /// Set the default tenant.
    pub fn default_tenant(mut self, tenant: impl Into<String>) -> Self {
        self.default_tenant = Some(tenant.into());
        self
    }

    /// Allow bypass for superusers.
    pub fn allow_bypass(mut self, allow: bool) -> Self {
        self.allow_bypass = allow;
        self
    }

    /// Set the tenant resolver.
    pub fn resolver<R: TenantResolver + 'static>(mut self, resolver: R) -> Self {
        self.resolver = Some(Arc::new(resolver));
        self
    }

    /// Enforce tenant on writes.
    pub fn enforce_on_writes(mut self, enforce: bool) -> Self {
        self.enforce_on_writes = enforce;
        self
    }

    /// Enable tenant context logging.
    pub fn log_context(mut self, log: bool) -> Self {
        self.log_tenant_context = log;
        self
    }

    /// Build the config.
    pub fn build(self) -> TenantConfig {
        TenantConfig {
            strategy: self.strategy.unwrap_or_else(|| IsolationStrategy::row_level("tenant_id")),
            require_tenant: self.require_tenant,
            default_tenant: self.default_tenant,
            allow_bypass: self.allow_bypass,
            resolver: self.resolver,
            enforce_on_writes: self.enforce_on_writes,
            log_tenant_context: self.log_tenant_context,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_level_config() {
        let config = TenantConfig::row_level("org_id")
            .with_default_tenant("default")
            .with_logging();

        assert!(config.strategy.is_row_level());
        assert_eq!(config.default_tenant, Some("default".to_string()));
        assert!(config.log_tenant_context);
    }

    #[test]
    fn test_schema_config() {
        let config = TenantConfig::schema_based()
            .optional()
            .without_bypass();

        assert!(config.strategy.is_schema_based());
        assert!(!config.require_tenant);
        assert!(!config.allow_bypass);
    }

    #[test]
    fn test_builder() {
        let config = TenantConfig::builder()
            .row_level(RowLevelConfig::new("tenant_id").with_database_rls())
            .default_tenant("system")
            .log_context(true)
            .build();

        assert!(config.strategy.is_row_level());
        assert!(config.row_level_config().unwrap().use_database_rls);
    }
}


