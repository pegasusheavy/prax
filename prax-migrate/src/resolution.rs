//! Migration resolution system.
//!
//! This module provides mechanisms to handle migration conflicts and issues:
//! - Checksum mismatches (when migrations are modified after being applied)
//! - Conflict resolution (when multiple migrations conflict)
//! - Baseline migrations (mark existing schema as a baseline)
//! - Renamed migrations (map old IDs to new ones)
//! - Skipped migrations (intentionally skip certain migrations)
//!
//! # Example
//!
//! ```rust,ignore
//! use prax_migrate::resolution::{ResolutionConfig, Resolution, ResolutionAction};
//!
//! let mut resolutions = ResolutionConfig::new();
//!
//! // Accept a modified migration's new checksum
//! resolutions.add(Resolution::accept_checksum(
//!     "20240101_create_users",
//!     "old_checksum",
//!     "new_checksum",
//!     "Fixed typo in column name"
//! ));
//!
//! // Skip a migration entirely
//! resolutions.add(Resolution::skip(
//!     "20240102_add_legacy_table",
//!     "Table already exists from legacy system"
//! ));
//!
//! // Mark a migration as baseline (applied without running)
//! resolutions.add(Resolution::baseline(
//!     "20240103_initial_schema",
//!     "Schema was imported from existing database"
//! ));
//!
//! // Save resolutions to file
//! resolutions.save("migrations/resolutions.toml").await?;
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{MigrateResult, MigrationError};

/// Configuration for managing migration resolutions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResolutionConfig {
    /// Map of migration ID to resolution.
    #[serde(default)]
    pub resolutions: HashMap<String, Resolution>,
    /// When the config was last modified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<DateTime<Utc>>,
    /// Path to the resolution file.
    #[serde(skip)]
    pub file_path: Option<PathBuf>,
}

