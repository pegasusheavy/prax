//! Armature framework integration for Prax ORM.
//!
//! This crate provides seamless integration between Prax ORM and the
//! [Armature](https://github.com/quinnjr/armature) HTTP framework.
//!
//! # Features
//!
//! - **Dependency Injection**: Register `PraxClient` as a singleton provider
//! - **Configuration Management**: Parse and validate database configuration
//!   once, then share it through the DI container
//! - **Request-scoped Transactions**: Transaction markers via DI container
//!   (engine-backed transactions are not yet wired up)
//! - **Middleware**: `DatabaseMiddleware` holds the client for injection
//!   (a middleware trait impl is not yet provided)
//!
//! # Example
//!
//! ```rust,ignore
//! use armature::prelude::*;
//! use prax_armature::{PraxClient, PraxClientBuilder};
//!
//! #[module_impl]
//! impl DatabaseModule {
//!     #[provider(singleton)]
//!     async fn prax_client() -> Arc<PraxClient> {
//!         PraxClientBuilder::new()
//!             .url("postgresql://localhost/mydb")
//!             .build()
//!             .await
//!             .expect("invalid database configuration")
//!     }
//! }
//!
//! #[module(
//!     imports = [DatabaseModule],
//!     controllers = [UserController],
//! )]
//! struct AppModule;
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;
use thiserror::Error;
use tracing::{debug, info};

use prax_query::connection::{DatabaseConfig, PoolConfig};

// Re-export key types
pub use prax_query::filter::{Filter, FilterValue};
pub use prax_query::prelude::*;

/// Errors that can occur during Prax-Armature integration.
#[derive(Error, Debug)]
pub enum PraxArmatureError {
    /// Failed to connect to the database.
    #[error("database connection failed: {0}")]
    ConnectionFailed(String),

    /// Transaction error.
    #[error("transaction error: {0}")]
    TransactionError(String),

    /// Configuration error.
    #[error("configuration error: {0}")]
    ConfigError(String),

    /// Pool exhausted.
    #[error("connection pool exhausted")]
    PoolExhausted,
}

/// Result type for Prax-Armature operations.
pub type Result<T> = std::result::Result<T, PraxArmatureError>;

/// A database client configuration holder that can be injected via Armature's DI system.
///
/// `PraxClient` stores a parsed [`DatabaseConfig`] and [`PoolConfig`] so they
/// can be built once and shared across providers. It does **not** open a
/// connection, create a pool, or expose query methods: the optional
/// `prax-postgres` / `prax-mysql` / `prax-sqlite` engine dependencies are
/// declared but not yet wired up. Until engine construction lands, use it to
/// centralize and validate database configuration for DI wiring.
///
/// # Example
///
/// ```rust,ignore
/// use prax_armature::PraxClient;
///
/// #[module_impl]
/// impl DatabaseModule {
///     #[provider(singleton)]
///     async fn database() -> Arc<PraxClient> {
///         PraxClient::connect("postgresql://localhost/mydb")
///             .await
///             .expect("invalid database URL")
///     }
/// }
/// ```
#[derive(Debug)]
pub struct PraxClient {
    config: DatabaseConfig,
    pool_config: PoolConfig,
}

impl PraxClient {
    /// Create a new PraxClient from a connection URL.
    ///
    /// This parses and validates the URL into a [`DatabaseConfig`] and stores
    /// it; **no database connection or pool is created**. Invalid URLs are
    /// rejected here so configuration errors surface at startup. See the
    /// struct-level docs for the current engine-wiring status.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let client = PraxClient::connect("postgresql://user:pass@localhost/mydb").await?;
    /// ```
    pub async fn connect(url: &str) -> Result<Arc<Self>> {
        debug!(
            url_len = url.len(),
            "PraxClient parsing database configuration"
        );

        let config = DatabaseConfig::from_url(url)
            .map_err(|e| PraxArmatureError::ConnectionFailed(e.to_string()))?;

        let client = Self {
            config,
            pool_config: PoolConfig::default(),
        };

        debug!("PraxClient configuration parsed and stored");
        Ok(Arc::new(client))
    }

