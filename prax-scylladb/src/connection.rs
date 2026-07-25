//! `ScyllaDB` connection management.

use parking_lot::RwLock;
use scylla::Session;
use std::sync::Arc;

use crate::config::{ConsistencyLevel, ScyllaConfig, SerialConsistencyLevel};
use crate::error::{ScyllaError, ScyllaResult};

/// A wrapper around a `ScyllaDB` session.
#[derive(Clone)]
pub struct ScyllaConnection {
    session: Arc<Session>,
    config: Arc<ScyllaConfig>,
    /// Keyspace selected via `use_keyspace` after connecting.
    ///
    /// Stored as a leaked `&'static str` so [`ScyllaConnection::current_keyspace`]
    /// can keep its `Option<&str>` signature without returning a reference tied to a
    /// lock guard. Keyspace switches are rare, so the tiny leak per switch is fine.
    current_keyspace: Arc<RwLock<Option<&'static str>>>,
}

impl ScyllaConnection {
    /// Create a new connection from a session and config.
    pub(crate) fn new(session: Session, config: ScyllaConfig) -> Self {
        Self {
            session: Arc::new(session),
            config: Arc::new(config),
            current_keyspace: Arc::new(RwLock::new(None)),
        }
    }

    /// Get a reference to the underlying session.
    #[must_use]
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Get a reference to the configuration.
    #[must_use]
    pub fn config(&self) -> &ScyllaConfig {
        &self.config
    }

    /// Check if the connection is healthy by executing a simple query.
    pub async fn is_healthy(&self) -> bool {
        self.session
            .query_unpaged("SELECT now() FROM system.local", &[])
            .await
            .is_ok()
    }

    /// Use a specific keyspace for this connection.
    ///
    /// The keyspace name is validated against CQL's unquoted-identifier
    /// rules (`[A-Za-z][A-Za-z0-9_]*`, max 48 bytes) before being sent —
    /// the driver interpolates it into a `USE` statement, so anything
    /// outside the unquoted set could alter the CQL.
    pub async fn use_keyspace(&self, keyspace: &str) -> ScyllaResult<()> {
        validate_keyspace_identifier(keyspace)?;
        self.session
            .use_keyspace(keyspace, true)
            .await
            .map_err(|e| ScyllaError::Keyspace(e.to_string()))?;
        // Track the switch so `current_keyspace` reflects the session, not just the
        // config default.
        *self.current_keyspace.write() = Some(keyspace.to_owned().leak());
        Ok(())
    }

    /// Get the current keyspace: the one selected via `use_keyspace`, or the
    /// configured default if no switch has happened.
    #[must_use]
    pub fn current_keyspace(&self) -> Option<&str> {
        if let Some(keyspace) = *self.current_keyspace.read() {
            return Some(keyspace);
        }
        self.config.default_keyspace()
    }

    /// Execute a raw CQL query.
    pub async fn execute_raw(&self, query: &str) -> ScyllaResult<scylla::QueryResult> {
        self.session
            .query_unpaged(query, &[])
            .await
            .map_err(Into::into)
    }

    /// Prepare a statement for execution.
    pub async fn prepare(
        &self,
        query: &str,
    ) -> ScyllaResult<scylla::prepared_statement::PreparedStatement> {
        self.session.prepare(query).await.map_err(Into::into)
    }
}

/// Validate a keyspace name against CQL's unquoted-identifier rules
/// (`[A-Za-z][A-Za-z0-9_]*`, max 48 bytes) — the same rules prax-cassandra
/// enforces. The driver interpolates this name into a `USE` statement,
/// so anything outside the unquoted set could alter the CQL — reject it.
fn validate_keyspace_identifier(keyspace: &str) -> ScyllaResult<()> {
    let mut bytes = keyspace.bytes();
    let valid = matches!(bytes.next(), Some(b) if b.is_ascii_alphabetic())
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && keyspace.len() <= 48;
    if !valid {
        return Err(ScyllaError::Keyspace(format!(
            "invalid keyspace name {keyspace:?}: must match [A-Za-z][A-Za-z0-9_]* and be at most 48 bytes"
        )));
    }
    Ok(())
}

