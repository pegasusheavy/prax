//! Configuration file parsing for `prax.toml`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::error::{SchemaError, SchemaResult};

/// Main configuration structure for `prax.toml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PraxConfig {
    /// Database configuration.
    #[serde(default)]
    pub database: DatabaseConfig,

    /// Schema file configuration.
    #[serde(default)]
    pub schema: SchemaConfig,

    /// Generator configuration.
    #[serde(default)]
    pub generator: GeneratorConfig,

    /// Migration settings.
    #[serde(default)]
    pub migrations: MigrationConfig,

    /// Seeding configuration.
    #[serde(default)]
    pub seed: SeedConfig,

    /// Debug/logging settings.
    #[serde(default)]
    pub debug: DebugConfig,

    /// Environment-specific overrides.
    #[serde(default)]
    pub environments: HashMap<String, EnvironmentOverride>,
}

impl Default for PraxConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            schema: SchemaConfig::default(),
            generator: GeneratorConfig::default(),
            migrations: MigrationConfig::default(),
            seed: SeedConfig::default(),
            debug: DebugConfig::default(),
            environments: HashMap::new(),
        }
    }
}

impl PraxConfig {
    /// Load configuration from a file path.
    pub fn from_file(path: impl AsRef<Path>) -> SchemaResult<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| SchemaError::IoError {
            path: path.display().to_string(),
            source: e,
        })?;

        Self::from_str(&content)
    }

    /// Parse configuration from a TOML string.
    pub fn from_str(content: &str) -> SchemaResult<Self> {
        // First, expand environment variables
        let expanded = expand_env_vars(content);
        
        toml::from_str(&expanded).map_err(|e| SchemaError::TomlError { source: e })
    }

    /// Get the database URL, resolving environment variables.
    pub fn database_url(&self) -> Option<&str> {
        self.database.url.as_deref()
    }

    /// Apply environment-specific overrides.
    pub fn with_environment(mut self, env: &str) -> Self {
        if let Some(overrides) = self.environments.remove(env) {
            if let Some(db) = overrides.database {
                if let Some(url) = db.url {
                    self.database.url = Some(url);
                }
                if let Some(pool) = db.pool {
                    self.database.pool = pool;
                }
            }
            if let Some(debug) = overrides.debug {
                if let Some(log_queries) = debug.log_queries {
                    self.debug.log_queries = log_queries;
                }
                if let Some(pretty_sql) = debug.pretty_sql {
                    self.debug.pretty_sql = pretty_sql;
                }
                if let Some(threshold) = debug.slow_query_threshold {
                    self.debug.slow_query_threshold = threshold;
                }
            }
        }
        self
    }
}

/// Database configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseConfig {
    /// Database provider.
    #[serde(default = "default_provider")]
    pub provider: DatabaseProvider,

    /// Connection URL (supports `${ENV_VAR}` interpolation).
    pub url: Option<String>,

    /// Connection pool settings.
    #[serde(default)]
    pub pool: PoolConfig,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            provider: DatabaseProvider::PostgreSql,
            url: None,
            pool: PoolConfig::default(),
        }
    }
}

fn default_provider() -> DatabaseProvider {
    DatabaseProvider::PostgreSql
}

/// Supported database providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseProvider {
    /// PostgreSQL.
    #[serde(alias = "postgres")]
    PostgreSql,
    /// MySQL / MariaDB.
    MySql,
    /// SQLite.
    #[serde(alias = "sqlite3")]
    Sqlite,
    /// MongoDB.
    #[serde(alias = "mongo")]
    MongoDb,
}

impl DatabaseProvider {
    /// Get the provider name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PostgreSql => "postgresql",
            Self::MySql => "mysql",
            Self::Sqlite => "sqlite",
            Self::MongoDb => "mongodb",
        }
    }
}

/// Connection pool configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PoolConfig {
    /// Minimum number of connections.
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,

    /// Maximum number of connections.
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Connection timeout.
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: String,

    /// Idle connection timeout.
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout: String,

    /// Maximum connection lifetime.
    #[serde(default = "default_max_lifetime")]
    pub max_lifetime: String,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: default_min_connections(),
            max_connections: default_max_connections(),
            connect_timeout: default_connect_timeout(),
            idle_timeout: default_idle_timeout(),
            max_lifetime: default_max_lifetime(),
        }
    }
}

fn default_min_connections() -> u32 { 2 }
fn default_max_connections() -> u32 { 10 }
fn default_connect_timeout() -> String { "30s".to_string() }
fn default_idle_timeout() -> String { "10m".to_string() }
fn default_max_lifetime() -> String { "30m".to_string() }

/// Schema file configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaConfig {
    /// Path to the schema file.
    #[serde(default = "default_schema_path")]
    pub path: String,
}

