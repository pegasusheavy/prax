//! Migration types and the migrator.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{debug, info};

use crate::error::{MigrateError, MigrateResult};

/// A database migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    /// Unique name/identifier for the migration.
    pub name: String,
    /// Version (typically a timestamp).
    pub version: String,
    /// Description of what this migration does.
    pub description: Option<String>,
    /// SQL to apply the migration (up).
    pub up_sql: String,
    /// SQL to reverse the migration (down).
    pub down_sql: Option<String>,
    /// Checksum of the migration content.
    pub checksum: String,
    /// When the migration was created.
    pub created_at: DateTime<Utc>,
}

impl Migration {
    /// Create a new migration.
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        up_sql: impl Into<String>,
    ) -> Self {
        let up_sql = up_sql.into();
        let checksum = compute_checksum(&up_sql);

        Self {
            name: name.into(),
            version: version.into(),
            description: None,
            up_sql,
            down_sql: None,
            checksum,
            created_at: Utc::now(),
        }
    }

    /// Set the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set the down SQL.
    pub fn with_down_sql(mut self, sql: impl Into<String>) -> Self {
        self.down_sql = Some(sql.into());
        self
    }

    /// Check if this migration is reversible.
    pub fn is_reversible(&self) -> bool {
        self.down_sql.is_some()
    }

    /// Verify the checksum matches the content.
    pub fn verify_checksum(&self) -> bool {
        let computed = compute_checksum(&self.up_sql);
        computed == self.checksum
    }

    /// Get the full migration name (version_name).
    pub fn full_name(&self) -> String {
        format!("{}_{}", self.version, self.name)
    }
}

/// Direction of migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MigrationDirection {
    /// Apply the migration (up).
    Up,
    /// Reverse the migration (down).
    Down,
}

/// Status of a migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MigrationStatus {
    /// Migration is pending (not applied).
    Pending,
    /// Migration has been applied.
    Applied,
    /// Migration was applied but has been rolled back.
    RolledBack,
    /// Migration failed to apply.
    Failed,
}

/// Configuration for the migrator.
#[derive(Debug, Clone)]
pub struct MigratorConfig {
    /// Directory containing migrations.
    pub migrations_dir: PathBuf,
    /// Path to the schema file.
    pub schema_path: Option<PathBuf>,
    /// Table name for tracking migrations.
    pub history_table: String,
    /// Whether to allow dirty state (unapplied changes).
    pub allow_dirty: bool,
    /// Whether to run migrations in a transaction.
    pub use_transaction: bool,
}

impl Default for MigratorConfig {
    fn default() -> Self {
        Self {
            migrations_dir: PathBuf::from("migrations"),
            schema_path: None,
            history_table: "_prax_migrations".to_string(),
            allow_dirty: false,
            use_transaction: true,
        }
    }
}

impl MigratorConfig {
    /// Create a new config with the migrations directory.
    pub fn new(migrations_dir: impl Into<PathBuf>) -> Self {
        Self {
            migrations_dir: migrations_dir.into(),
            ..Default::default()
        }
    }

    /// Set the schema path.
    pub fn with_schema(mut self, path: impl Into<PathBuf>) -> Self {
        self.schema_path = Some(path.into());
        self
    }

    /// Set the history table name.
    pub fn with_history_table(mut self, name: impl Into<String>) -> Self {
        self.history_table = name.into();
        self
    }

    /// Allow dirty state.
    pub fn allow_dirty(mut self) -> Self {
        self.allow_dirty = true;
        self
    }

    /// Disable transactions for migrations.
    pub fn no_transaction(mut self) -> Self {
        self.use_transaction = false;
        self
    }
}

/// The migrator handles migration operations.
pub struct Migrator {
    config: MigratorConfig,
    migrations: Vec<Migration>,
}

impl Migrator {
    /// Create a new migrator with the given directory.
    pub fn new(migrations_dir: impl Into<PathBuf>) -> MigratorBuilder {
        MigratorBuilder::new(migrations_dir)
    }

