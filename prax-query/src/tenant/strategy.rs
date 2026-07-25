//! Tenant isolation strategies.

use std::collections::HashSet;

use crate::error::{QueryError, QueryResult};
use crate::sql::is_valid_sql_identifier;

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
    ///
    /// # Hazard
    ///
    /// `Uuid`, `Integer`, and `BigInt` values are interpolated into the
    /// resulting SQL fragment **without any validation or escaping**. A
    /// malicious value (e.g. a raw HTTP header used as a tenant id) can
    /// inject arbitrary SQL. Use [`Self::try_format_value`] instead; the
    /// tenant middleware also validates tenant ids at the middleware
    /// boundary.
    #[deprecated(
        since = "0.11.0",
        note = "interpolates `value` into SQL without validation; use `try_format_value`, \
                which rejects invalid UUID/integer values with a `QueryError`"
    )]
    pub fn format_value(&self, value: &str) -> String {
        match self {
            Self::String => format!("'{}'", value.replace('\'', "''")),
            Self::Uuid => format!("'{}'::uuid", value),
            Self::Integer | Self::BigInt => value.to_string(),
        }
    }

    /// Format a value for this column type, validating it first.
    ///
    /// - `String` values must match the conservative tenant id whitelist
    ///   `^[A-Za-z0-9_\-\:.@]+$` — `''`-doubling alone is not sufficient on
    ///   backends that honor backslash escapes (MySQL), so string ids are
    ///   fail-closed instead of escaped.
    /// - `Uuid` values must parse via [`uuid::Uuid::parse_str`].
    /// - `Integer`/`BigInt` values must parse as an `i64`.
    ///
    /// Invalid input is rejected with a [`QueryError`] instead of being
    /// interpolated into SQL.
    pub fn try_format_value(&self, value: &str) -> QueryResult<String> {
        match self {
            Self::String => {
                if is_valid_string_tenant_id(value) {
                    Ok(format!("'{}'", value))
                } else {
                    Err(QueryError::invalid_input(
                        "tenant_id",
                        "value is outside the string tenant id whitelist \
                         (allowed: letters, digits, `_`, `-`, `:`, `.`, `@`)",
                    ))
                }
            }
            Self::Uuid => uuid::Uuid::parse_str(value)
                .map(|uuid| format!("'{}'::uuid", uuid))
                .map_err(|_| QueryError::invalid_input("tenant_id", "value is not a valid UUID")),
            Self::Integer | Self::BigInt => {
                value.parse::<i64>().map(|n| n.to_string()).map_err(|_| {
                    QueryError::invalid_input("tenant_id", "value is not a valid integer")
                })
            }
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
    ///
    /// This is a pure string composition; no validation is performed. Use
    /// [`Self::try_schema_name`] to reject tenant ids that would compose
    /// into an invalid SQL identifier.
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

    /// Generate the schema name for a tenant, validating the composed name.
    ///
    /// The composed schema name must match the strict SQL identifier charset
    /// `^[A-Za-z_][A-Za-z0-9_]*$`; otherwise a [`QueryError`] is returned so
    /// a malicious tenant id is rejected before it can reach a SQL
    /// statement.
    pub fn try_schema_name(&self, tenant_id: &str) -> QueryResult<String> {
        let name = self.schema_name(tenant_id);
        if is_valid_schema_ident(&name) {
            Ok(name)
        } else {
            Err(QueryError::invalid_input(
                "tenant_id",
                format!("schema name `{}` is not a valid SQL identifier", name),
            ))
        }
    }

    /// Generate the search_path SQL for a tenant.
    ///
    /// The composed tenant schema name is validated against the strict SQL
    /// identifier charset `^[A-Za-z_][A-Za-z0-9_]*$` at the point of SQL
    /// generation. A name outside that charset is emitted as a double-quoted
    /// identifier (with embedded `"` doubled) so a malicious tenant id
    /// cannot break out of the identifier and inject SQL. Use
    /// [`Self::try_search_path`] to reject invalid tenant ids with an error
    /// instead.
    pub fn search_path(&self, tenant_id: &str) -> String {
        let tenant_schema = safe_schema_ident(&self.schema_name(tenant_id));
        let shared_schema = self.shared_schema.as_deref().map(safe_schema_ident);
        self.search_path_sql(&tenant_schema, shared_schema.as_deref())
    }

    /// Generate the search_path SQL for a tenant, validating all schema
    /// names first.
    ///
    /// Returns a [`QueryError`] if the composed tenant schema name or the
    /// configured shared schema is not a valid SQL identifier
    /// (`^[A-Za-z_][A-Za-z0-9_]*$`).
    pub fn try_search_path(&self, tenant_id: &str) -> QueryResult<String> {
        let tenant_schema = self.try_schema_name(tenant_id)?;
        if let Some(shared) = &self.shared_schema
            && !is_valid_schema_ident(shared)
        {
            return Err(QueryError::invalid_input(
                "shared_schema",
                format!("schema name `{}` is not a valid SQL identifier", shared),
            ));
        }
        Ok(self.search_path_sql(&tenant_schema, self.shared_schema.as_deref()))
    }

    /// Build the search_path SQL from pre-validated (or safely quoted)
    /// schema names.
    fn search_path_sql(&self, tenant_schema: &str, shared_schema: Option<&str>) -> String {
        match self.search_path_format {
            SearchPathFormat::TenantOnly => {
                format!("SET search_path TO {}", tenant_schema)
            }
            SearchPathFormat::TenantFirst => {
                if let Some(shared) = shared_schema {
                    format!("SET search_path TO {}, {}", tenant_schema, shared)
                } else {
                    format!("SET search_path TO {}, public", tenant_schema)
                }
            }
            SearchPathFormat::SharedFirst => {
                if let Some(shared) = shared_schema {
                    format!("SET search_path TO {}, {}", shared, tenant_schema)
                } else {
                    format!("SET search_path TO public, {}", tenant_schema)
                }
            }
        }
    }
}

/// Check whether a string tenant id matches the conservative whitelist
/// `^[A-Za-z0-9_\-\:.@]+$`. Quote-doubling alone is not sufficient
/// validation on backends that honor backslash escapes (MySQL), so string
/// tenant ids are restricted to characters that are safe inside a
/// single-quoted SQL literal on every backend.
fn is_valid_string_tenant_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b':' | b'.' | b'@'))
}

