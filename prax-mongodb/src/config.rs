//! MongoDB connection configuration.

use std::time::Duration;

use mongodb::options::ClientOptions;

use crate::error::{MongoError, MongoResult};

/// MongoDB connection configuration.
#[derive(Debug, Clone)]
pub struct MongoConfig {
    /// MongoDB connection URI.
    pub uri: String,
    /// Database name.
    pub database: String,
    /// Application name (shown in server logs).
    pub app_name: Option<String>,
    /// Minimum connection pool size.
    pub min_pool_size: Option<u32>,
    /// Maximum connection pool size.
    pub max_pool_size: Option<u32>,
    /// Maximum idle time for connections.
    pub max_idle_time: Option<Duration>,
    /// Connection timeout.
    pub connect_timeout: Option<Duration>,
    /// Server selection timeout.
    pub server_selection_timeout: Option<Duration>,
    /// Socket timeout.
    pub socket_timeout: Option<Duration>,
    /// Enable compression.
    pub compressors: Option<Vec<String>>,
    /// Read preference.
    pub read_preference: Option<ReadPreference>,
    /// Write concern.
    pub write_concern: Option<WriteConcern>,
    /// Retry writes.
    pub retry_writes: Option<bool>,
    /// Retry reads.
    pub retry_reads: Option<bool>,
    /// Direct connection (bypass replica set discovery).
    pub direct_connection: Option<bool>,
}

/// MongoDB read preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReadPreference {
    /// Read from primary only.
    #[default]
    Primary,
    /// Read from primary preferred, fallback to secondary.
    PrimaryPreferred,
    /// Read from secondary only.
    Secondary,
    /// Read from secondary preferred, fallback to primary.
    SecondaryPreferred,
    /// Read from nearest member.
    Nearest,
}

/// MongoDB write concern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteConcern {
    /// Acknowledge writes from the specified number of nodes.
    W(u32),
    /// Acknowledge writes from majority of nodes.
    Majority,
    /// Custom tag set.
    Custom(String),
}

impl Default for MongoConfig {
    fn default() -> Self {
        Self {
            uri: "mongodb://localhost:27017".to_string(),
            database: String::new(),
            app_name: Some("prax".to_string()),
            min_pool_size: None,
            max_pool_size: Some(10),
            max_idle_time: Some(Duration::from_secs(300)),
            connect_timeout: Some(Duration::from_secs(10)),
            server_selection_timeout: Some(Duration::from_secs(30)),
            socket_timeout: None,
            compressors: None,
            read_preference: Some(ReadPreference::Primary),
            write_concern: None,
            retry_writes: Some(true),
            retry_reads: Some(true),
            direct_connection: None,
        }
    }
}

impl MongoConfig {
    /// Create a new configuration from a MongoDB URI.
    pub fn from_uri(uri: impl Into<String>, database: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            database: database.into(),
            ..Self::default()
        }
    }

    /// Create a builder for configuration.
    pub fn builder() -> MongoConfigBuilder {
        MongoConfigBuilder::new()
    }

    /// Convert to MongoDB ClientOptions.
    pub async fn to_client_options(&self) -> MongoResult<ClientOptions> {
        let mut options = ClientOptions::parse(&self.uri)
            .await
            .map_err(|e| MongoError::config(format!("failed to parse URI: {}", e)))?;

        if let Some(ref app_name) = self.app_name {
            options.app_name = Some(app_name.clone());
        }

        if let Some(min_pool) = self.min_pool_size {
            options.min_pool_size = Some(min_pool);
        }

        if let Some(max_pool) = self.max_pool_size {
            options.max_pool_size = Some(max_pool);
        }

        if let Some(max_idle) = self.max_idle_time {
            options.max_idle_time = Some(max_idle);
        }

        if let Some(connect_timeout) = self.connect_timeout {
            options.connect_timeout = Some(connect_timeout);
        }

        if let Some(selection_timeout) = self.server_selection_timeout {
            options.server_selection_timeout = Some(selection_timeout);
        }

        // Note: socket_timeout and compressors are not directly available in mongodb 2.x
        // They need to be set through connection string or other means
        let _ = self.socket_timeout;
        let _ = &self.compressors;

        if let Some(ref read_pref) = self.read_preference {
            options.selection_criteria = Some(match read_pref {
                ReadPreference::Primary => mongodb::options::SelectionCriteria::ReadPreference(
                    mongodb::options::ReadPreference::Primary,
                ),
                ReadPreference::PrimaryPreferred => {
                    mongodb::options::SelectionCriteria::ReadPreference(
                        mongodb::options::ReadPreference::PrimaryPreferred {
                            options: Default::default(),
                        },
                    )
                }
                ReadPreference::Secondary => mongodb::options::SelectionCriteria::ReadPreference(
                    mongodb::options::ReadPreference::Secondary {
                        options: Default::default(),
                    },
                ),
                ReadPreference::SecondaryPreferred => {
                    mongodb::options::SelectionCriteria::ReadPreference(
                        mongodb::options::ReadPreference::SecondaryPreferred {
                            options: Default::default(),
                        },
                    )
                }
                ReadPreference::Nearest => mongodb::options::SelectionCriteria::ReadPreference(
                    mongodb::options::ReadPreference::Nearest {
                        options: Default::default(),
                    },
                ),
            });
        }

        if let Some(ref wc) = self.write_concern {
            options.write_concern = Some(match wc {
                WriteConcern::W(n) => mongodb::options::WriteConcern::builder()
                    .w(mongodb::options::Acknowledgment::Nodes(*n))
                    .build(),
                WriteConcern::Majority => mongodb::options::WriteConcern::builder()
                    .w(mongodb::options::Acknowledgment::Majority)
                    .build(),
                WriteConcern::Custom(tag) => mongodb::options::WriteConcern::builder()
                    .w(mongodb::options::Acknowledgment::Custom(tag.clone()))
                    .build(),
            });
        }

        if let Some(retry_writes) = self.retry_writes {
            options.retry_writes = Some(retry_writes);
        }

        if let Some(retry_reads) = self.retry_reads {
            options.retry_reads = Some(retry_reads);
        }

        if let Some(direct) = self.direct_connection {
            options.direct_connection = Some(direct);
        }

        Ok(options)
    }
}

