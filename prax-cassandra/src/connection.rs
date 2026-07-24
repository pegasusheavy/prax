//! Connection wrapper around a cdrs-tokio Session.

use std::sync::Arc;

use cdrs_tokio::authenticators::{NoneAuthenticatorProvider, StaticPasswordAuthenticatorProvider};
use cdrs_tokio::cluster::connection_pool::ConnectionPoolConfigBuilder;
use cdrs_tokio::cluster::session::{Session, SessionBuilder, TcpSessionBuilder};
use cdrs_tokio::cluster::{NodeAddress, NodeTcpConfigBuilder, TcpConnectionManager};
use cdrs_tokio::load_balancing::RoundRobinLoadBalancingStrategy;
use cdrs_tokio::retry::{DefaultRetryPolicy, FallthroughRetryPolicy, RetryPolicy};
use cdrs_tokio::transport::TransportTcp;

use crate::auth::PreparedSaslAuthenticatorProvider;
use crate::config::{CassandraAuth, CassandraConfig, RetryPolicyKind, TlsConfig};
use crate::error::{CassandraError, CassandraResult};

/// Concrete cdrs-tokio session types used by prax-cassandra. We pin the
/// load-balancing strategy to round-robin so the outer type is
/// nameable (otherwise we'd need to box it behind `dyn Any`). TLS and
/// plaintext sessions are different generic instantiations of the same
/// `Session`, unified here behind a forwarding enum.
pub(crate) enum CdrsSession {
    Tcp(
        Session<
            TransportTcp,
            TcpConnectionManager,
            RoundRobinLoadBalancingStrategy<TransportTcp, TcpConnectionManager>,
        >,
    ),
    Rustls(
        Session<
            cdrs_tokio::transport::TransportRustls,
            cdrs_tokio::cluster::RustlsConnectionManager,
            RoundRobinLoadBalancingStrategy<
                cdrs_tokio::transport::TransportRustls,
                cdrs_tokio::cluster::RustlsConnectionManager,
            >,
        >,
    ),
}

impl CdrsSession {
    /// Forward to the underlying session's `query`.
    pub(crate) async fn query<Q: ToString>(
        &self,
        query: Q,
    ) -> cdrs_tokio::error::Result<cdrs_tokio::frame::Envelope> {
        match self {
            Self::Tcp(session) => session.query(query).await,
            Self::Rustls(session) => session.query(query).await,
        }
    }

    /// Forward to the underlying session's `query_with_values`.
    pub(crate) async fn query_with_values<Q: ToString, V: Into<cdrs_tokio::query::QueryValues>>(
        &self,
        query: Q,
        values: V,
    ) -> cdrs_tokio::error::Result<cdrs_tokio::frame::Envelope> {
        match self {
            Self::Tcp(session) => session.query_with_values(query, values).await,
            Self::Rustls(session) => session.query_with_values(query, values).await,
        }
    }

    /// Forward to the underlying session's `batch`.
    pub(crate) async fn batch(
        &self,
        batch: cdrs_tokio::query::QueryBatch,
    ) -> cdrs_tokio::error::Result<cdrs_tokio::frame::Envelope> {
        match self {
            Self::Tcp(session) => session.batch(batch).await,
            Self::Rustls(session) => session.batch(batch).await,
        }
    }
}

/// A handle to an established Cassandra session.
pub struct CassandraConnection {
    config: CassandraConfig,
    pub(crate) session: Arc<CdrsSession>,
}