/// Check whether a composed schema name is a strict SQL identifier
/// (`^[A-Za-z_][A-Za-z0-9_]*$`).
fn is_valid_schema_ident(name: &str) -> bool {
    is_valid_sql_identifier(name)
}

/// Return the schema name unchanged when it is a strict SQL identifier;
/// otherwise emit it as a double-quoted identifier (with embedded `"`
/// doubled) so it cannot inject SQL into a `SET search_path` statement.
fn safe_schema_ident(name: &str) -> String {
    if is_valid_schema_ident(name) {
        name.to_string()
    } else {
        format!("\"{}\"", name.replace('"', "\"\""))
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
    #[allow(deprecated)]
    fn test_column_type_format() {
        assert_eq!(ColumnType::String.format_value("test"), "'test'");
        assert_eq!(
            ColumnType::Uuid.format_value("123e4567-e89b-12d3-a456-426614174000"),
            "'123e4567-e89b-12d3-a456-426614174000'::uuid"
        );
        assert_eq!(ColumnType::Integer.format_value("42"), "42");
    }

    #[test]
    fn test_schema_config_rejects_malicious_tenant_ids() {
        let config = SchemaConfig::default().with_prefix("tenant_");

        for bad in ["acme'; DROP", "1 OR true--", "a b"] {
            assert!(config.try_schema_name(bad).is_err());
            assert!(config.try_search_path(bad).is_err());
        }

        assert_eq!(config.try_schema_name("acme").unwrap(), "tenant_acme");
        assert_eq!(
            config.try_search_path("acme").unwrap(),
            "SET search_path TO tenant_acme, public"
        );
    }

    #[test]
    fn test_search_path_neutralizes_malicious_tenant_ids() {
        let config = SchemaConfig::default().with_prefix("tenant_");

        for bad in ["acme'; DROP", "1 OR true--", "a b"] {
            // The infallible path cannot reject, so the composed name is
            // emitted as a single double-quoted identifier and cannot break
            // out of the statement to inject SQL.
            assert_eq!(
                config.search_path(bad),
                format!(
                    "SET search_path TO \"tenant_{}\", public",
                    bad.replace('"', "\"\"")
                )
            );
        }
    }

    #[test]
    fn test_try_format_value_rejects_malicious_values() {
        for bad in [
            "acme'; DROP",
            "1 OR true--",
            "a b",
            "' OR 1=1-- ",
            "\\' OR 1=1-- ",
            "",
        ] {
            // String tenant ids are fail-closed: anything outside the
            // whitelist is rejected, not escaped.
            assert!(ColumnType::String.try_format_value(bad).is_err());
            assert!(ColumnType::Uuid.try_format_value(bad).is_err());
            assert!(ColumnType::Integer.try_format_value(bad).is_err());
            assert!(ColumnType::BigInt.try_format_value(bad).is_err());
        }
    }

    #[test]
    fn test_try_format_value_valid_values() {
        // Whitelisted string tenant ids: letters, digits, `_`, `-`, `:`,
        // `.`, `@`.
        for good in ["test", "tenant-123", "a_b-c:d.e@f", "Tenant_01", "x"] {
            assert_eq!(
                ColumnType::String.try_format_value(good).unwrap(),
                format!("'{good}'")
            );
        }
        assert_eq!(
            ColumnType::Uuid
                .try_format_value("123e4567-e89b-12d3-a456-426614174000")
                .unwrap(),
            "'123e4567-e89b-12d3-a456-426614174000'::uuid"
        );
        assert_eq!(ColumnType::Integer.try_format_value("42").unwrap(), "42");
        assert_eq!(ColumnType::BigInt.try_format_value("-42").unwrap(), "-42");
    }
}