    /// Create a migrator from config.
    pub fn from_config(config: MigratorConfig) -> Self {
        Self {
            config,
            migrations: Vec::new(),
        }
    }

    /// Get the configuration.
    pub fn config(&self) -> &MigratorConfig {
        &self.config
    }

    /// Load migrations from the migrations directory.
    pub fn load_migrations(&mut self) -> MigrateResult<()> {
        let dir = &self.config.migrations_dir;

        if !dir.exists() {
            debug!("Migrations directory does not exist: {:?}", dir);
            return Ok(());
        }

        let mut entries: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().is_dir()
                    || e.path().extension().map_or(false, |ext| ext == "sql")
            })
            .collect();

        entries.sort_by_key(|e| e.path());

        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                // Directory-based migration
                self.load_migration_dir(&path)?;
            } else {
                // Single file migration
                self.load_migration_file(&path)?;
            }
        }

        info!("Loaded {} migrations", self.migrations.len());
        Ok(())
    }

    /// Load a migration from a directory.
    fn load_migration_dir(&mut self, path: &Path) -> MigrateResult<()> {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| MigrateError::migration_file("invalid directory name"))?;

        let up_path = path.join("up.sql");
        let down_path = path.join("down.sql");

        if !up_path.exists() {
            return Err(MigrateError::migration_file(format!(
                "missing up.sql in {}",
                name
            )));
        }

        let up_sql = std::fs::read_to_string(&up_path)?;
        let down_sql = if down_path.exists() {
            Some(std::fs::read_to_string(&down_path)?)
        } else {
            None
        };

        let (version, migration_name) = parse_migration_name(name)?;

        let mut migration = Migration::new(migration_name, version, up_sql);
        if let Some(down) = down_sql {
            migration = migration.with_down_sql(down);
        }

        self.migrations.push(migration);
        Ok(())
    }

    /// Load a migration from a single file.
    fn load_migration_file(&mut self, path: &Path) -> MigrateResult<()> {
        let name = path
            .file_stem()
            .and_then(|n| n.to_str())
            .ok_or_else(|| MigrateError::migration_file("invalid file name"))?;

        let content = std::fs::read_to_string(path)?;
        let (version, migration_name) = parse_migration_name(name)?;

        // Check for -- down comment to split up/down
        let (up_sql, down_sql) = if content.contains("-- down") || content.contains("-- DOWN") {
            let parts: Vec<&str> = content
                .splitn(2, |c| content.to_lowercase().contains("-- down"))
                .collect();
            if parts.len() == 2 {
                (parts[0].to_string(), Some(parts[1].to_string()))
            } else {
                (content, None)
            }
        } else {
            (content, None)
        };

        let mut migration = Migration::new(migration_name, version, up_sql);
        if let Some(down) = down_sql {
            migration = migration.with_down_sql(down);
        }

        self.migrations.push(migration);
        Ok(())
    }

    /// Get all loaded migrations.
    pub fn migrations(&self) -> &[Migration] {
        &self.migrations
    }

    /// Get pending migrations (not yet applied).
    pub fn pending_migrations(&self, applied: &[String]) -> Vec<&Migration> {
        self.migrations
            .iter()
            .filter(|m| !applied.contains(&m.full_name()))
            .collect()
    }

    /// Generate SQL for creating the migrations history table.
    pub fn create_history_table_sql(&self) -> String {
        format!(
            r#"
CREATE TABLE IF NOT EXISTS {} (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    version VARCHAR(255) NOT NULL,
    checksum VARCHAR(64) NOT NULL,
    applied_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    execution_time_ms INTEGER,
    rolled_back_at TIMESTAMP WITH TIME ZONE
);
"#,
            self.config.history_table
        )
    }

    /// Generate a new migration with the given name.
    pub fn generate_migration(&self, name: &str) -> MigrateResult<PathBuf> {
        let version = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
        let dir_name = format!("{}_{}", version, name);
        let dir_path = self.config.migrations_dir.join(&dir_name);

        std::fs::create_dir_all(&dir_path)?;

        let up_path = dir_path.join("up.sql");
        let down_path = dir_path.join("down.sql");

        std::fs::write(&up_path, "-- Add migration SQL here\n")?;
        std::fs::write(&down_path, "-- Add rollback SQL here\n")?;

        info!("Generated migration: {}", dir_name);
        Ok(dir_path)
    }
}