impl CassandraConnection {
    /// Connect to the cluster using the provided configuration.
    pub async fn connect(config: CassandraConfig) -> CassandraResult<Self> {
        if config.known_nodes.is_empty() {
            return Err(CassandraError::Connection(
                "at least one contact point is required".into(),
            ));
        }

        if config.pool_size == 0 {
            return Err(CassandraError::Connection(
                "pool_size must be at least 1".into(),
            ));
        }

        if let Some(keyspace) = &config.default_keyspace {
            validate_keyspace_identifier(keyspace)?;
        }

        // cdrs-tokio 9.x has no session-level default consistency and no
        // request deadline: consistency is per-query (`StatementParams`)
        // and queries carry no timeout. Warn once per unhonored option
        // instead of dropping them silently.
        tracing::warn!(
            consistency = ?config.consistency,
            "default consistency is not applied at connection level: \
             cdrs-tokio only supports per-query consistency"
        );
        tracing::warn!(
            request_timeout = ?config.request_timeout,
            "request_timeout is not applied: cdrs-tokio does not expose \
             a per-query or session request timeout"
        );

        // Resolve the authenticator once — both transports consume it.
        let authenticator_provider: Arc<
            dyn cdrs_tokio::authenticators::SaslAuthenticatorProvider + Send + Sync,
        > = match &config.auth {
            Some(CassandraAuth::Password { username, password }) => {
                Arc::new(StaticPasswordAuthenticatorProvider::new(username, password))
            }
            Some(CassandraAuth::Sasl(mechanism)) => Arc::new(
                PreparedSaslAuthenticatorProvider::prepare(Arc::clone(mechanism))
                    .await
                    .map_err(|e| {
                        CassandraError::Connection(format!("SASL initial response failed: {e}"))
                    })?,
            ),
            None => Arc::new(NoneAuthenticatorProvider),
        };

        let retry_policy: Box<dyn RetryPolicy + Send + Sync> = match config.retry_policy {
            RetryPolicyKind::Default => Box::<DefaultRetryPolicy>::default(),
            RetryPolicyKind::Never => Box::<FallthroughRetryPolicy>::default(),
            // cdrs-tokio retry decisions cannot downgrade consistency, so
            // there is no faithful equivalent; fall back to the default.
            RetryPolicyKind::Downgrading => {
                tracing::warn!(
                    "retry_policy=Downgrading is not supported by cdrs-tokio; \
                     using the default retry policy"
                );
                Box::<DefaultRetryPolicy>::default()
            }
        };

        let pool_config = ConnectionPoolConfigBuilder::new()
            .with_local_size(config.pool_size)
            .with_remote_size(config.pool_size)
            .with_connect_timeout(Some(config.connection_timeout))
            .build();

        let session = match &config.tls {
            Some(tls) => {
                Self::connect_rustls(
                    &config,
                    tls,
                    authenticator_provider,
                    retry_policy,
                    pool_config,
                )
                .await?
            }
            None => {
                Self::connect_tcp(&config, authenticator_provider, retry_policy, pool_config)
                    .await?
            }
        };

        Ok(Self {
            config,
            session: Arc::new(session),
        })
    }

    /// Plaintext TCP session path.
    async fn connect_tcp(
        config: &CassandraConfig,
        authenticator_provider: Arc<
            dyn cdrs_tokio::authenticators::SaslAuthenticatorProvider + Send + Sync,
        >,
        retry_policy: Box<dyn RetryPolicy + Send + Sync>,
        pool_config: cdrs_tokio::cluster::connection_pool::ConnectionPoolConfig,
    ) -> CassandraResult<CdrsSession> {
        let mut builder = NodeTcpConfigBuilder::new();
        for node in &config.known_nodes {
            let addr: NodeAddress = node.as_str().into();
            builder = builder.with_contact_point(addr);
        }
        let node_config = builder
            .with_authenticator_provider(authenticator_provider)
            .build()
            .await
            .map_err(|e| CassandraError::Connection(format!("resolve contact points: {e}")))?;

        let lb = RoundRobinLoadBalancingStrategy::<TransportTcp, TcpConnectionManager>::new();
        let mut session_builder = TcpSessionBuilder::new(lb, node_config)
            .with_retry_policy(retry_policy)
            .with_connection_pool_config(pool_config);
        if let Some(keyspace) = &config.default_keyspace {
            // Prefer the builder knob over a post-connect `USE`: cdrs-tokio
            // warns that `USE` does not propagate atomically to pooled
            // connections, while `with_keyspace` is set before any
            // connection is established.
            session_builder = session_builder.with_keyspace(keyspace.clone());
        }
        let session = session_builder
            .build()
            .await
            .map_err(|e| CassandraError::Connection(format!("build session: {e}")))?;
        Ok(CdrsSession::Tcp(session))
    }

