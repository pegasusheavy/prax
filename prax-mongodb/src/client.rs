//! MongoDB client wrapper with built-in connection pooling.

use std::sync::Arc;

use bson::{Document, doc};
use mongodb::{Client, Collection, Database};
use tracing::{debug, info};

use crate::config::MongoConfig;
use crate::error::{MongoError, MongoResult};

/// A MongoDB client with connection pooling.
///
/// The MongoDB driver handles connection pooling internally,
/// so this client wraps the driver's Client with additional
/// Prax-specific functionality.
#[derive(Clone)]
pub struct MongoClient {
    client: Client,
    database: Database,
    config: Arc<MongoConfig>,
}

impl MongoClient {
    /// Create a new client from configuration.
    pub async fn new(config: MongoConfig) -> MongoResult<Self> {
        let options = config.to_client_options().await?;

        let client = Client::with_options(options)
            .map_err(|e| MongoError::connection(format!("failed to create client: {}", e)))?;

        let database = client.database(&config.database);

        info!(
            uri = %config.uri,
            database = %config.database,
            "MongoDB client created"
        );

        Ok(Self {
            client,
            database,
            config: Arc::new(config),
        })
    }

    /// Create a builder for the client.
    pub fn builder() -> MongoClientBuilder {
        MongoClientBuilder::new()
    }

    /// Get a typed collection.
    pub fn collection<T>(&self, name: &str) -> Collection<T>
    where
        T: Send + Sync,
    {
        self.database.collection(name)
    }

    /// Get a collection with BSON documents.
    pub fn collection_doc(&self, name: &str) -> Collection<Document> {
        self.database.collection(name)
    }

    /// Get the underlying database.
    pub fn database(&self) -> &Database {
        &self.database
    }

    /// Get a different database from the same client.
    pub fn get_database(&self, name: &str) -> Database {
        self.client.database(name)
    }

    /// Get the underlying MongoDB client.
    pub fn inner(&self) -> &Client {
        &self.client
    }

    /// Get the configuration.
    pub fn config(&self) -> &MongoConfig {
        &self.config
    }

    /// Check if the client is healthy by pinging the server.
    pub async fn is_healthy(&self) -> bool {
        self.database
            .run_command(doc! { "ping": 1 }, None)
            .await
            .is_ok()
    }

    /// List all collection names in the database.
    pub async fn list_collections(&self) -> MongoResult<Vec<String>> {
        let names = self
            .database
            .list_collection_names(None)
            .await
            .map_err(MongoError::from)?;
        Ok(names)
    }

    /// Drop a collection.
    pub async fn drop_collection(&self, name: &str) -> MongoResult<()> {
        debug!(collection = %name, "Dropping collection");
        self.database
            .collection::<Document>(name)
            .drop(None)
            .await
            .map_err(MongoError::from)?;
        Ok(())
    }

    /// Create an index on a collection.
    pub async fn create_index(
        &self,
        collection: &str,
        keys: Document,
        unique: bool,
    ) -> MongoResult<String> {
        use mongodb::IndexModel;
        use mongodb::options::IndexOptions;

        let options = IndexOptions::builder().unique(unique).build();
        let model = IndexModel::builder().keys(keys).options(options).build();

        let result = self
            .database
            .collection::<Document>(collection)
            .create_index(model, None)
            .await
            .map_err(MongoError::from)?;

        Ok(result.index_name)
    }

    /// Run a database command.
    pub async fn run_command(&self, command: Document) -> MongoResult<Document> {
        let result = self
            .database
            .run_command(command, None)
            .await
            .map_err(MongoError::from)?;
        Ok(result)
    }

    /// Start a client session for transactions.
    pub async fn start_session(&self) -> MongoResult<mongodb::ClientSession> {
        let session = self
            .client
            .start_session(None)
            .await
            .map_err(MongoError::from)?;
        Ok(session)
    }
}

/// Builder for MongoClient.
#[derive(Debug, Default)]
pub struct MongoClientBuilder {
    uri: Option<String>,
    database: Option<String>,
    app_name: Option<String>,
    max_pool_size: Option<u32>,
    min_pool_size: Option<u32>,
    connect_timeout: Option<std::time::Duration>,
    direct_connection: Option<bool>,
}

impl MongoClientBuilder {
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

    /// Set the maximum pool size.
    pub fn max_pool_size(mut self, size: u32) -> Self {
        self.max_pool_size = Some(size);
        self
    }

    /// Set the minimum pool size.
    pub fn min_pool_size(mut self, size: u32) -> Self {
        self.min_pool_size = Some(size);
        self
    }

    /// Set the connection timeout.
    pub fn connect_timeout(mut self, duration: std::time::Duration) -> Self {
        self.connect_timeout = Some(duration);
        self
    }

    /// Enable direct connection (bypass replica set discovery).
    pub fn direct_connection(mut self, enabled: bool) -> Self {
        self.direct_connection = Some(enabled);
        self
    }

    /// Build the client.
    pub async fn build(self) -> MongoResult<MongoClient> {
        let mut config_builder = MongoConfig::builder();

        if let Some(uri) = self.uri {
            config_builder = config_builder.uri(uri);
        }

        if let Some(database) = self.database {
            config_builder = config_builder.database(database);
        }

        if let Some(app_name) = self.app_name {
            config_builder = config_builder.app_name(app_name);
        }

        if let Some(max_pool) = self.max_pool_size {
            config_builder = config_builder.max_pool_size(max_pool);
        }

        if let Some(min_pool) = self.min_pool_size {
            config_builder = config_builder.min_pool_size(min_pool);
        }

        if let Some(timeout) = self.connect_timeout {
            config_builder = config_builder.connect_timeout(timeout);
        }

        if let Some(direct) = self.direct_connection {
            config_builder = config_builder.direct_connection(direct);
        }

        let config = config_builder.build()?;
        MongoClient::new(config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder() {
        let builder = MongoClientBuilder::new()
            .uri("mongodb://localhost:27017")
            .database("test")
            .max_pool_size(20);

        assert_eq!(builder.uri, Some("mongodb://localhost:27017".to_string()));
        assert_eq!(builder.database, Some("test".to_string()));
        assert_eq!(builder.max_pool_size, Some(20));
    }
}