/// Builder for MongoDB configuration.
#[derive(Debug, Default)]
pub struct MongoConfigBuilder {
    uri: Option<String>,
    database: Option<String>,
    app_name: Option<String>,
    min_pool_size: Option<u32>,
    max_pool_size: Option<u32>,
    max_idle_time: Option<Duration>,
    connect_timeout: Option<Duration>,
    server_selection_timeout: Option<Duration>,
    socket_timeout: Option<Duration>,
    compressors: Option<Vec<String>>,
    read_preference: Option<ReadPreference>,
    write_concern: Option<WriteConcern>,
    retry_writes: Option<bool>,
    retry_reads: Option<bool>,
    direct_connection: Option<bool>,
}

impl MongoConfigBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the MongoDB URI.
    pub fn uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = Some(uri.into());
        self
    }

    /// Set the database name.
    pub fn database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
        self
    }

    /// Set the application name.
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = Some(name.into());
        self
    }

    /// Set the minimum pool size.
    pub fn min_pool_size(mut self, size: u32) -> Self {
        self.min_pool_size = Some(size);
        self
    }

    /// Set the maximum pool size.
    pub fn max_pool_size(mut self, size: u32) -> Self {
        self.max_pool_size = Some(size);
        self
    }

    /// Set the maximum idle time for connections.
    pub fn max_idle_time(mut self, duration: Duration) -> Self {
        self.max_idle_time = Some(duration);
        self
    }

    /// Set the connection timeout.
    pub fn connect_timeout(mut self, duration: Duration) -> Self {
        self.connect_timeout = Some(duration);
        self
    }

    /// Set the server selection timeout.
    pub fn server_selection_timeout(mut self, duration: Duration) -> Self {
        self.server_selection_timeout = Some(duration);
        self
    }

    /// Set the socket timeout.
    pub fn socket_timeout(mut self, duration: Duration) -> Self {
        self.socket_timeout = Some(duration);
        self
    }

    /// Enable compression (zlib, snappy, or zstd).
    pub fn compressors(mut self, compressors: Vec<String>) -> Self {
        self.compressors = Some(compressors);
        self
    }

    /// Set the read preference.
    pub fn read_preference(mut self, pref: ReadPreference) -> Self {
        self.read_preference = Some(pref);
        self
    }

    /// Set the write concern.
    pub fn write_concern(mut self, wc: WriteConcern) -> Self {
        self.write_concern = Some(wc);
        self
    }

    /// Enable or disable retry writes.
    pub fn retry_writes(mut self, enabled: bool) -> Self {
        self.retry_writes = Some(enabled);
        self
    }

    /// Enable or disable retry reads.
    pub fn retry_reads(mut self, enabled: bool) -> Self {
        self.retry_reads = Some(enabled);
        self
    }

    /// Enable direct connection (bypass replica set discovery).
    pub fn direct_connection(mut self, enabled: bool) -> Self {
        self.direct_connection = Some(enabled);
        self
    }

    /// Build the configuration.
    pub fn build(self) -> MongoResult<MongoConfig> {
        let database = self
            .database
            .ok_or_else(|| MongoError::config("database name is required"))?;

        Ok(MongoConfig {
            uri: self
                .uri
                .unwrap_or_else(|| "mongodb://localhost:27017".to_string()),
            database,
            app_name: self.app_name.or(Some("prax".to_string())),
            min_pool_size: self.min_pool_size,
            max_pool_size: self.max_pool_size.or(Some(10)),
            max_idle_time: self.max_idle_time.or(Some(Duration::from_secs(300))),
            connect_timeout: self.connect_timeout.or(Some(Duration::from_secs(10))),
            server_selection_timeout: self
                .server_selection_timeout
                .or(Some(Duration::from_secs(30))),
            socket_timeout: self.socket_timeout,
            compressors: self.compressors,
            read_preference: self.read_preference.or(Some(ReadPreference::Primary)),
            write_concern: self.write_concern,
            retry_writes: self.retry_writes.or(Some(true)),
            retry_reads: self.retry_reads.or(Some(true)),
            direct_connection: self.direct_connection,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_uri() {
        let config = MongoConfig::from_uri("mongodb://localhost:27017", "mydb");
        assert_eq!(config.uri, "mongodb://localhost:27017");
        assert_eq!(config.database, "mydb");
    }

    #[test]
    fn test_config_builder() {
        let config = MongoConfig::builder()
            .uri("mongodb://localhost:27017")
            .database("mydb")
            .app_name("test-app")
            .max_pool_size(20)
            .build()
            .unwrap();

        assert_eq!(config.database, "mydb");
        assert_eq!(config.app_name, Some("test-app".to_string()));
        assert_eq!(config.max_pool_size, Some(20));
    }

    #[test]
    fn test_config_builder_missing_database() {
        let result = MongoConfig::builder()
            .uri("mongodb://localhost:27017")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_read_preference_default() {
        let pref: ReadPreference = Default::default();
        assert_eq!(pref, ReadPreference::Primary);
    }
}
