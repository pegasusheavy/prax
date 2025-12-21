//! Shadow database support for safe migration testing.
//!
//! Shadow databases are temporary databases used to:
//! - Test migrations before applying them to production
//! - Validate migration correctness and rollback safety
//! - Generate accurate diffs by comparing desired schema vs actual state
//! - Detect drift between schema definition and database state
//!
//! # How it works
//!
//! 1. Create a temporary database with a unique name
//! 2. Apply all migrations to the shadow database
//! 3. Introspect the shadow database to get actual schema state
//! 4. Compare with desired schema to detect drift
//! 5. Clean up (drop) the shadow database
//!
//! # Example
//!
//! ```rust,ignore
//! use prax_migrate::shadow::{ShadowDatabase, ShadowConfig};
//!
//! // Create shadow database manager
//! let shadow = ShadowDatabase::new(ShadowConfig::default());
//!
//! // Create and use shadow database
//! let shadow_url = shadow.create().await?;
//!
//! // Apply migrations to shadow
//! shadow.apply_migrations(&migrations).await?;
//!
//! // Introspect to get actual state
//! let actual_schema = shadow.introspect().await?;
//!
//! // Compare with desired
//! let diff = compare_schemas(&desired_schema, &actual_schema);
//!
//! // Clean up
//! shadow.drop().await?;
//! ```

use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{MigrateResult, MigrationError};
use crate::file::MigrationFile;

/// Configuration for shadow database operations.
#[derive(Debug, Clone)]
pub struct ShadowConfig {
    /// Base connection URL (without database name).
    pub base_url: String,
    /// Prefix for shadow database names.
    pub prefix: String,
    /// Whether to auto-cleanup on drop.
    pub auto_cleanup: bool,
    /// Timeout for shadow operations in seconds.
    pub timeout_seconds: u64,
}

impl Default for ShadowConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            prefix: "_prax_shadow_".to_string(),
            auto_cleanup: true,
            timeout_seconds: 300, // 5 minutes
        }
    }
}

impl ShadowConfig {
    /// Create a new shadow database config.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Default::default()
        }
    }

    /// Set a custom prefix for shadow database names.
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Disable auto-cleanup (for debugging).
    pub fn no_auto_cleanup(mut self) -> Self {
        self.auto_cleanup = false;
        self
    }

    /// Set operation timeout.
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Generate a unique shadow database name.
    pub fn generate_name(&self) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let random: u32 = rand_simple();
        format!("{}{:x}_{:x}", self.prefix, timestamp, random)
    }

    /// Get the full connection URL for a shadow database.
    pub fn shadow_url(&self, db_name: &str) -> String {
        // Parse base URL and replace database name
        if self.base_url.contains("://") {
            // Handle URL format: protocol://user:pass@host:port/dbname
            if let Some(idx) = self.base_url.rfind('/') {
                format!("{}/{}", &self.base_url[..idx], db_name)
            } else {
                format!("{}/{}", self.base_url, db_name)
            }
        } else {
            // Simple format
            format!("{}/{}", self.base_url, db_name)
        }
    }
}

/// Simple random number generator for unique names.
fn rand_simple() -> u32 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::thread::current().id().hash(&mut hasher);
    SystemTime::now().hash(&mut hasher);
    hasher.finish() as u32
}

/// Current state of a shadow database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowState {
    /// Shadow database not yet created.
    NotCreated,
    /// Shadow database exists and is ready.
    Ready,
    /// Shadow database has been dropped.
    Dropped,
    /// Shadow database is in an error state.
    Error,
}

/// A shadow database instance for testing migrations.
#[derive(Debug)]
pub struct ShadowDatabase {
    config: ShadowConfig,
    db_name: String,
    state: ShadowState,
    applied_migrations: Vec<String>,
}

impl ShadowDatabase {
    /// Create a new shadow database manager.
    pub fn new(config: ShadowConfig) -> Self {
        let db_name = config.generate_name();
        Self {
            config,
            db_name,
            state: ShadowState::NotCreated,
            applied_migrations: Vec::new(),
        }
    }

    /// Get the shadow database name.
    pub fn name(&self) -> &str {
        &self.db_name
    }

    /// Get the full connection URL for this shadow database.
    pub fn url(&self) -> String {
        self.config.shadow_url(&self.db_name)
    }

    /// Get the current state.
    pub fn state(&self) -> ShadowState {
        self.state
    }

    /// Get list of applied migrations.
    pub fn applied_migrations(&self) -> &[String] {
        &self.applied_migrations
    }

