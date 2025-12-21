//! SQLite configuration.

use std::path::{Path, PathBuf};

use crate::error::{SqliteError, SqliteResult};

/// SQLite database configuration.
#[derive(Debug, Clone)]
pub struct SqliteConfig {
    /// Database path (or ":memory:" for in-memory).
    pub path: DatabasePath,
    /// Enable foreign keys.
    pub foreign_keys: bool,
    /// Enable WAL mode.
    pub wal_mode: bool,
    /// Busy timeout in milliseconds.
    pub busy_timeout_ms: Option<u32>,
    /// Cache size (in pages, negative for KB).
    pub cache_size: Option<i32>,
    /// Synchronous mode.
    pub synchronous: SynchronousMode,
    /// Journal mode.
    pub journal_mode: JournalMode,
}

/// Database path configuration.
#[derive(Debug, Clone)]
#[derive(Default)]
pub enum DatabasePath {
    /// In-memory database.
    #[default]
    Memory,
    /// File-based database.
    File(PathBuf),
}

impl DatabasePath {
    /// Get the path string for SQLite.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Memory => ":memory:",
            Self::File(path) => path.to_str().unwrap_or(":memory:"),
        }
    }

    /// Check if this is an in-memory database.
    pub fn is_memory(&self) -> bool {
        matches!(self, Self::Memory)
    }
}


/// SQLite synchronous mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SynchronousMode {
    /// Synchronous OFF - Fastest but unsafe.
    Off,
    /// Synchronous NORMAL - Good balance.
    #[default]
    Normal,
    /// Synchronous FULL - Safe but slower.
    Full,
    /// Synchronous EXTRA - Maximum safety.
    Extra,
}

impl SynchronousMode {
    /// Get the SQLite pragma value.
    pub fn as_pragma(&self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::Normal => "NORMAL",
            Self::Full => "FULL",
            Self::Extra => "EXTRA",
        }
    }
}

/// SQLite journal mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum JournalMode {
    /// DELETE - Default mode, deletes journal after transaction.
    Delete,
    /// TRUNCATE - Truncates journal instead of deleting.
    Truncate,
    /// PERSIST - Keep journal file, zero out on commit.
    Persist,
    /// MEMORY - Keep journal in memory.
    Memory,
    /// WAL - Write-Ahead Logging (best for concurrent access).
    #[default]
    Wal,
    /// OFF - No journal (dangerous).
    Off,
}

impl JournalMode {
    /// Get the SQLite pragma value.
    pub fn as_pragma(&self) -> &'static str {
        match self {
            Self::Delete => "DELETE",
            Self::Truncate => "TRUNCATE",
            Self::Persist => "PERSIST",
            Self::Memory => "MEMORY",
            Self::Wal => "WAL",
            Self::Off => "OFF",
        }
    }
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            path: DatabasePath::Memory,
            foreign_keys: true,
            wal_mode: true,
            busy_timeout_ms: Some(5000),
            cache_size: Some(-2000), // 2MB cache
            synchronous: SynchronousMode::Normal,
            journal_mode: JournalMode::Wal,
        }
    }
}

impl SqliteConfig {
    /// Create a new configuration for an in-memory database.
    pub fn memory() -> Self {
        Self {
            path: DatabasePath::Memory,
            ..Default::default()
        }
    }

    /// Create a new configuration for a file-based database.
    pub fn file(path: impl AsRef<Path>) -> Self {
        Self {
            path: DatabasePath::File(path.as_ref().to_path_buf()),
            ..Default::default()
        }
    }