    /// TLS session path (rustls).
    async fn connect_rustls(
        config: &CassandraConfig,
        tls: &TlsConfig,
        authenticator_provider: Arc<
            dyn cdrs_tokio::authenticators::SaslAuthenticatorProvider + Send + Sync,
        >,
        retry_policy: Box<dyn RetryPolicy + Send + Sync>,
        pool_config: cdrs_tokio::cluster::connection_pool::ConnectionPoolConfig,
    ) -> CassandraResult<CdrsSession> {
        use cdrs_tokio::cluster::session::RustlsSessionBuilder;
        use cdrs_tokio::cluster::{NodeRustlsConfigBuilder, RustlsConnectionManager};
        use cdrs_tokio::transport::TransportRustls;

        let client_config = build_tls_client_config(tls)?;

        // cdrs's rustls config verifies every node against a single DNS
        // name; use the first contact point's host part (documented).
        let host = config.known_nodes[0]
            .rsplit_once(':')
            .map(|(host, _port)| host)
            .unwrap_or(&config.known_nodes[0]);
        let dns_name = rustls::pki_types::ServerName::try_from(host.to_string()).map_err(|e| {
            CassandraError::Connection(format!(
                "first contact point host {host:?} is not a valid TLS server name: {e}"
            ))
        })?;

        let mut builder = NodeRustlsConfigBuilder::new(dns_name, Arc::new(client_config));
        for node in &config.known_nodes {
            let addr: NodeAddress = node.as_str().into();
            builder = builder.with_contact_point(addr);
        }
        let node_config = builder
            .with_authenticator_provider(authenticator_provider)
            .build()
            .await
            .map_err(|e| CassandraError::Connection(format!("resolve contact points: {e}")))?;

        let lb = RoundRobinLoadBalancingStrategy::<TransportRustls, RustlsConnectionManager>::new();
        let mut session_builder = RustlsSessionBuilder::new(lb, node_config)
            .with_retry_policy(retry_policy)
            .with_connection_pool_config(pool_config);
        if let Some(keyspace) = &config.default_keyspace {
            session_builder = session_builder.with_keyspace(keyspace.clone());
        }
        let session = session_builder
            .build()
            .await
            .map_err(|e| CassandraError::Connection(format!("build TLS session: {e}")))?;
        Ok(CdrsSession::Rustls(session))
    }

    /// Borrow the configuration this connection was built from.
    pub fn config(&self) -> &CassandraConfig {
        &self.config
    }

    /// Borrow the underlying cdrs-tokio session.
    pub(crate) fn session(&self) -> &CdrsSession {
        &self.session
    }

    /// Ping the cluster with `SELECT now() FROM system.local`.
    pub async fn ping(&self) -> CassandraResult<()> {
        self.session()
            .query("SELECT now() FROM system.local")
            .await
            .map_err(|e| CassandraError::Connection(format!("ping failed: {e}")))?;
        Ok(())
    }
}

/// Build the rustls `ClientConfig` for a Cassandra TLS session.
///
/// Root store: `ca_cert` PEM if given, else the Mozilla/webpki roots.
/// `client_cert` + `client_key` enable mutual TLS. `verify_hostname=false`
/// builds an encrypt-only context (no certificate verification) with a loud
/// warning — chain-without-hostname verification is not expressible with
/// stock rustls verifiers.
fn build_tls_client_config(tls: &TlsConfig) -> CassandraResult<rustls::ClientConfig> {
    use rustls::pki_types::CertificateDer;
    use rustls::{ClientConfig, RootCertStore};

    let mut roots = RootCertStore::empty();
    match &tls.ca_cert {
        Some(path) => {
            let pem = std::fs::read(path).map_err(|e| {
                CassandraError::Connection(format!("read ca_cert {}: {e}", path.display()))
            })?;
            let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut pem.as_slice())
                .collect::<Result<_, _>>()
                .map_err(|e| {
                    CassandraError::Connection(format!(
                        "parse ca_cert {} as PEM: {e}",
                        path.display()
                    ))
                })?;
            if certs.is_empty() {
                return Err(CassandraError::Connection(format!(
                    "ca_cert {} contains no PEM certificates",
                    path.display()
                )));
            }
            let (added, _) = roots.add_parsable_certificates(certs);
            if added == 0 {
                return Err(CassandraError::Connection(format!(
                    "ca_cert {}: no certificates could be parsed",
                    path.display()
                )));
            }
        }
        None => {
            roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        }
    }

    // Select aws-lc-rs explicitly: workspace feature unification can enable
    // both rustls providers, in which case rustls cannot auto-determine a
    // process-level default.
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let builder = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| CassandraError::Connection(format!("TLS protocol versions: {e}")))?;

    let builder = if !tls.verify_hostname {
        tracing::warn!(
            "verify_hostname=false: building an encrypt-only TLS context with NO \
             certificate verification (vulnerable to MITM; use only for development)"
        );
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertVerifier))
    } else {
        builder.with_root_certificates(roots)
    };

    match (&tls.client_cert, &tls.client_key) {
        (Some(cert_path), Some(key_path)) => {
            let certs = read_pem_certs(cert_path)?;
            let key = read_pem_key(key_path)?;
            builder
                .with_client_auth_cert(certs, key)
                .map_err(|e| CassandraError::Connection(format!("client TLS identity: {e}")))
        }
        (None, None) => Ok(builder.with_no_client_auth()),
        _ => Err(CassandraError::Connection(
            "client_cert and client_key must be set together for mutual TLS".into(),
        )),
    }
}

