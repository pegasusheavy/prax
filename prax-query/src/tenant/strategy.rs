//! Tenant isolation strategies.

use std::collections::HashSet;

/// The isolation strategy for multi-tenancy.
#[derive(Debug, Clone)]
pub enum IsolationStrategy {
    /// Row-level security: all tenants share tables, filtered by column.
    RowLevel(RowLevelConfig),
    /// Schema-based: each tenant has their own schema.
    Schema(SchemaConfig),
    /// Database-based: each tenant has their own database.
    Database(DatabaseConfig),
    /// Hybrid: combination of strategies (e.g., schema + row-level).
    Hybrid(Box<IsolationStrategy>, Box<IsolationStrategy>),
}

impl IsolationStrategy {
    /// Create a row-level isolation strategy.
    pub fn row_level(column: impl Into<String>) -> Self {
        Self::RowLevel(RowLevelConfig::new(column))
    }

    /// Create a schema-based isolation strategy.
    pub fn schema_based() -> Self {
        Self::Schema(SchemaConfig::default())
    }

    /// Create a database-based isolation strategy.
    pub fn database_based() -> Self {
        Self::Database(DatabaseConfig::default())
    }

    /// Check if this is row-level isolation.
    pub fn is_row_level(&self) -> bool {
        matches!(self, Self::RowLevel(_))
    }

    /// Check if this is schema-based isolation.
    pub fn is_schema_based(&self) -> bool {
        matches!(self, Self::Schema(_))
    }

    /// Check if this is database-based isolation.
    pub fn is_database_based(&self) -> bool {
        matches!(self, Self::Database(_))
    }

    /// Get the row-level config if applicable.
    pub fn row_level_config(&self) -> Option<&RowLevelConfig> {
        match self {
            Self::RowLevel(config) => Some(config),
            Self::Hybrid(a, b) => a.row_level_config().or_else(|| b.row_level_config()),
            _ => None,
        }
    }

    /// Get the schema config if applicable.
    pub fn schema_config(&self) -> Option<&SchemaConfig> {
        match self {
            Self::Schema(config) => Some(config),
            Self::Hybrid(a, b) => a.schema_config().or_else(|| b.schema_config()),
            _ => None,
        }
    }

    /// Get the database config if applicable.
    pub fn database_config(&self) -> Option<&DatabaseConfig> {
        match self {
            Self::Database(config) => Some(config),
            Self::Hybrid(a, b) => a.database_config().or_else(|| b.database_config()),
            _ => None,
        }
    }
}

/// Configuration for row-level tenant isolation.
#[derive(Debug, Clone)]
pub struct RowLevelConfig {
    /// The column name that stores the tenant ID.
    pub column: String,
    /// The column type (for type-safe comparisons).
    pub column_type: ColumnType,
    /// Tables that should be excluded from tenant filtering.
    pub excluded_tables: HashSet<String>,
    /// Tables that are shared across all tenants.
    pub shared_tables: HashSet<String>,
    /// Whether to automatically add tenant_id to INSERT statements.
    pub auto_insert: bool,
    /// Whether to validate tenant_id on UPDATE/DELETE.
    pub validate_writes: bool,
    /// Whether to use database-level RLS (PostgreSQL).
    pub use_database_rls: bool,
}

impl RowLevelConfig {
    /// Create a new row-level config with the given column name.
    pub fn new(column: impl Into<String>) -> Self {
        Self {
            column: column.into(),
            column_type: ColumnType::String,
            excluded_tables: HashSet::new(),
            shared_tables: HashSet::new(),
            auto_insert: true,
            validate_writes: true,
            use_database_rls: false,
        }
    }

    /// Set the column type.
    pub fn with_column_type(mut self, column_type: ColumnType) -> Self {
        self.column_type = column_type;
        self
    }

    /// Exclude a table from tenant filtering.
    pub fn exclude_table(mut self, table: impl Into<String>) -> Self {
        self.excluded_tables.insert(table.into());
        self
    }