    /// Create a new PraxClient from environment variables.
    ///
    /// Reads `DATABASE_URL` from the environment.
    pub async fn from_env() -> Result<Arc<Self>> {
        info!("PraxClient loading configuration from DATABASE_URL");

        let config = DatabaseConfig::from_env()
            .map_err(|e| PraxArmatureError::ConfigError(e.to_string()))?;

        let client = Self {
            config,
            pool_config: PoolConfig::default(),
        };

        info!("PraxClient configuration loaded from environment");
        Ok(Arc::new(client))
    }

    /// Create a new PraxClient with custom configuration.
    pub fn with_config(config: DatabaseConfig) -> Arc<Self> {
        info!(driver = %config.driver.name(), "PraxClient created with custom config");
        Arc::new(Self {
            config,
            pool_config: PoolConfig::default(),
        })
    }

    /// Set the pool configuration.
    pub fn with_pool_config(mut self, pool_config: PoolConfig) -> Self {
        self.pool_config = pool_config;
        self
    }

    /// Get the database configuration.
    pub fn config(&self) -> &DatabaseConfig {
        &self.config
    }

    /// Get the pool configuration.
    pub fn pool_config(&self) -> &PoolConfig {
        &self.pool_config
    }
}

/// Trait for types that can provide a database connection.
///
/// This is used by Armature's DI system to inject database connections.
#[async_trait]
pub trait DatabaseProvider: Send + Sync {
    /// Get a database client.
    fn client(&self) -> &PraxClient;
}

#[async_trait]
impl DatabaseProvider for Arc<PraxClient> {
    fn client(&self) -> &PraxClient {
        self.as_ref()
    }
}

/// A request-scoped transaction marker.
///
/// This type is a DI-scoped placeholder that records commit intent for a
/// request; it does **not** hold a real database transaction. [`commit`](Self::commit)
/// only flips an internal flag and `Drop` only logs — no SQL is issued and
/// nothing is committed or rolled back against a database. Real engine
/// transactions arrive with the engine wiring tracked on [`PraxClient`].
///
/// # Example
///
/// ```rust,ignore
/// #[controller("/users")]
/// impl UserController {
///     #[post("/transfer")]
///     async fn transfer(
///         &self,
///         #[inject] tx: RequestTransaction,
///     ) -> Result<Json<()>, HttpError> {
///         // Marker only: no database transaction is active yet.
///         let _client = tx.client();
///         tx.commit(); // records commit intent for this request scope
///         Ok(Json(()))
///     }
/// }
/// ```
pub struct RequestTransaction {
    /// The underlying client.
    client: Arc<PraxClient>,
    /// Whether the transaction has been committed.
    committed: RwLock<bool>,
}

impl RequestTransaction {
    /// Create a new request-scoped transaction.
    pub fn new(client: Arc<PraxClient>) -> Self {
        debug!("RequestTransaction created");
        Self {
            client,
            committed: RwLock::new(false),
        }
    }

    /// Get the underlying client.
    pub fn client(&self) -> &PraxClient {
        self.client.as_ref()
    }

    /// Mark the transaction as committed.
    ///
    /// Records commit intent only; no database work is performed.
    pub fn commit(&self) {
        let mut committed = self.committed.write();
        *committed = true;
        debug!("RequestTransaction committed");
    }

    /// Check if the transaction has been committed.
    pub fn is_committed(&self) -> bool {
        *self.committed.read()
    }
}

impl Drop for RequestTransaction {
    fn drop(&mut self) {
        if !self.is_committed() {
            debug!("RequestTransaction rolled back (not committed)");
        }
    }
}

/// Middleware-shaped holder for database client injection.
///
/// This type currently only holds the [`PraxClient`] so it can be injected
/// where per-request database access is needed; a middleware trait impl is
/// not yet provided, so it performs no connection handling on its own today.
///
/// # Example
///
/// ```rust,ignore
/// use prax_armature::DatabaseMiddleware;
///
/// let middleware = DatabaseMiddleware::new(client);
/// let client = middleware.client();
/// ```
pub struct DatabaseMiddleware {
    client: Arc<PraxClient>,
}