    /// Parse a SQLite URL into configuration.
    ///
    /// Supported formats:
    /// - `sqlite::memory:` - In-memory database
    /// - `sqlite://path/to/db.sqlite` - File-based database
    /// - `sqlite:///absolute/path/db.sqlite` - Absolute path
    /// - `file:path/to/db.sqlite` - Alternative format
    pub fn from_url(url: impl AsRef<str>) -> SqliteResult<Self> {
        let url_str = url.as_ref();

        // Handle special memory URL
        if url_str == "sqlite::memory:" || url_str == ":memory:" {
            return Ok(Self::memory());
        }

        // Parse the URL
        let path = if let Some(path_part) = url_str.strip_prefix("sqlite://") {
            // Handle query parameters
            let path_only = path_part.split('?').next().unwrap_or(path_part);
            if path_only.is_empty() {
                return Err(SqliteError::config("database path is required"));
            }
            path_only.to_string()
        } else if let Some(path_part) = url_str.strip_prefix("sqlite:") {
            let path_only = path_part.split('?').next().unwrap_or(path_part);
            if path_only == ":memory:" {
                return Ok(Self::memory());
            }
            path_only.to_string()
        } else if let Some(path_part) = url_str.strip_prefix("file:") {
            let path_only = path_part.split('?').next().unwrap_or(path_part);
            path_only.to_string()
        } else {
            // Assume it's a direct file path
            url_str.to_string()
        };

        let mut config = Self::file(&path);

        // Parse query parameters if present
        if let Some(query_start) = url_str.find('?') {
            let query = &url_str[query_start + 1..];
            for pair in query.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    match key {
                        "mode" if value == "memory" => {
                            config.path = DatabasePath::Memory;
                        }
                        "foreign_keys" => {
                            config.foreign_keys = value == "true" || value == "1";
                        }
                        "wal_mode" => {
                            config.wal_mode = value == "true" || value == "1";
                        }
                        "busy_timeout" => {
                            if let Ok(ms) = value.parse() {
                                config.busy_timeout_ms = Some(ms);
                            }
                        }
                        "cache_size" => {
                            if let Ok(size) = value.parse() {
                                config.cache_size = Some(size);
                            }
                        }
                        "synchronous" => {
                            config.synchronous = match value.to_lowercase().as_str() {
                                "off" => SynchronousMode::Off,
                                "normal" => SynchronousMode::Normal,
                                "full" => SynchronousMode::Full,
                                "extra" => SynchronousMode::Extra,
                                _ => SynchronousMode::Normal,
                            };
                        }
                        "journal_mode" => {
                            config.journal_mode = match value.to_lowercase().as_str() {
                                "delete" => JournalMode::Delete,
                                "truncate" => JournalMode::Truncate,
                                "persist" => JournalMode::Persist,
                                "memory" => JournalMode::Memory,
                                "wal" => JournalMode::Wal,
                                "off" => JournalMode::Off,
                                _ => JournalMode::Wal,
                            };
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(config)
    }

    /// Get the path string for SQLite.
    pub fn path_str(&self) -> &str {
        self.path.as_str()
    }

    /// Generate the initialization SQL for this configuration.
    pub fn init_sql(&self) -> String {
        let mut sql = String::new();

        if self.foreign_keys {
            sql.push_str("PRAGMA foreign_keys = ON;\n");
        }

        sql.push_str(&format!(
            "PRAGMA journal_mode = {};\n",
            self.journal_mode.as_pragma()
        ));

        sql.push_str(&format!(
            "PRAGMA synchronous = {};\n",
            self.synchronous.as_pragma()
        ));

        if let Some(timeout) = self.busy_timeout_ms {
            sql.push_str(&format!("PRAGMA busy_timeout = {};\n", timeout));
        }

        if let Some(cache) = self.cache_size {
            sql.push_str(&format!("PRAGMA cache_size = {};\n", cache));
        }

        sql
    }

    /// Set the database path.
    pub fn path(mut self, path: DatabasePath) -> Self {
        self.path = path;
        self
    }

    /// Enable or disable foreign keys.
    pub fn foreign_keys(mut self, enabled: bool) -> Self {
        self.foreign_keys = enabled;
        self
    }

    /// Enable or disable WAL mode.
    pub fn wal_mode(mut self, enabled: bool) -> Self {
        self.wal_mode = enabled;
        if enabled {
            self.journal_mode = JournalMode::Wal;
        }
        self
    }

    /// Set the busy timeout in milliseconds.
    pub fn busy_timeout(mut self, ms: u32) -> Self {
        self.busy_timeout_ms = Some(ms);
        self
    }

    /// Set the cache size.
    pub fn cache_size(mut self, size: i32) -> Self {
        self.cache_size = Some(size);
        self
    }

    /// Set the synchronous mode.
    pub fn synchronous(mut self, mode: SynchronousMode) -> Self {
        self.synchronous = mode;
        self
    }

    /// Set the journal mode.
    pub fn journal_mode(mut self, mode: JournalMode) -> Self {
        self.journal_mode = mode;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_memory() {
        let config = SqliteConfig::memory();
        assert!(config.path.is_memory());
        assert_eq!(config.path.as_str(), ":memory:");
    }

    #[test]
    fn test_config_file() {
        let config = SqliteConfig::file("test.db");
        assert!(!config.path.is_memory());
        assert_eq!(config.path.as_str(), "test.db");
    }

    #[test]
    fn test_config_from_url_memory() {
        let config = SqliteConfig::from_url("sqlite::memory:").unwrap();
        assert!(config.path.is_memory());

        let config = SqliteConfig::from_url(":memory:").unwrap();
        assert!(config.path.is_memory());
    }

    #[test]
    fn test_config_from_url_file() {
        let config = SqliteConfig::from_url("sqlite://./test.db").unwrap();
        assert!(!config.path.is_memory());
        assert_eq!(config.path.as_str(), "./test.db");
    }

    #[test]
    fn test_config_from_url_with_options() {
        let config = SqliteConfig::from_url(
            "sqlite://./test.db?foreign_keys=true&busy_timeout=10000&synchronous=full",
        )
        .unwrap();

        assert!(config.foreign_keys);
        assert_eq!(config.busy_timeout_ms, Some(10000));
        assert_eq!(config.synchronous, SynchronousMode::Full);
    }

    #[test]
    fn test_init_sql() {
        let config = SqliteConfig::default();
        let sql = config.init_sql();

        assert!(sql.contains("foreign_keys = ON"));
        assert!(sql.contains("journal_mode = WAL"));
        assert!(sql.contains("synchronous = NORMAL"));
    }

    #[test]
    fn test_builder_pattern() {
        let config = SqliteConfig::memory()
            .foreign_keys(false)
            .busy_timeout(3000)
            .synchronous(SynchronousMode::Full)
            .journal_mode(JournalMode::Memory);

        assert!(!config.foreign_keys);
        assert_eq!(config.busy_timeout_ms, Some(3000));
        assert_eq!(config.synchronous, SynchronousMode::Full);
        assert_eq!(config.journal_mode, JournalMode::Memory);
    }

    #[test]
    fn test_synchronous_mode_pragma() {
        assert_eq!(SynchronousMode::Off.as_pragma(), "OFF");
        assert_eq!(SynchronousMode::Normal.as_pragma(), "NORMAL");
        assert_eq!(SynchronousMode::Full.as_pragma(), "FULL");
        assert_eq!(SynchronousMode::Extra.as_pragma(), "EXTRA");
    }

    #[test]
    fn test_journal_mode_pragma() {
        assert_eq!(JournalMode::Delete.as_pragma(), "DELETE");
        assert_eq!(JournalMode::Wal.as_pragma(), "WAL");
        assert_eq!(JournalMode::Memory.as_pragma(), "MEMORY");
    }
}