impl ResolutionConfig {
    /// Create a new empty resolution config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load resolutions from a file.
    pub async fn load(path: impl AsRef<Path>) -> MigrateResult<Self> {
        let path = path.as_ref();

        if !path.exists() {
            return Ok(Self {
                file_path: Some(path.to_path_buf()),
                ..Default::default()
            });
        }

        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            MigrationError::ResolutionFile(format!("Failed to read resolution file: {}", e))
        })?;

        let mut config: Self = toml::from_str(&content).map_err(|e| {
            MigrationError::ResolutionFile(format!("Failed to parse resolution file: {}", e))
        })?;

        config.file_path = Some(path.to_path_buf());
        Ok(config)
    }

    /// Save resolutions to file.
    pub async fn save(&self, path: impl AsRef<Path>) -> MigrateResult<()> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                MigrationError::ResolutionFile(format!("Failed to create directory: {}", e))
            })?;
        }

        let mut config = self.clone();
        config.last_modified = Some(Utc::now());

        let content = toml::to_string_pretty(&config).map_err(|e| {
            MigrationError::ResolutionFile(format!("Failed to serialize resolutions: {}", e))
        })?;

        // Add header comment
        let content = format!(
            "# Prax Migration Resolutions\n\
             # This file tracks intentional changes to migrations\n\
             # Do not edit manually unless you know what you're doing\n\n\
             {}",
            content
        );

        tokio::fs::write(path, content).await.map_err(|e| {
            MigrationError::ResolutionFile(format!("Failed to write resolution file: {}", e))
        })?;

        Ok(())
    }

    /// Add a resolution.
    pub fn add(&mut self, resolution: Resolution) {
        self.resolutions
            .insert(resolution.migration_id.clone(), resolution);
    }

    /// Remove a resolution.
    pub fn remove(&mut self, migration_id: &str) -> Option<Resolution> {
        self.resolutions.remove(migration_id)
    }

    /// Get a resolution for a migration.
    pub fn get(&self, migration_id: &str) -> Option<&Resolution> {
        self.resolutions.get(migration_id)
    }

    /// Check if a migration has a resolution.
    pub fn has_resolution(&self, migration_id: &str) -> bool {
        self.resolutions.contains_key(migration_id)
    }

    /// Get all resolutions of a specific type.
    pub fn by_action(&self, action: ResolutionAction) -> Vec<&Resolution> {
        self.resolutions
            .values()
            .filter(|r| std::mem::discriminant(&r.action) == std::mem::discriminant(&action))
            .collect()
    }

    /// Get all skipped migrations.
    pub fn skipped(&self) -> Vec<&str> {
        self.resolutions
            .values()
            .filter_map(|r| {
                if matches!(r.action, ResolutionAction::Skip) {
                    Some(r.migration_id.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all baseline migrations.
    pub fn baselines(&self) -> Vec<&str> {
        self.resolutions
            .values()
            .filter_map(|r| {
                if matches!(r.action, ResolutionAction::Baseline) {
                    Some(r.migration_id.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if a migration should be skipped.
    pub fn should_skip(&self, migration_id: &str) -> bool {
        self.get(migration_id)
            .map(|r| matches!(r.action, ResolutionAction::Skip))
            .unwrap_or(false)
    }

    /// Check if a migration is a baseline.
    pub fn is_baseline(&self, migration_id: &str) -> bool {
        self.get(migration_id)
            .map(|r| matches!(r.action, ResolutionAction::Baseline))
            .unwrap_or(false)
    }

    /// Check if a checksum mismatch is accepted.
    pub fn accepts_checksum(
        &self,
        migration_id: &str,
        old_checksum: &str,
        new_checksum: &str,
    ) -> bool {
        self.get(migration_id)
            .map(|r| {
                if let ResolutionAction::AcceptChecksum {
                    from_checksum,
                    to_checksum,
                } = &r.action
                {
                    from_checksum == old_checksum && to_checksum == new_checksum
                } else {
                    false
                }
            })
            .unwrap_or(false)
    }

    /// Get the renamed migration ID if this migration was renamed.
    pub fn get_renamed(&self, old_id: &str) -> Option<&str> {
        self.resolutions.values().find_map(|r| {
            if let ResolutionAction::Rename { from_id } = &r.action
                && from_id == old_id
            {
                return Some(r.migration_id.as_str());
            }
            None
        })
    }

    /// Validate all resolutions.
    pub fn validate(&self) -> MigrateResult<Vec<ResolutionWarning>> {
        let mut warnings = Vec::new();

        for (id, resolution) in &self.resolutions {
            // Check for duplicate rename targets
            if let ResolutionAction::Rename { from_id } = &resolution.action {
                let count = self
                    .resolutions
                    .values()
                    .filter(|r| {
                        if let ResolutionAction::Rename { from_id: other } = &r.action {
                            other == from_id
                        } else {
                            false
                        }
                    })
                    .count();

                if count > 1 {
                    warnings.push(ResolutionWarning::DuplicateRename {
                        migration_id: id.clone(),
                        from_id: from_id.clone(),
                    });
                }
            }

            // Check for expired resolutions
            if let Some(expires_at) = resolution.expires_at
                && expires_at < Utc::now()
            {
                warnings.push(ResolutionWarning::Expired {
                    migration_id: id.clone(),
                    expired_at: expires_at,
                });
            }
        }

        Ok(warnings)
    }

    /// Merge another config into this one.
    pub fn merge(&mut self, other: ResolutionConfig) {
        for (id, resolution) in other.resolutions {
            self.resolutions.entry(id).or_insert(resolution);
        }
    }

    /// Count resolutions by type.
    pub fn count_by_type(&self) -> ResolutionCounts {
        let mut counts = ResolutionCounts::default();

        for resolution in self.resolutions.values() {
            match &resolution.action {
                ResolutionAction::AcceptChecksum { .. } => counts.checksum_accepted += 1,
                ResolutionAction::Skip => counts.skipped += 1,
                ResolutionAction::Baseline => counts.baseline += 1,
                ResolutionAction::Rename { .. } => counts.renamed += 1,
                ResolutionAction::ResolveConflict { .. } => counts.conflicts_resolved += 1,
                ResolutionAction::ForceApply => counts.force_applied += 1,
            }
        }

        counts
    }
}

/// A single migration resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    /// The migration ID this resolution applies to.
    pub migration_id: String,
    /// The action to take.
    pub action: ResolutionAction,
    /// Human-readable reason for this resolution.
    pub reason: String,
    /// When this resolution was created.
    pub created_at: DateTime<Utc>,
    /// Who created this resolution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    /// When this resolution expires (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// Additional metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl Resolution {
    /// Create a resolution to accept a checksum change.
    pub fn accept_checksum(
        migration_id: impl Into<String>,
        from_checksum: impl Into<String>,
        to_checksum: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            migration_id: migration_id.into(),
            action: ResolutionAction::AcceptChecksum {
                from_checksum: from_checksum.into(),
                to_checksum: to_checksum.into(),
            },
            reason: reason.into(),
            created_at: Utc::now(),
            created_by: None,
            expires_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a resolution to skip a migration.
    pub fn skip(migration_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            migration_id: migration_id.into(),
            action: ResolutionAction::Skip,
            reason: reason.into(),
            created_at: Utc::now(),
            created_by: None,
            expires_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a resolution to mark a migration as baseline.
    pub fn baseline(migration_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            migration_id: migration_id.into(),
            action: ResolutionAction::Baseline,
            reason: reason.into(),
            created_at: Utc::now(),
            created_by: None,
            expires_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a resolution to rename a migration.
    pub fn rename(
        new_id: impl Into<String>,
        old_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            migration_id: new_id.into(),
            action: ResolutionAction::Rename {
                from_id: old_id.into(),
            },
            reason: reason.into(),
            created_at: Utc::now(),
            created_by: None,
            expires_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a resolution to resolve a conflict.
    pub fn resolve_conflict(
        migration_id: impl Into<String>,
        conflicting_ids: Vec<String>,
        strategy: ConflictStrategy,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            migration_id: migration_id.into(),
            action: ResolutionAction::ResolveConflict {
                conflicting_ids,
                strategy,
            },
            reason: reason.into(),
            created_at: Utc::now(),
            created_by: None,
            expires_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a resolution to force apply a migration.
    pub fn force_apply(migration_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            migration_id: migration_id.into(),
            action: ResolutionAction::ForceApply,
            reason: reason.into(),
            created_at: Utc::now(),
            created_by: None,
            expires_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Set who created this resolution.
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.created_by = Some(author.into());
        self
    }

    /// Set when this resolution expires.
    pub fn with_expiration(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Add metadata to the resolution.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Check if this resolution has expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|e| e < Utc::now()).unwrap_or(false)
    }
}

/// The action to take for a resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResolutionAction {
    /// Accept a checksum change (migration was intentionally modified).
    AcceptChecksum {
        /// The old checksum.
        from_checksum: String,
        /// The new checksum.
        to_checksum: String,
    },
    /// Skip this migration entirely.
    Skip,
    /// Mark this migration as a baseline (applied without running).
    Baseline,
    /// Rename a migration (map old ID to new ID).
    Rename {
        /// The old migration ID.
        from_id: String,
    },
    /// Resolve a conflict between migrations.
    ResolveConflict {
        /// The conflicting migration IDs.
        conflicting_ids: Vec<String>,
        /// The strategy to resolve the conflict.
        strategy: ConflictStrategy,
    },
    /// Force apply this migration even if it would normally be blocked.
    ForceApply,
}

/// Strategy for resolving migration conflicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStrategy {
    /// Keep this migration, skip the conflicting ones.
    KeepThis,
    /// Merge the conflicting migrations into this one.
    Merge,
    /// Apply this migration first, then the conflicting ones.
    ApplyFirst,
    /// Apply this migration last, after the conflicting ones.
    ApplyLast,
}

/// Warning from resolution validation.
#[derive(Debug, Clone)]
pub enum ResolutionWarning {
    /// Multiple resolutions try to rename from the same ID.
    DuplicateRename {
        migration_id: String,
        from_id: String,
    },
    /// A resolution has expired.
    Expired {
        migration_id: String,
        expired_at: DateTime<Utc>,
    },
    /// A baseline resolution references a migration that doesn't exist.
    BaselineNotFound { migration_id: String },
    /// A rename resolution's source migration doesn't exist in history.
    RenameSourceNotFound {
        migration_id: String,
        from_id: String,
    },
}

impl std::fmt::Display for ResolutionWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateRename {
                migration_id,
                from_id,
            } => {
                write!(
                    f,
                    "Multiple resolutions rename from '{}' (found in '{}')",
                    from_id, migration_id
                )
            }
            Self::Expired {
                migration_id,
                expired_at,
            } => {
                write!(
                    f,
                    "Resolution for '{}' expired at {}",
                    migration_id, expired_at
                )
            }
            Self::BaselineNotFound { migration_id } => {
                write!(
                    f,
                    "Baseline migration '{}' not found in migration files",
                    migration_id
                )
            }
            Self::RenameSourceNotFound {
                migration_id,
                from_id,
            } => {
                write!(
                    f,
                    "Rename source '{}' not found in history (target: '{}')",
                    from_id, migration_id
                )
            }
        }
    }
}

/// Counts of resolutions by type.
#[derive(Debug, Clone, Default)]
pub struct ResolutionCounts {
    /// Number of accepted checksum changes.
    pub checksum_accepted: usize,
    /// Number of skipped migrations.
    pub skipped: usize,
    /// Number of baseline migrations.
    pub baseline: usize,
    /// Number of renamed migrations.
    pub renamed: usize,
    /// Number of resolved conflicts.
    pub conflicts_resolved: usize,
    /// Number of force-applied migrations.
    pub force_applied: usize,
}

impl ResolutionCounts {
    /// Get total number of resolutions.
    pub fn total(&self) -> usize {
        self.checksum_accepted
            + self.skipped
            + self.baseline
            + self.renamed
            + self.conflicts_resolved
            + self.force_applied
    }
}

/// Builder for creating resolutions interactively.
pub struct ResolutionBuilder {
    migration_id: String,
    action: Option<ResolutionAction>,
    reason: Option<String>,
    created_by: Option<String>,
    expires_at: Option<DateTime<Utc>>,
    metadata: HashMap<String, String>,
}

impl ResolutionBuilder {
    /// Create a new resolution builder.
    pub fn new(migration_id: impl Into<String>) -> Self {
        Self {
            migration_id: migration_id.into(),
            action: None,
            reason: None,
            created_by: None,
            expires_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the action to accept a checksum change.
    pub fn accept_checksum(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.action = Some(ResolutionAction::AcceptChecksum {
            from_checksum: from.into(),
            to_checksum: to.into(),
        });
        self
    }

    /// Set the action to skip.
    pub fn skip(mut self) -> Self {
        self.action = Some(ResolutionAction::Skip);
        self
    }

    /// Set the action to baseline.
    pub fn baseline(mut self) -> Self {
        self.action = Some(ResolutionAction::Baseline);
        self
    }

    /// Set the action to rename.
    pub fn rename_from(mut self, old_id: impl Into<String>) -> Self {
        self.action = Some(ResolutionAction::Rename {
            from_id: old_id.into(),
        });
        self
    }

    /// Set the action to resolve conflict.
    pub fn resolve_conflict(
        mut self,
        conflicting: Vec<String>,
        strategy: ConflictStrategy,
    ) -> Self {
        self.action = Some(ResolutionAction::ResolveConflict {
            conflicting_ids: conflicting,
            strategy,
        });
        self
    }

    /// Set the action to force apply.
    pub fn force_apply(mut self) -> Self {
        self.action = Some(ResolutionAction::ForceApply);
        self
    }

    /// Set the reason.
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Set the author.
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.created_by = Some(author.into());
        self
    }

    /// Set the expiration.
    pub fn expires(mut self, at: DateTime<Utc>) -> Self {
        self.expires_at = Some(at);
        self
    }

    /// Add metadata.
    pub fn meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Build the resolution.
    pub fn build(self) -> MigrateResult<Resolution> {
        let action = self.action.ok_or_else(|| {
            MigrationError::ResolutionFile("Resolution action is required".to_string())
        })?;

        let reason = self.reason.ok_or_else(|| {
            MigrationError::ResolutionFile("Resolution reason is required".to_string())
        })?;

        Ok(Resolution {
            migration_id: self.migration_id,
            action,
            reason,
            created_at: Utc::now(),
            created_by: self.created_by,
            expires_at: self.expires_at,
            metadata: self.metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolution_accept_checksum() {
        let resolution = Resolution::accept_checksum(
            "20240101_create_users",
            "abc123",
            "def456",
            "Fixed typo in column name",
        );

        assert_eq!(resolution.migration_id, "20240101_create_users");
        assert!(matches!(
            resolution.action,
            ResolutionAction::AcceptChecksum { .. }
        ));
    }

    #[test]
    fn test_resolution_skip() {
        let resolution = Resolution::skip(
            "20240102_add_legacy_table",
            "Already exists from legacy system",
        );

        assert_eq!(resolution.migration_id, "20240102_add_legacy_table");
        assert!(matches!(resolution.action, ResolutionAction::Skip));
    }

    #[test]
    fn test_resolution_baseline() {
        let resolution =
            Resolution::baseline("20240103_initial_schema", "Imported from existing database");

        assert!(matches!(resolution.action, ResolutionAction::Baseline));
    }

    #[test]
    fn test_resolution_rename() {
        let resolution = Resolution::rename(
            "20240104_new_name",
            "20240104_old_name",
            "Renamed for clarity",
        );

        if let ResolutionAction::Rename { from_id } = &resolution.action {
            assert_eq!(from_id, "20240104_old_name");
        } else {
            panic!("Expected Rename action");
        }
    }

    #[test]
    fn test_resolution_config() {
        let mut config = ResolutionConfig::new();

        config.add(Resolution::skip("migration_1", "Skip reason"));
        config.add(Resolution::baseline("migration_2", "Baseline reason"));

        assert!(config.has_resolution("migration_1"));
        assert!(config.should_skip("migration_1"));
        assert!(config.is_baseline("migration_2"));
        assert!(!config.should_skip("migration_2"));
    }

    #[test]
    fn test_resolution_accepts_checksum() {
        let mut config = ResolutionConfig::new();

        config.add(Resolution::accept_checksum(
            "migration_1",
            "old_hash",
            "new_hash",
            "Fixed typo",
        ));

        assert!(config.accepts_checksum("migration_1", "old_hash", "new_hash"));
        assert!(!config.accepts_checksum("migration_1", "wrong", "new_hash"));
        assert!(!config.accepts_checksum("migration_2", "old_hash", "new_hash"));
    }

    #[test]
    fn test_resolution_skipped_list() {
        let mut config = ResolutionConfig::new();

        config.add(Resolution::skip("skip_1", "Reason 1"));
        config.add(Resolution::skip("skip_2", "Reason 2"));
        config.add(Resolution::baseline("baseline_1", "Reason 3"));

        let skipped = config.skipped();
        assert_eq!(skipped.len(), 2);
        assert!(skipped.contains(&"skip_1"));
        assert!(skipped.contains(&"skip_2"));
    }

    #[test]
    fn test_resolution_builder() {
        let resolution = ResolutionBuilder::new("migration_1")
            .skip()
            .reason("Testing")
            .author("Test User")
            .meta("ticket", "JIRA-123")
            .build()
            .unwrap();

        assert_eq!(resolution.migration_id, "migration_1");
        assert!(matches!(resolution.action, ResolutionAction::Skip));
        assert_eq!(resolution.reason, "Testing");
        assert_eq!(resolution.created_by, Some("Test User".to_string()));
        assert_eq!(
            resolution.metadata.get("ticket"),
            Some(&"JIRA-123".to_string())
        );
    }

    #[test]
    fn test_resolution_counts() {
        let mut config = ResolutionConfig::new();

        config.add(Resolution::skip("m1", "r"));
        config.add(Resolution::skip("m2", "r"));
        config.add(Resolution::baseline("m3", "r"));
        config.add(Resolution::accept_checksum("m4", "a", "b", "r"));

        let counts = config.count_by_type();
        assert_eq!(counts.skipped, 2);
        assert_eq!(counts.baseline, 1);
        assert_eq!(counts.checksum_accepted, 1);
        assert_eq!(counts.total(), 4);
    }

    #[test]
    fn test_resolution_expiration() {
        let expired = Resolution::skip("migration", "reason")
            .with_expiration(Utc::now() - chrono::Duration::hours(1));

        assert!(expired.is_expired());

        let valid = Resolution::skip("migration", "reason")
            .with_expiration(Utc::now() + chrono::Duration::hours(1));

        assert!(!valid.is_expired());
    }

    #[test]
    fn test_get_renamed() {
        let mut config = ResolutionConfig::new();
        config.add(Resolution::rename("new_id", "old_id", "Renamed"));

        assert_eq!(config.get_renamed("old_id"), Some("new_id"));
        assert_eq!(config.get_renamed("unknown"), None);
    }
}