    /// Create the shadow database.
    ///
    /// This creates an empty database that can be used for migration testing.
    pub async fn create(&mut self) -> MigrateResult<String> {
        if self.state != ShadowState::NotCreated {
            return Err(MigrationError::ShadowDatabaseError(format!(
                "Shadow database already in state {:?}",
                self.state
            )));
        }

        // The actual creation would be done through the query engine:
        // CREATE DATABASE shadow_name
        // For now, we just update state
        self.state = ShadowState::Ready;

        Ok(self.url())
    }

    /// Generate the SQL to create this shadow database.
    pub fn create_sql(&self) -> String {
        format!(
            "CREATE DATABASE {} WITH TEMPLATE template0 ENCODING 'UTF8'",
            quote_identifier(&self.db_name)
        )
    }

    /// Generate the SQL to drop this shadow database.
    pub fn drop_sql(&self) -> String {
        format!(
            "DROP DATABASE IF EXISTS {}",
            quote_identifier(&self.db_name)
        )
    }

    /// Apply a migration to the shadow database.
    pub async fn apply_migration(&mut self, migration: &MigrationFile) -> MigrateResult<()> {
        if self.state != ShadowState::Ready {
            return Err(MigrationError::ShadowDatabaseError(
                "Shadow database not ready".to_string(),
            ));
        }

        // The actual execution would be done through the query engine
        // For now, we track the applied migration
        self.applied_migrations.push(migration.id.clone());

        Ok(())
    }

    /// Apply multiple migrations to the shadow database.
    pub async fn apply_migrations(&mut self, migrations: &[MigrationFile]) -> MigrateResult<()> {
        for migration in migrations {
            self.apply_migration(migration).await?;
        }
        Ok(())
    }

    /// Check if the shadow database is ready for introspection.
    ///
    /// The actual introspection should be done through a concrete `Introspector`
    /// implementation connected to this shadow database's URL.
    pub fn is_ready_for_introspection(&self) -> bool {
        self.state == ShadowState::Ready
    }

    /// Get SQL to verify the shadow database schema matches expected structure.
    ///
    /// This can be used to validate migrations before applying to production.
    pub fn verify_schema_sql(&self, table_name: &str) -> String {
        format!(
            "SELECT column_name, data_type, is_nullable \
             FROM information_schema.columns \
             WHERE table_schema = 'public' AND table_name = '{}' \
             ORDER BY ordinal_position",
            table_name.replace('\'', "''")
        )
    }

    /// Reset the shadow database (drop and recreate).
    pub async fn reset(&mut self) -> MigrateResult<()> {
        if self.state == ShadowState::Ready {
            self.drop().await?;
        }

        // Generate new name
        self.db_name = self.config.generate_name();
        self.applied_migrations.clear();
        self.state = ShadowState::NotCreated;

        self.create().await?;
        Ok(())
    }

    /// Drop the shadow database.
    pub async fn drop(&mut self) -> MigrateResult<()> {
        if self.state == ShadowState::Dropped {
            return Ok(());
        }

        // The actual drop would be done through the query engine:
        // DROP DATABASE shadow_name
        self.state = ShadowState::Dropped;
        self.applied_migrations.clear();

        Ok(())
    }
}

impl Drop for ShadowDatabase {
    fn drop(&mut self) {
        if self.config.auto_cleanup && self.state == ShadowState::Ready {
            // In a real implementation, we'd spawn a task to drop the database
            // For now, we just log
            #[cfg(feature = "tracing")]
            tracing::warn!(
                "Shadow database '{}' was not explicitly dropped. Consider calling drop() explicitly.",
                self.db_name
            );
        }
    }
}

/// Quote a PostgreSQL identifier.
fn quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Manager for shadow database operations.
#[derive(Debug)]
pub struct ShadowDatabaseManager {
    config: ShadowConfig,
    active_shadows: Vec<String>,
}

impl ShadowDatabaseManager {
    /// Create a new shadow database manager.
    pub fn new(config: ShadowConfig) -> Self {
        Self {
            config,
            active_shadows: Vec::new(),
        }
    }

    /// Create a new shadow database.
    pub fn create_shadow(&mut self) -> ShadowDatabase {
        let shadow = ShadowDatabase::new(self.config.clone());
        self.active_shadows.push(shadow.name().to_string());
        shadow
    }

    /// Clean up all active shadow databases.
    pub async fn cleanup_all(&mut self) -> MigrateResult<()> {
        // In a real implementation, this would drop all shadow databases
        self.active_shadows.clear();
        Ok(())
    }

    /// Get list of active shadow database names.
    pub fn active_shadows(&self) -> &[String] {
        &self.active_shadows
    }

    /// Check if a database name looks like a shadow database.
    pub fn is_shadow_database(&self, name: &str) -> bool {
        name.starts_with(&self.config.prefix)
    }
}