    /// Mark a table as shared (no tenant filtering).
    pub fn shared_table(mut self, table: impl Into<String>) -> Self {
        self.shared_tables.insert(table.into());
        self
    }

    /// Disable automatic tenant_id insertion.
    pub fn without_auto_insert(mut self) -> Self {
        self.auto_insert = false;
        self
    }

    /// Disable write validation.
    pub fn without_write_validation(mut self) -> Self {
        self.validate_writes = false;
        self
    }

    /// Enable PostgreSQL database-level RLS.
    pub fn with_database_rls(mut self) -> Self {
        self.use_database_rls = true;
        self
    }

    /// Check if a table should be filtered.
    pub fn should_filter(&self, table: &str) -> bool {
        !self.excluded_tables.contains(table) && !self.shared_tables.contains(table)
    }
}

/// The type of the tenant column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColumnType {
    /// String/VARCHAR/TEXT column.
    #[default]
    String,
    /// UUID column.
    Uuid,
    /// Integer column.
    Integer,
    /// BigInt column.
    BigInt,
}

impl ColumnType {
    /// Get the SQL placeholder for this column type.
    pub fn placeholder(&self, index: usize) -> String {
        format!("${}", index)
    }

    /// Format a value for this column type.
    pub fn format_value(&self, value: &str) -> String {
        match self {
            Self::String => format!("'{}'", value.replace('\'', "''")),
            Self::Uuid => format!("'{}'::uuid", value),
            Self::Integer | Self::BigInt => value.to_string(),
        }
    }
}

/// Configuration for schema-based tenant isolation.
#[derive(Debug, Clone, Default)]
pub struct SchemaConfig {
    /// Prefix for tenant schema names (e.g., "tenant_" -> "tenant_acme").
    pub schema_prefix: Option<String>,
    /// Suffix for tenant schema names.
    pub schema_suffix: Option<String>,
    /// Name of the shared schema for common tables.
    pub shared_schema: Option<String>,
    /// Whether to create schemas automatically.
    pub auto_create: bool,
    /// Default schema for new tenants.
    pub default_schema: Option<String>,
    /// Schema search path format.
    pub search_path_format: SearchPathFormat,
}

impl SchemaConfig {
    /// Set the schema prefix.
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.schema_prefix = Some(prefix.into());
        self
    }

    /// Set the schema suffix.
    pub fn with_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.schema_suffix = Some(suffix.into());
        self
    }

    /// Set the shared schema name.
    pub fn with_shared_schema(mut self, schema: impl Into<String>) -> Self {
        self.shared_schema = Some(schema.into());
        self
    }

    /// Enable auto-creation of schemas.
    pub fn with_auto_create(mut self) -> Self {
        self.auto_create = true;
        self
    }

    /// Set the default schema.
    pub fn with_default_schema(mut self, schema: impl Into<String>) -> Self {
        self.default_schema = Some(schema.into());
        self
    }

    /// Set the search path format.
    pub fn with_search_path(mut self, format: SearchPathFormat) -> Self {
        self.search_path_format = format;
        self
    }

    /// Generate the schema name for a tenant.
    pub fn schema_name(&self, tenant_id: &str) -> String {
        let mut name = String::new();
        if let Some(prefix) = &self.schema_prefix {
            name.push_str(prefix);
        }
        name.push_str(tenant_id);
        if let Some(suffix) = &self.schema_suffix {
            name.push_str(suffix);
        }
        name
    }

    /// Generate the search_path SQL for a tenant.
    pub fn search_path(&self, tenant_id: &str) -> String {
        let tenant_schema = self.schema_name(tenant_id);
        match self.search_path_format {
            SearchPathFormat::TenantOnly => {
                format!("SET search_path TO {}", tenant_schema)
            }
            SearchPathFormat::TenantFirst => {
                if let Some(shared) = &self.shared_schema {
                    format!("SET search_path TO {}, {}", tenant_schema, shared)
                } else {
                    format!("SET search_path TO {}, public", tenant_schema)
                }
            }
            SearchPathFormat::SharedFirst => {
                if let Some(shared) = &self.shared_schema {
                    format!("SET search_path TO {}, {}", shared, tenant_schema)
                } else {
                    format!("SET search_path TO public, {}", tenant_schema)
                }
            }
        }
    }
}