impl DatabaseMiddleware {
    /// Create a new database middleware.
    pub fn new(client: Arc<PraxClient>) -> Self {
        info!("DatabaseMiddleware initialized");
        Self { client }
    }

    /// Get the underlying client.
    pub fn client(&self) -> &PraxClient {
        self.client.as_ref()
    }
}

/// Builder for configuring PraxClient with Armature.
///
/// # Example
///
/// ```rust,ignore
/// let client = PraxClientBuilder::new()
///     .url("postgresql://localhost/mydb")
///     .pool_config(PoolConfig::read_heavy())
///     .build()
///     .await?;
/// ```
pub struct PraxClientBuilder {
    url: Option<String>,
    config: Option<DatabaseConfig>,
    pool_config: PoolConfig,
}

impl PraxClientBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            url: None,
            config: None,
            pool_config: PoolConfig::default(),
        }
    }

    /// Set the database URL.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set the database configuration directly.
    pub fn config(mut self, config: DatabaseConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Set the pool configuration.
    pub fn pool_config(mut self, pool_config: PoolConfig) -> Self {
        self.pool_config = pool_config;
        self
    }

    /// Use read-heavy pool configuration.
    pub fn read_heavy(mut self) -> Self {
        self.pool_config = PoolConfig::read_heavy();
        self
    }

    /// Use write-heavy pool configuration.
    pub fn write_heavy(mut self) -> Self {
        self.pool_config = PoolConfig::write_heavy();
        self
    }

    /// Use serverless pool configuration.
    pub fn serverless(mut self) -> Self {
        self.pool_config = PoolConfig::serverless();
        self
    }

    /// Use development pool configuration.
    pub fn development(mut self) -> Self {
        self.pool_config = PoolConfig::development();
        self
    }

    /// Build the PraxClient.
    pub async fn build(self) -> Result<Arc<PraxClient>> {
        let config = if let Some(config) = self.config {
            config
        } else if let Some(url) = self.url {
            DatabaseConfig::from_url(&url)
                .map_err(|e| PraxArmatureError::ConfigError(e.to_string()))?
        } else {
            DatabaseConfig::from_env().map_err(|e| PraxArmatureError::ConfigError(e.to_string()))?
        };

        info!(
            driver = %config.driver.name(),
            "PraxClientBuilder building client"
        );

        let client = PraxClient {
            config,
            pool_config: self.pool_config,
        };

        Ok(Arc::new(client))
    }
}

impl Default for PraxClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::{
        DatabaseMiddleware, DatabaseProvider, PraxArmatureError, PraxClient, PraxClientBuilder,
        RequestTransaction, Result,
    };
    pub use prax_query::prelude::*;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default() {
        let builder = PraxClientBuilder::new();
        assert!(builder.url.is_none());
        assert!(builder.config.is_none());
    }

    #[test]
    fn test_builder_with_url() {
        let builder = PraxClientBuilder::new().url("postgresql://localhost/test");
        assert_eq!(builder.url, Some("postgresql://localhost/test".to_string()));
    }

    #[test]
    fn test_builder_pool_configs() {
        let builder = PraxClientBuilder::new().read_heavy();
        assert_eq!(builder.pool_config.pool.max_connections, 30);

        let builder = PraxClientBuilder::new().write_heavy();
        assert_eq!(builder.pool_config.pool.max_connections, 15);

        let builder = PraxClientBuilder::new().serverless();
        assert_eq!(builder.pool_config.pool.max_connections, 10);

        let builder = PraxClientBuilder::new().development();
        assert_eq!(builder.pool_config.pool.max_connections, 5);
    }

    #[test]
    fn test_request_transaction() {
        let config = DatabaseConfig::from_url("sqlite::memory:").unwrap();
        let client = PraxClient::with_config(config);
        let tx = RequestTransaction::new(client);

        assert!(!tx.is_committed());
        tx.commit();
        assert!(tx.is_committed());
    }
}