impl Default for SchemaConfig {
    fn default() -> Self {
        Self {
            path: default_schema_path(),
        }
    }
}

fn default_schema_path() -> String {
    "schema.prax".to_string()
}

/// Generator configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratorConfig {
    /// Client generator settings.
    #[serde(default)]
    pub client: ClientGeneratorConfig,
}

/// Client generator configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClientGeneratorConfig {
    /// Output directory.
    #[serde(default = "default_output")]
    pub output: String,

    /// Generate async client.
    #[serde(default = "default_true")]
    pub async_client: bool,

    /// Enable tracing instrumentation.
    #[serde(default)]
    pub tracing: bool,

    /// Preview features to enable.
    #[serde(default)]
    pub preview_features: Vec<String>,
}

impl Default for ClientGeneratorConfig {
    fn default() -> Self {
        Self {
            output: default_output(),
            async_client: true,
            tracing: false,
            preview_features: vec![],
        }
    }
}

fn default_output() -> String { "./src/generated".to_string() }
fn default_true() -> bool { true }

/// Migration configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationConfig {
    /// Migration files directory.
    #[serde(default = "default_migrations_dir")]
    pub directory: String,

    /// Auto-apply migrations in development.
    #[serde(default)]
    pub auto_migrate: bool,

    /// Migration history table name.
    #[serde(default = "default_migrations_table")]
    pub table_name: String,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            directory: default_migrations_dir(),
            auto_migrate: false,
            table_name: default_migrations_table(),
        }
    }
}

fn default_migrations_dir() -> String { "./migrations".to_string() }
fn default_migrations_table() -> String { "_prax_migrations".to_string() }

/// Seed configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SeedConfig {
    /// Seed script path.
    pub script: Option<String>,

    /// Run seed after migrations.
    #[serde(default)]
    pub auto_seed: bool,

    /// Environment-specific seeding flags.
    #[serde(default)]
    pub environments: HashMap<String, bool>,
}

impl Default for SeedConfig {
    fn default() -> Self {
        Self {
            script: None,
            auto_seed: false,
            environments: HashMap::new(),
        }
    }
}

/// Debug/logging configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DebugConfig {
    /// Log all queries.
    #[serde(default)]
    pub log_queries: bool,

    /// Pretty print SQL.
    #[serde(default = "default_true")]
    pub pretty_sql: bool,

    /// Slow query threshold in milliseconds.
    #[serde(default = "default_slow_query_threshold")]
    pub slow_query_threshold: u64,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            log_queries: false,
            pretty_sql: true,
            slow_query_threshold: default_slow_query_threshold(),
        }
    }
}

fn default_slow_query_threshold() -> u64 { 1000 }

/// Environment-specific configuration overrides.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentOverride {
    /// Database overrides.
    pub database: Option<DatabaseOverride>,

    /// Debug overrides.
    pub debug: Option<DebugOverride>,
}

/// Database configuration overrides.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseOverride {
    /// Override connection URL.
    pub url: Option<String>,

    /// Override pool settings.
    pub pool: Option<PoolConfig>,
}

/// Debug configuration overrides.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DebugOverride {
    /// Override log_queries.
    pub log_queries: Option<bool>,

    /// Override pretty_sql.
    pub pretty_sql: Option<bool>,

    /// Override slow_query_threshold.
    pub slow_query_threshold: Option<u64>,
}

/// Expand environment variables in the format `${VAR_NAME}`.
fn expand_env_vars(content: &str) -> String {
    let mut result = content.to_string();
    let re = regex_lite::Regex::new(r"\$\{([^}]+)\}").unwrap();
    
    for cap in re.captures_iter(content) {
        let var_name = &cap[1];
        let full_match = &cap[0];
        
        if let Ok(value) = std::env::var(var_name) {
            result = result.replace(full_match, &value);
        }
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PraxConfig::default();
        assert_eq!(config.database.provider, DatabaseProvider::PostgreSql);
        assert_eq!(config.schema.path, "schema.prax");
    }

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
            [database]
            provider = "postgresql"
            url = "postgres://localhost/test"
        "#;

        let config = PraxConfig::from_str(toml).unwrap();
        assert_eq!(config.database.url, Some("postgres://localhost/test".to_string()));
    }

    #[test]
    fn test_env_var_expansion() {
        // SAFETY: This test runs single-threaded and we clean up after
        unsafe {
            std::env::set_var("TEST_DB_URL", "postgres://test");
        }
        let expanded = expand_env_vars("url = \"${TEST_DB_URL}\"");
        assert_eq!(expanded, "url = \"postgres://test\"");
        unsafe {
            std::env::remove_var("TEST_DB_URL");
        }
    }
}