impl std::fmt::Debug for ScyllaConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScyllaConnection")
            .field("keyspace", &self.config.default_keyspace())
            .field("nodes", &self.config.known_nodes())
            .finish()
    }
}

/// Connect to a `ScyllaDB` cluster.
pub async fn connect(config: ScyllaConfig) -> ScyllaResult<ScyllaConnection> {
    use scylla::SessionBuilder;

    // TLS: with the `ssl` cargo feature, build an openssl `SslContext` with
    // client-side peer verification (OpenSSL's default system root store).
    // Without the feature, refuse to silently downgrade to plaintext.
    #[cfg(feature = "ssl")]
    let ssl_context = if config.ssl_enabled() {
        let mut context_builder = openssl::ssl::SslContext::builder(openssl::ssl::SslMethod::tls())
            .map_err(|e| {
                ScyllaError::Configuration(format!("failed to initialize TLS context: {e}"))
            })?;
        context_builder.set_verify(openssl::ssl::SslVerifyMode::PEER);
        Some(context_builder.build())
    } else {
        None
    };
    #[cfg(not(feature = "ssl"))]
    if config.ssl_enabled() {
        return Err(ScyllaError::Configuration(
            "ssl_enabled=true requested, but prax-scylladb was built without the `ssl` \
             cargo feature; refusing to connect over plaintext. Rebuild with \
             `--features ssl` to enable TLS"
                .into(),
        ));
    }

    let mut builder = SessionBuilder::new()
        .known_nodes(config.known_nodes())
        .connection_timeout(config.connection_timeout());

    #[cfg(feature = "ssl")]
    {
        builder = builder.ssl_context(ssl_context);
    }

    // Set default keyspace
    if let Some(keyspace) = config.default_keyspace() {
        builder = builder.use_keyspace(keyspace, true);
    }

    // Set authentication
    if let (Some(username), Some(password)) = (config.username(), config.password()) {
        builder = builder.user(username, password);
    }

    // Build the default execution profile: request timeout, consistency, serial
    // consistency, and (optionally) datacenter-aware load balancing.
    let mut profile_builder = scylla::execution_profile::ExecutionProfile::builder()
        .request_timeout(Some(config.request_timeout()))
        .consistency(map_consistency(config.consistency()))
        .serial_consistency(config.serial_consistency().map(map_serial_consistency));

    // Set local datacenter if specified
    if let Some(dc) = config.local_datacenter() {
        profile_builder = profile_builder.load_balancing_policy(
            scylla::load_balancing::DefaultPolicy::builder()
                .prefer_datacenter(dc.to_string())
                .build(),
        );
    }

    builder = builder.default_execution_profile_handle(profile_builder.build().into_handle());

    // Set per-node pool size
    if let Some(size) = std::num::NonZeroUsize::new(config.pool_size()) {
        builder = builder.pool_size(scylla::transport::session::PoolSize::PerHost(size));
    }

    // scylla 0.14's `SessionBuilder` has no application-name option (newer driver
    // versions do), so this config field cannot be honored.
    if let Some(name) = config.application_name() {
        tracing::warn!(
            application_name = name,
            "application_name is not supported by scylla 0.14 SessionBuilder and will not be applied"
        );
    }

    // Set compression
    if let Some(compression) = config.compression() {
        let compression = match compression.to_lowercase().as_str() {
            "lz4" => Some(scylla::transport::Compression::Lz4),
            "snappy" => Some(scylla::transport::Compression::Snappy),
            _ => None, // No compression
        };
        builder = builder.compression(compression);
    }

    // Build and connect
    let session = builder.build().await?;

    Ok(ScyllaConnection::new(session, config))
}

/// Map the config consistency level to the scylla driver's.
fn map_consistency(level: ConsistencyLevel) -> scylla::frame::types::Consistency {
    match level {
        ConsistencyLevel::Any => scylla::frame::types::Consistency::Any,
        ConsistencyLevel::One => scylla::frame::types::Consistency::One,
        ConsistencyLevel::Two => scylla::frame::types::Consistency::Two,
        ConsistencyLevel::Three => scylla::frame::types::Consistency::Three,
        ConsistencyLevel::Quorum => scylla::frame::types::Consistency::Quorum,
        ConsistencyLevel::All => scylla::frame::types::Consistency::All,
        ConsistencyLevel::LocalQuorum => scylla::frame::types::Consistency::LocalQuorum,
        ConsistencyLevel::EachQuorum => scylla::frame::types::Consistency::EachQuorum,
        ConsistencyLevel::LocalOne => scylla::frame::types::Consistency::LocalOne,
    }
}