/// Read a PEM certificate chain from disk.
fn read_pem_certs(
    path: &std::path::Path,
) -> CassandraResult<Vec<rustls::pki_types::CertificateDer<'static>>> {
    let pem = std::fs::read(path).map_err(|e| {
        CassandraError::Connection(format!("read certificate {}: {e}", path.display()))
    })?;
    let certs: Vec<_> = rustls_pemfile::certs(&mut pem.as_slice())
        .collect::<Result<_, _>>()
        .map_err(|e| {
            CassandraError::Connection(format!("parse certificate {} as PEM: {e}", path.display()))
        })?;
    if certs.is_empty() {
        return Err(CassandraError::Connection(format!(
            "certificate {} contains no PEM certificates",
            path.display()
        )));
    }
    Ok(certs)
}

/// Read a PEM private key (PKCS#8, PKCS#1, or SEC1) from disk.
fn read_pem_key(
    path: &std::path::Path,
) -> CassandraResult<rustls::pki_types::PrivateKeyDer<'static>> {
    let pem = std::fs::read(path)
        .map_err(|e| CassandraError::Connection(format!("read key {}: {e}", path.display())))?;
    rustls_pemfile::private_key(&mut pem.as_slice())
        .map_err(|e| {
            CassandraError::Connection(format!("parse key {} as PEM: {e}", path.display()))
        })?
        .ok_or_else(|| {
            CassandraError::Connection(format!("key {} contains no private key", path.display()))
        })
}

/// Certificate verifier that accepts everything — used only when
/// `verify_hostname` is explicitly set to `false` (encrypt-only mode).
#[derive(Debug)]
struct NoCertVerifier;

impl rustls::client::danger::ServerCertVerifier for NoCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

/// Validate a keyspace name against Cassandra's unquoted-identifier
/// rules (`[A-Za-z][A-Za-z0-9_]*`, max 48 bytes). The driver forwards
/// this name into protocol messages and per-connection `USE` statements,
/// so anything outside the unquoted set could alter the CQL — reject it.
fn validate_keyspace_identifier(keyspace: &str) -> CassandraResult<()> {
    let mut bytes = keyspace.bytes();
    let valid = matches!(bytes.next(), Some(b) if b.is_ascii_alphabetic())
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && keyspace.len() <= 48;
    if !valid {
        return Err(CassandraError::Connection(format!(
            "invalid keyspace name {keyspace:?}: must match [A-Za-z][A-Za-z0-9_]* and be at most 48 bytes"
        )));
    }
    Ok(())
}