/// Builder for creating a migrator.
pub struct MigratorBuilder {
    config: MigratorConfig,
}

impl MigratorBuilder {
    /// Create a new builder.
    pub fn new(migrations_dir: impl Into<PathBuf>) -> Self {
        Self {
            config: MigratorConfig::new(migrations_dir),
        }
    }

    /// Set the schema path.
    pub fn with_schema(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.schema_path = Some(path.into());
        self
    }

    /// Set the history table name.
    pub fn with_history_table(mut self, name: impl Into<String>) -> Self {
        self.config.history_table = name.into();
        self
    }

    /// Allow dirty state.
    pub fn allow_dirty(mut self) -> Self {
        self.config.allow_dirty = true;
        self
    }

    /// Disable transactions.
    pub fn no_transaction(mut self) -> Self {
        self.config.use_transaction = false;
        self
    }

    /// Build the migrator.
    pub fn build(self) -> MigrateResult<Migrator> {
        let mut migrator = Migrator::from_config(self.config);
        migrator.load_migrations()?;
        Ok(migrator)
    }
}

/// Compute a SHA256 checksum of the content.
fn compute_checksum(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

/// Parse a migration name into (version, name).
fn parse_migration_name(full_name: &str) -> MigrateResult<(String, String)> {
    let parts: Vec<&str> = full_name.splitn(2, '_').collect();
    if parts.len() != 2 {
        return Err(MigrateError::migration_file(format!(
            "invalid migration name format: {}. Expected: VERSION_NAME",
            full_name
        )));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_creation() {
        let migration = Migration::new("add_users", "20240101120000", "CREATE TABLE users (id INT);");

        assert_eq!(migration.name, "add_users");
        assert_eq!(migration.version, "20240101120000");
        assert!(!migration.checksum.is_empty());
        assert!(!migration.is_reversible());
    }

    #[test]
    fn test_migration_with_down() {
        let migration = Migration::new("add_users", "20240101120000", "CREATE TABLE users (id INT);")
            .with_down_sql("DROP TABLE users;");

        assert!(migration.is_reversible());
    }

    #[test]
    fn test_migration_checksum() {
        let migration = Migration::new("test", "1", "SELECT 1;");
        assert!(migration.verify_checksum());
    }

    #[test]
    fn test_migration_full_name() {
        let migration = Migration::new("add_users", "20240101120000", "CREATE TABLE users;");
        assert_eq!(migration.full_name(), "20240101120000_add_users");
    }

    #[test]
    fn test_parse_migration_name() {
        let (version, name) = parse_migration_name("20240101120000_add_users").unwrap();
        assert_eq!(version, "20240101120000");
        assert_eq!(name, "add_users");
    }

    #[test]
    fn test_parse_migration_name_invalid() {
        let result = parse_migration_name("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_migrator_config() {
        let config = MigratorConfig::new("./migrations")
            .with_schema("schema.prax")
            .with_history_table("custom_migrations")
            .allow_dirty();

        assert_eq!(config.migrations_dir, PathBuf::from("./migrations"));
        assert_eq!(config.schema_path, Some(PathBuf::from("schema.prax")));
        assert_eq!(config.history_table, "custom_migrations");
        assert!(config.allow_dirty);
    }

    #[test]
    fn test_create_history_table_sql() {
        let migrator = Migrator::from_config(MigratorConfig::default());
        let sql = migrator.create_history_table_sql();

        assert!(sql.contains("CREATE TABLE IF NOT EXISTS _prax_migrations"));
        assert!(sql.contains("name VARCHAR"));
        assert!(sql.contains("checksum VARCHAR"));
    }
}

