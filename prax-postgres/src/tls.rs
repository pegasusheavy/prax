//! TLS connector construction for the connection pool, via rustls.
//!
//! Enabled by the `tls` cargo feature (on by default). Certificates are
//! verified against the Mozilla root store ([`webpki_roots`]) — chain and
//! hostname — for every TLS-capable [`crate::config::SslMode`]. This is
//! deliberately stricter than libpq, whose `sslmode=require` performs no
//! certificate verification; there is currently no encrypt-without-verify
//! mode.

use std::sync::Arc;

use postgres_rustls::MakeTlsConnector;
use rustls::{ClientConfig, RootCertStore};

/// Build the TLS connector handed to deadpool's `Manager`.
///
/// The same connector serves every TLS mode: `Prefer` (tokio-postgres falls
/// back to plaintext only when the *server* declines TLS), and
/// `Require`/`VerifyCa`/`VerifyFull` (driver-level `SslMode::Require`, so a
/// server that refuses TLS fails the connection). Verification is always
/// webpki chain + hostname — see the module docs.
///
/// This is the workspace's shared rustls connector, exposed so downstream
/// tooling (e.g. `prax-cli`'s introspector) can reuse the same certificate
/// verification behavior. Available only when the `tls` cargo feature is
/// enabled (on by default).
pub fn make_tls_connector() -> MakeTlsConnector {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    // Select aws-lc-rs explicitly: workspace feature unification can enable
    // both rustls providers (aws-lc-rs via sqlx, ring via other crates), in
    // which case rustls cannot auto-determine a process-level default.
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let client_config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("safe default protocol versions are available")
        .with_root_certificates(roots)
        .with_no_client_auth();

    MakeTlsConnector::new(tokio_rustls::TlsConnector::from(Arc::new(client_config)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_postgres::tls::MakeTlsConnect;

    #[test]
    fn connector_is_constructible_and_cloneable() {
        // deadpool requires MakeTlsConnect + Clone + Sync + Send + 'static;
        // construction also proves the rustls provider and root store load.
        let connector = make_tls_connector();
        let mut cloned = connector.clone();
        // A syntactically invalid domain must fail in make_tls_connect
        // (DNS name parsing) without any network I/O.
        assert!(
            <MakeTlsConnector as MakeTlsConnect<tokio::net::TcpStream>>::make_tls_connect(
                &mut cloned,
                "invalid..domain"
            )
            .is_err()
        );
    }
}
