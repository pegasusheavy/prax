//! SASL authentication framework for Cassandra.
//!
//! Cassandra supports pluggable authentication mechanisms via SASL.
//! This module provides the [`SaslMechanism`] trait and a `PLAIN` implementation
//! covering username+password authentication.
//!
//! Future crates can implement additional mechanisms (LDAP, GSSAPI/Kerberos)
//! by implementing [`SaslMechanism`].

use async_trait::async_trait;

use crate::error::CassandraResult;

/// A SASL mechanism for authenticating against a Cassandra cluster.
///
/// Implementations are generally stateful — the `evaluate` method is called
/// repeatedly with server challenges until authentication completes.
#[async_trait]
pub trait SaslMechanism: Send + Sync + std::fmt::Debug {
    /// The SASL mechanism name (e.g., "PLAIN", "GSSAPI").
    fn name(&self) -> &str;

    /// Generate the initial client response sent with the SASL AUTHENTICATE.
    async fn initial_response(&self) -> CassandraResult<Vec<u8>>;

    /// Respond to a SASL challenge sent by the server.
    ///
    /// Returns the next client response. For single-round mechanisms like
    /// PLAIN, this returns an empty vector.
    async fn evaluate(&self, challenge: &[u8]) -> CassandraResult<Vec<u8>>;
}

/// PLAIN SASL mechanism: username + password over a single round.
#[derive(Debug, Clone)]
pub struct PlainSasl {
    /// Username for authentication.
    pub username: String,
    /// Password for authentication.
    pub password: String,
}

impl PlainSasl {
    /// Create a new PlainSasl authenticator.
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }
}

#[async_trait]
impl SaslMechanism for PlainSasl {
    fn name(&self) -> &str {
        "PLAIN"
    }

    async fn initial_response(&self) -> CassandraResult<Vec<u8>> {
        // PLAIN format: \0username\0password
        let mut buf = Vec::with_capacity(2 + self.username.len() + self.password.len());
        buf.push(0);
        buf.extend_from_slice(self.username.as_bytes());
        buf.push(0);
        buf.extend_from_slice(self.password.as_bytes());
        Ok(buf)
    }

    async fn evaluate(&self, _challenge: &[u8]) -> CassandraResult<Vec<u8>> {
        // PLAIN completes in the initial response; no further challenges.
        Ok(Vec::new())
    }
}

// ── cdrs-tokio bridge ──────────────────────────────────────────────────────

use std::sync::Arc;

use cdrs_tokio::authenticators::{SaslAuthenticator, SaslAuthenticatorProvider};
use cdrs_tokio::types::CBytes;
use tokio::runtime::{Handle, RuntimeFlavor};

/// Bridges prax's async [`SaslMechanism`] to cdrs-tokio's synchronous
/// `SaslAuthenticatorProvider`.
///
/// The initial response is computed **eagerly** in async context at
/// [`prepare`](Self::prepare) time (connect-time, where awaiting is legal),
/// then replayed synchronously by the driver. Challenge rounds (not used by
/// PLAIN) are driven synchronously via `block_in_place` on a multi-threaded
/// runtime, or `futures::executor::block_on` elsewhere — the latter stalls
/// if the mechanism's `evaluate` performs I/O on a current-thread runtime,
/// which is documented rather than silently papered over.
#[derive(Debug)]
pub struct PreparedSaslAuthenticatorProvider {
    mechanism: Arc<dyn SaslMechanism>,
    initial: Vec<u8>,
    handle: Handle,
}

impl PreparedSaslAuthenticatorProvider {
    /// Compute the mechanism's initial response in async context and
    /// capture the current runtime handle for later challenge rounds.
    pub async fn prepare(mechanism: Arc<dyn SaslMechanism>) -> CassandraResult<Self> {
        let initial = mechanism.initial_response().await?;
        Ok(Self {
            mechanism,
            initial,
            handle: Handle::current(),
        })
    }
}

impl SaslAuthenticatorProvider for PreparedSaslAuthenticatorProvider {
    fn name(&self) -> Option<&str> {
        // Only PLAIN maps to a known server-side authenticator class. For
        // custom mechanisms we return None and the driver omits the option,
        // letting the server use its configured default — a deployment-
        // specific class name can't be invented here.
        match self.mechanism.name() {
            "PLAIN" => Some("org.apache.cassandra.auth.PasswordAuthenticator"),
            _ => None,
        }
    }

    fn create_authenticator(&self) -> Box<dyn SaslAuthenticator + Send> {
        Box::new(PreparedSaslAuthenticator {
            mechanism: Arc::clone(&self.mechanism),
            initial: self.initial.clone(),
            handle: self.handle.clone(),
        })
    }
}

#[derive(Debug)]
struct PreparedSaslAuthenticator {
    mechanism: Arc<dyn SaslMechanism>,
    initial: Vec<u8>,
    handle: Handle,
}

impl PreparedSaslAuthenticator {
    /// Drive an async mechanism call synchronously from inside the driver's
    /// blocking auth handshake.
    fn drive(
        &self,
        fut: impl std::future::Future<Output = CassandraResult<Vec<u8>>> + Send + 'static,
    ) -> Result<Vec<u8>, String> {
        if self.handle.runtime_flavor() == RuntimeFlavor::MultiThread {
            let handle = self.handle.clone();
            tokio::task::block_in_place(|| handle.block_on(fut)).map_err(|e| e.to_string())
        } else {
            futures::executor::block_on(fut).map_err(|e| e.to_string())
        }
    }
}

impl SaslAuthenticator for PreparedSaslAuthenticator {
    fn initial_response(&self) -> CBytes {
        CBytes::new(self.initial.clone())
    }

    fn evaluate_challenge(&self, challenge: CBytes) -> cdrs_tokio::error::Result<CBytes> {
        let bytes = challenge.as_slice().unwrap_or(&[]).to_vec();
        let mechanism = Arc::clone(&self.mechanism);
        let response = self
            .drive(async move { mechanism.evaluate(&bytes).await })
            .map_err(cdrs_tokio::error::Error::from)?;
        Ok(CBytes::new(response))
    }

    fn handle_success(&self, _data: CBytes) -> cdrs_tokio::error::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plain_sasl_initial_response_format() {
        let sasl = PlainSasl::new("alice", "s3cret");
        let response = sasl.initial_response().await.unwrap();
        let expected: Vec<u8> = b"\0alice\0s3cret".to_vec();
        assert_eq!(response, expected);
    }

    #[tokio::test]
    async fn test_plain_sasl_evaluate_returns_empty() {
        let sasl = PlainSasl::new("alice", "s3cret");
        let response = sasl.evaluate(b"challenge").await.unwrap();
        assert!(response.is_empty());
    }

    #[test]
    fn test_plain_sasl_name() {
        let sasl = PlainSasl::new("u", "p");
        assert_eq!(sasl.name(), "PLAIN");
    }
}