/// Result of comparing schemas using a shadow database.
#[derive(Debug)]
pub struct ShadowDiffResult {
    /// The desired schema (from Prax schema files).
    pub desired: prax_schema::Schema,
    /// The actual schema (from shadow database after migrations).
    pub actual: prax_schema::Schema,
    /// Drift detected between schemas.
    pub drift: SchemaDrift,
}

/// Detected drift between desired and actual schema.
#[derive(Debug, Default)]
pub struct SchemaDrift {
    /// Models in desired but not in actual.
    pub missing_models: Vec<String>,
    /// Models in actual but not in desired.
    pub extra_models: Vec<String>,
    /// Fields that differ between schemas.
    pub field_differences: Vec<FieldDrift>,
    /// Index differences.
    pub index_differences: Vec<IndexDrift>,
}

impl SchemaDrift {
    /// Check if there's any drift.
    pub fn has_drift(&self) -> bool {
        !self.missing_models.is_empty()
            || !self.extra_models.is_empty()
            || !self.field_differences.is_empty()
            || !self.index_differences.is_empty()
    }

    /// Get a summary of the drift.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        if !self.missing_models.is_empty() {
            parts.push(format!("{} missing models", self.missing_models.len()));
        }
        if !self.extra_models.is_empty() {
            parts.push(format!("{} extra models", self.extra_models.len()));
        }
        if !self.field_differences.is_empty() {
            parts.push(format!(
                "{} field differences",
                self.field_differences.len()
            ));
        }
        if !self.index_differences.is_empty() {
            parts.push(format!(
                "{} index differences",
                self.index_differences.len()
            ));
        }

        if parts.is_empty() {
            "No drift detected".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// A detected difference in a field.
#[derive(Debug)]
pub struct FieldDrift {
    /// Model name.
    pub model: String,
    /// Field name.
    pub field: String,
    /// Description of the difference.
    pub description: String,
}

/// A detected difference in an index.
#[derive(Debug)]
pub struct IndexDrift {
    /// Model name.
    pub model: String,
    /// Index name.
    pub index: String,
    /// Description of the difference.
    pub description: String,
}

/// Compare two schemas and detect drift.
pub fn detect_drift(desired: &prax_schema::Schema, actual: &prax_schema::Schema) -> SchemaDrift {
    let mut drift = SchemaDrift::default();

    // Collect model names (IndexMap returns (key, value) tuples)
    let desired_models: std::collections::HashSet<&str> = desired
        .models
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    let actual_models: std::collections::HashSet<&str> = actual
        .models
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();

    // Find missing and extra models
    drift.missing_models = desired_models
        .difference(&actual_models)
        .map(|s: &&str| s.to_string())
        .collect();
    drift.extra_models = actual_models
        .difference(&desired_models)
        .map(|s: &&str| s.to_string())
        .collect();

    // Compare fields in common models
    for (model_name, desired_model) in &desired.models {
        let model_name_str = model_name.as_str();
        if let Some(actual_model) = actual.models.get(model_name_str) {
            // Compare fields (fields is IndexMap<SmolStr, Field>)
            let desired_field_names: std::collections::HashSet<&str> =
                desired_model.fields.keys().map(|k| k.as_str()).collect();
            let actual_field_names: std::collections::HashSet<&str> =
                actual_model.fields.keys().map(|k| k.as_str()).collect();

            // Check for missing and extra fields
            for field_name in desired_field_names.difference(&actual_field_names) {
                drift.field_differences.push(FieldDrift {
                    model: model_name_str.to_string(),
                    field: field_name.to_string(),
                    description: "Field missing in actual schema".to_string(),
                });
            }

            for field_name in actual_field_names.difference(&desired_field_names) {
                drift.field_differences.push(FieldDrift {
                    model: model_name_str.to_string(),
                    field: field_name.to_string(),
                    description: "Extra field in actual schema".to_string(),
                });
            }

            // Check for type/modifier differences in common fields
            for field_name in desired_field_names.intersection(&actual_field_names) {
                let desired_field = desired_model.fields.get(*field_name).unwrap();
                let actual_field = actual_model.fields.get(*field_name).unwrap();

                if desired_field.field_type != actual_field.field_type {
                    drift.field_differences.push(FieldDrift {
                        model: model_name_str.to_string(),
                        field: field_name.to_string(),
                        description: format!(
                            "Type mismatch: expected {:?}, got {:?}",
                            desired_field.field_type, actual_field.field_type
                        ),
                    });
                }
                if desired_field.modifier != actual_field.modifier {
                    drift.field_differences.push(FieldDrift {
                        model: model_name_str.to_string(),
                        field: field_name.to_string(),
                        description: format!(
                            "Modifier mismatch: expected {:?}, got {:?}",
                            desired_field.modifier, actual_field.modifier
                        ),
                    });
                }
            }
        }
    }

    drift
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_config_default() {
        let config = ShadowConfig::default();
        assert_eq!(config.prefix, "_prax_shadow_");
        assert!(config.auto_cleanup);
        assert_eq!(config.timeout_seconds, 300);
    }

    #[test]
    fn test_shadow_config_builder() {
        let config = ShadowConfig::new("postgresql://localhost")
            .with_prefix("_test_shadow_")
            .no_auto_cleanup()
            .with_timeout(60);

        assert_eq!(config.base_url, "postgresql://localhost");
        assert_eq!(config.prefix, "_test_shadow_");
        assert!(!config.auto_cleanup);
        assert_eq!(config.timeout_seconds, 60);
    }

    #[test]
    fn test_generate_name() {
        let config = ShadowConfig::default();
        let name1 = config.generate_name();
        let name2 = config.generate_name();

        assert!(name1.starts_with("_prax_shadow_"));
        assert!(name2.starts_with("_prax_shadow_"));
        // Names should be unique
        assert_ne!(name1, name2);
    }

    #[test]
    fn test_shadow_url() {
        let config = ShadowConfig::new("postgresql://user:pass@localhost:5432/original");
        let url = config.shadow_url("shadow_db");
        assert_eq!(url, "postgresql://user:pass@localhost:5432/shadow_db");
    }

    #[test]
    fn test_shadow_database_new() {
        let config = ShadowConfig::new("postgresql://localhost");
        let shadow = ShadowDatabase::new(config);

        assert!(shadow.name().starts_with("_prax_shadow_"));
        assert_eq!(shadow.state(), ShadowState::NotCreated);
        assert!(shadow.applied_migrations().is_empty());
    }

    #[tokio::test]
    async fn test_shadow_database_lifecycle() {
        let config = ShadowConfig::new("postgresql://localhost");
        let mut shadow = ShadowDatabase::new(config);

        // Create
        let url = shadow.create().await.unwrap();
        assert!(url.contains(&shadow.name().to_string()));
        assert_eq!(shadow.state(), ShadowState::Ready);

        // Drop
        shadow.drop().await.unwrap();
        assert_eq!(shadow.state(), ShadowState::Dropped);
    }

    #[test]
    fn test_create_sql() {
        let config = ShadowConfig::new("postgresql://localhost");
        let shadow = ShadowDatabase::new(config);
        let sql = shadow.create_sql();

        assert!(sql.starts_with("CREATE DATABASE"));
        assert!(sql.contains(&shadow.name().to_string()));
    }

    #[test]
    fn test_drop_sql() {
        let config = ShadowConfig::new("postgresql://localhost");
        let shadow = ShadowDatabase::new(config);
        let sql = shadow.drop_sql();

        assert!(sql.starts_with("DROP DATABASE IF EXISTS"));
        assert!(sql.contains(&shadow.name().to_string()));
    }

    #[test]
    fn test_shadow_manager() {
        let config = ShadowConfig::new("postgresql://localhost");
        let mut manager = ShadowDatabaseManager::new(config);

        let shadow1 = manager.create_shadow();
        let shadow2 = manager.create_shadow();

        assert_eq!(manager.active_shadows().len(), 2);
        assert!(manager.is_shadow_database(shadow1.name()));
        assert!(manager.is_shadow_database(shadow2.name()));
        assert!(!manager.is_shadow_database("regular_db"));
    }

    #[test]
    fn test_schema_drift_empty() {
        let drift = SchemaDrift::default();
        assert!(!drift.has_drift());
        assert_eq!(drift.summary(), "No drift detected");
    }

    #[test]
    fn test_schema_drift_with_differences() {
        let drift = SchemaDrift {
            missing_models: vec!["User".to_string()],
            extra_models: vec!["OldTable".to_string()],
            field_differences: vec![FieldDrift {
                model: "Post".to_string(),
                field: "title".to_string(),
                description: "Type mismatch".to_string(),
            }],
            index_differences: Vec::new(),
        };

        assert!(drift.has_drift());
        let summary = drift.summary();
        assert!(summary.contains("1 missing models"));
        assert!(summary.contains("1 extra models"));
        assert!(summary.contains("1 field differences"));
    }

    #[test]
    fn test_detect_drift_no_drift() {
        let schema = prax_schema::Schema::new();
        let drift = detect_drift(&schema, &schema);
        assert!(!drift.has_drift());
    }

    #[test]
    fn test_quote_identifier() {
        assert_eq!(quote_identifier("table"), "\"table\"");
        assert_eq!(quote_identifier("has\"quote"), "\"has\"\"quote\"");
    }
}