/// Format for the schema search path.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchPathFormat {
    /// Only include the tenant schema.
    TenantOnly,
    /// Tenant schema first, then shared.
    #[default]
    TenantFirst,
    /// Shared schema first, then tenant.
    SharedFirst,
}

/// Configuration for database-based tenant isolation.
#[derive(Debug, Clone, Default)]
pub struct DatabaseConfig {
    /// Prefix for tenant database names.
    pub database_prefix: Option<String>,
    /// Suffix for tenant database names.
    pub database_suffix: Option<String>,
    /// Whether to create databases automatically.
    pub auto_create: bool,
    /// Template database for new tenant databases.
    pub template_database: Option<String>,
    /// Connection pool size per tenant.
    pub pool_size_per_tenant: usize,
    /// Maximum number of tenant connections to keep.
    pub max_tenant_connections: usize,
}

impl DatabaseConfig {
    /// Set the database prefix.
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.database_prefix = Some(prefix.into());
        self
    }

    /// Set the database suffix.
    pub fn with_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.database_suffix = Some(suffix.into());
        self
    }

    /// Enable auto-creation of databases.
    pub fn with_auto_create(mut self) -> Self {
        self.auto_create = true;
        self
    }

    /// Set the template database.
    pub fn with_template(mut self, template: impl Into<String>) -> Self {
        self.template_database = Some(template.into());
        self
    }

    /// Set the pool size per tenant.
    pub fn with_pool_size(mut self, size: usize) -> Self {
        self.pool_size_per_tenant = size;
        self
    }

    /// Set the maximum tenant connections.
    pub fn with_max_connections(mut self, max: usize) -> Self {
        self.max_tenant_connections = max;
        self
    }

    /// Generate the database name for a tenant.
    pub fn database_name(&self, tenant_id: &str) -> String {
        let mut name = String::new();
        if let Some(prefix) = &self.database_prefix {
            name.push_str(prefix);
        }
        name.push_str(tenant_id);
        if let Some(suffix) = &self.database_suffix {
            name.push_str(suffix);
        }
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_level_config() {
        let config = RowLevelConfig::new("tenant_id")
            .with_column_type(ColumnType::Uuid)
            .exclude_table("audit_logs")
            .shared_table("plans");

        assert_eq!(config.column, "tenant_id");
        assert_eq!(config.column_type, ColumnType::Uuid);
        assert!(config.should_filter("users"));
        assert!(!config.should_filter("audit_logs"));
        assert!(!config.should_filter("plans"));
    }

    #[test]
    fn test_schema_config() {
        let config = SchemaConfig::default()
            .with_prefix("tenant_")
            .with_shared_schema("shared");

        assert_eq!(config.schema_name("acme"), "tenant_acme");
        assert!(config.search_path("acme").contains("tenant_acme"));
        assert!(config.search_path("acme").contains("shared"));
    }

    #[test]
    fn test_database_config() {
        let config = DatabaseConfig::default()
            .with_prefix("prax_")
            .with_suffix("_db");

        assert_eq!(config.database_name("acme"), "prax_acme_db");
    }

    #[test]
    fn test_column_type_format() {
        assert_eq!(
            ColumnType::String.format_value("test"),
            "'test'"
        );
        assert_eq!(
            ColumnType::Uuid.format_value("123e4567-e89b-12d3-a456-426614174000"),
            "'123e4567-e89b-12d3-a456-426614174000'::uuid"
        );
        assert_eq!(
            ColumnType::Integer.format_value("42"),
            "42"
        );
    }
}