/// Map the config serial consistency level to the scylla driver's.
fn map_serial_consistency(
    level: SerialConsistencyLevel,
) -> scylla::frame::types::SerialConsistency {
    match level {
        SerialConsistencyLevel::Serial => scylla::frame::types::SerialConsistency::Serial,
        SerialConsistencyLevel::LocalSerial => scylla::frame::types::SerialConsistency::LocalSerial,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_debug() {
        // Can't fully test without a real connection, but we can test the builder
        let config = ScyllaConfig::builder()
            .known_nodes(["localhost:9042"])
            .default_keyspace("test")
            .build();

        assert_eq!(config.default_keyspace(), Some("test"));
    }

    #[cfg(not(feature = "ssl"))]
    #[tokio::test]
    async fn test_connect_ssl_enabled_errors_instead_of_plaintext() {
        // Without the `ssl` feature, connect() must fail fast (before any
        // network I/O) rather than silently use plaintext.
        let config = ScyllaConfig::builder()
            .known_nodes(["localhost:9042"])
            .ssl_enabled(true)
            .build();

        let result = connect(config).await;
        assert!(
            matches!(result, Err(ScyllaError::Configuration(_))),
            "expected Configuration error, got: {result:?}"
        );
    }

    #[cfg(feature = "ssl")]
    #[tokio::test]
    async fn test_connect_ssl_enabled_builds_tls_context() {
        // With the `ssl` feature, ssl_enabled=true must NOT fail at config
        // time: the TLS context is built and handed to the session builder.
        // The connect itself fails at network I/O (no server here), which
        // proves we got past TLS configuration.
        let config = ScyllaConfig::builder()
            .known_nodes(["127.0.0.1:1"])
            .ssl_enabled(true)
            .build();

        let result = connect(config).await;
        assert!(
            !matches!(result, Err(ScyllaError::Configuration(_))),
            "TLS context should build; expected a network-level failure instead, got: {result:?}"
        );
    }

    #[test]
    fn test_validate_keyspace_identifier() {
        // Mirrors the prax-cassandra keyspace validation tests.
        assert!(validate_keyspace_identifier("myapp").is_ok());
        assert!(validate_keyspace_identifier("App_2024").is_ok());
        assert!(validate_keyspace_identifier("").is_err());
        assert!(validate_keyspace_identifier("1app").is_err());
        assert!(validate_keyspace_identifier("my app").is_err());
        assert!(validate_keyspace_identifier(&"a".repeat(49)).is_err());
        let err = validate_keyspace_identifier("evil\"; DROP KEYSPACE system; --").unwrap_err();
        assert!(
            err.to_string().contains("invalid keyspace name"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn test_map_consistency() {
        use scylla::frame::types::Consistency as S;

        assert_eq!(map_consistency(ConsistencyLevel::Any), S::Any);
        assert_eq!(map_consistency(ConsistencyLevel::One), S::One);
        assert_eq!(map_consistency(ConsistencyLevel::Two), S::Two);
        assert_eq!(map_consistency(ConsistencyLevel::Three), S::Three);
        assert_eq!(map_consistency(ConsistencyLevel::Quorum), S::Quorum);
        assert_eq!(map_consistency(ConsistencyLevel::All), S::All);
        assert_eq!(
            map_consistency(ConsistencyLevel::LocalQuorum),
            S::LocalQuorum
        );
        assert_eq!(map_consistency(ConsistencyLevel::EachQuorum), S::EachQuorum);
        assert_eq!(map_consistency(ConsistencyLevel::LocalOne), S::LocalOne);
    }

    #[test]
    fn test_map_serial_consistency() {
        use scylla::frame::types::SerialConsistency as S;

        assert_eq!(
            map_serial_consistency(SerialConsistencyLevel::Serial),
            S::Serial
        );
        assert_eq!(
            map_serial_consistency(SerialConsistencyLevel::LocalSerial),
            S::LocalSerial
        );
    }
}