impl std::fmt::Debug for CassandraConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CassandraConnection")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::PlainSasl;
    use crate::config::TlsConfig;

    fn block_on_connect(config: CassandraConfig) -> CassandraResult<CassandraConnection> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            // cdrs retries contact points with backoff; a refused/!routable
            // test address would otherwise hang the test. A timeout proves
            // the same thing as a fast network error: config succeeded.
            match tokio::time::timeout(
                std::time::Duration::from_secs(10),
                CassandraConnection::connect(config),
            )
            .await
            {
                Ok(result) => result,
                Err(_) => Err(CassandraError::Connection(
                    "test timeout (proves config succeeded and network was attempted)".into(),
                )),
            }
        })
    }

    #[test]
    fn empty_known_nodes_is_an_error() {
        // Building a connection with no contact points should fail
        // fast rather than wait for cdrs-tokio to complain. Keep
        // this as a fast unit test; the live-cluster connect path
        // is exercised by the e2e integration tests.
        let config = CassandraConfig::builder().build();
        let result = block_on_connect(config);
        assert!(result.is_err(), "expected connect to fail with no nodes");
    }

    #[test]
    fn tls_config_builds_client_config_and_fails_at_network() {
        // With the `rust-tls` feature enabled, TLS configuration must NOT
        // be rejected at config time: the rustls ClientConfig is built and
        // the connect proceeds to (here, failing) network I/O — proving
        // TLS was wired rather than downgraded to plaintext.
        let config = CassandraConfig::builder()
            .known_nodes(["127.0.0.1:1".to_string()])
            .tls(TlsConfig::default())
            .build();
        let err = block_on_connect(config).unwrap_err();
        assert!(
            !err.to_string().contains("TLS configured but not supported"),
            "TLS should be supported now, got: {err}"
        );
    }

    #[test]
    fn tls_verify_hostname_false_builds_encrypt_only_config() {
        // The dangerous path must still construct a client config (with a
        // loud warning) rather than erroring.
        let tls = TlsConfig {
            verify_hostname: false,
            ..Default::default()
        };
        let result = build_tls_client_config(&tls);
        assert!(
            result.is_ok(),
            "encrypt-only config should build: {result:?}"
        );
    }

    #[test]
    fn tls_client_cert_without_key_is_rejected() {
        let tls = TlsConfig {
            client_cert: Some(std::path::PathBuf::from("/tmp/cert.pem")),
            client_key: None,
            ..Default::default()
        };
        let err = build_tls_client_config(&tls).unwrap_err();
        assert!(
            err.to_string().contains("must be set together"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn sasl_auth_builds_authenticator_and_fails_at_network() {
        // SASL must no longer be rejected at config time: the authenticator
        // is prepared (eager initial response) and the connect proceeds to
        // network I/O, which fails here for lack of a server.
        let config = CassandraConfig::builder()
            .known_nodes(["127.0.0.1:1".to_string()])
            .auth(CassandraAuth::Sasl(Arc::new(PlainSasl::new("u", "p"))))
            .build();
        let err = block_on_connect(config).unwrap_err();
        assert!(
            !err.to_string()
                .contains("SASL authentication is not yet supported"),
            "SASL should be supported now, got: {err}"
        );
    }

    #[tokio::test]
    async fn prepared_sasl_provider_replays_plain_initial_response() {
        use cdrs_tokio::authenticators::SaslAuthenticatorProvider;

        let provider =
            PreparedSaslAuthenticatorProvider::prepare(Arc::new(PlainSasl::new("alice", "s3cret")))
                .await
                .unwrap();
        assert_eq!(
            provider.name(),
            Some("org.apache.cassandra.auth.PasswordAuthenticator")
        );
        let auth = provider.create_authenticator();
        let initial = auth.initial_response();
        assert_eq!(initial.as_slice(), Some(b"\0alice\0s3cret".as_slice()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn prepared_sasl_authenticator_drives_evaluate_synchronously() {
        use cdrs_tokio::authenticators::SaslAuthenticatorProvider;

        let provider =
            PreparedSaslAuthenticatorProvider::prepare(Arc::new(PlainSasl::new("u", "p")))
                .await
                .unwrap();
        let auth = provider.create_authenticator();
        // PLAIN's evaluate returns an empty vector; the point is that the
        // sync bridge successfully drives the async call from a blocking
        // context without panicking or deadlocking.
        let response = auth
            .evaluate_challenge(cdrs_tokio::types::CBytes::new(b"challenge".to_vec()))
            .unwrap();
        assert_eq!(response.as_slice(), Some(&[][..]));
    }

    #[test]
    fn invalid_keyspace_name_is_rejected() {
        let config = CassandraConfig::builder()
            .known_nodes(["127.0.0.1:9042".to_string()])
            .default_keyspace("evil\"; DROP KEYSPACE system; --")
            .build();
        let err = block_on_connect(config).unwrap_err();
        assert!(
            err.to_string().contains("invalid keyspace name"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn zero_pool_size_is_rejected() {
        let config = CassandraConfig::builder()
            .known_nodes(["127.0.0.1:9042".to_string()])
            .pool_size(0)
            .build();
        let err = block_on_connect(config).unwrap_err();
        assert!(
            err.to_string().contains("pool_size"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn valid_keyspace_names_pass_validation() {
        assert!(validate_keyspace_identifier("myapp").is_ok());
        assert!(validate_keyspace_identifier("App_2024").is_ok());
        assert!(validate_keyspace_identifier("").is_err());
        assert!(validate_keyspace_identifier("1app").is_err());
        assert!(validate_keyspace_identifier("my app").is_err());
        assert!(validate_keyspace_identifier(&"a".repeat(49)).is_err());
    }
}
