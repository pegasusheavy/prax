//! TLS connector construction for the connection pool, via rustls.
//!
//! Enabled by the `tls` cargo feature (on by default). Certificates are
//! verified against the Mozilla root store ([`webpki_roots`]) — chain and
//! hostname — for every TLS-capable [`crate::config::SslMode`]. This is
//! deliberately stricter than libpq, whose `sslmode=require` performs no
//! certificate verification; there is no encrypt-without-verify mode.
//!
//! A server whose CA is deliberately not publicly trusted therefore cannot be
//! verified out of the box. Amazon RDS is the common case: its `rds-ca-*`
//! authorities are Amazon-operated and absent from the Mozilla store, so with
//! `rds.force_ssl` on there is no working combination — the TLS-requiring
//! modes cannot verify the chain and the plaintext modes are refused by the
//! server. Point [`crate::config::PgConfig::ssl_root_cert`] (the libpq
//! `sslrootcert` URL parameter) at the provider's CA bundle for those.

use std::path::Path;
use std::sync::Arc;

use postgres_rustls::MakeTlsConnector;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::pem::PemObject;
use rustls::{ClientConfig, RootCertStore};

use crate::error::{PgError, PgResult};

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
    connector_with_roots(webpki_root_store())
}

/// Build the TLS connector, optionally verifying against a PEM bundle instead
/// of the Mozilla root store.
///
/// `None` is identical to [`make_tls_connector`]. `Some(path)` *replaces* the
/// webpki roots with the certificates in that file rather than adding to them,
/// matching libpq's `sslrootcert`: a pool addresses one server, so "trust
/// exactly this bundle" is the stricter and more predictable reading. It also
/// means a typo in the path cannot silently fall back to public trust.
///
/// Errors if the file cannot be read, contains no certificates, or contains
/// one rustls rejects — all of which would otherwise surface much later as an
/// opaque handshake failure.
pub fn make_tls_connector_with_root_cert(root_cert: Option<&Path>) -> PgResult<MakeTlsConnector> {
    let roots = match root_cert {
        None => webpki_root_store(),
        Some(path) => {
            let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_file_iter(path)
                .map_err(|e| {
                    PgError::config(format!(
                        "sslrootcert: cannot read certificates from {}: {e}",
                        path.display()
                    ))
                })?
                .collect::<Result<_, _>>()
                .map_err(|e| {
                    PgError::config(format!(
                        "sslrootcert: invalid certificate in {}: {e}",
                        path.display()
                    ))
                })?;

            if certs.is_empty() {
                return Err(PgError::config(format!(
                    "sslrootcert: no certificates found in {}",
                    path.display()
                )));
            }

            let mut roots = RootCertStore::empty();
            for cert in certs {
                roots.add(cert).map_err(|e| {
                    PgError::config(format!(
                        "sslrootcert: rustls rejected a certificate in {}: {e}",
                        path.display()
                    ))
                })?;
            }
            roots
        }
    };

    Ok(connector_with_roots(roots))
}

fn webpki_root_store() -> RootCertStore {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    roots
}

fn connector_with_roots(roots: RootCertStore) -> MakeTlsConnector {
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
    use std::io::Write;
    use tokio_postgres::tls::MakeTlsConnect;

    /// A self-signed certificate, PEM-encoded. Only ever parsed, never used to
    /// establish a connection, so it needs to be well-formed rather than valid
    /// for any particular name.
    const TEST_CA_PEM: &str = include_str!("../tests/data/test-ca.pem");

    fn write_temp(contents: &str, name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("prax-{}-{}.pem", std::process::id(), name));
        let mut f = std::fs::File::create(&path).expect("create temp pem");
        f.write_all(contents.as_bytes()).expect("write temp pem");
        path
    }

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

    #[test]
    fn none_root_cert_matches_the_default_connector() {
        // The no-argument form must stay exactly the webpki path.
        assert!(make_tls_connector_with_root_cert(None).is_ok());
    }

    #[test]
    fn loads_a_pem_bundle() {
        let path = write_temp(TEST_CA_PEM, "valid");
        let loaded = make_tls_connector_with_root_cert(Some(&path)).is_ok();
        let _ = std::fs::remove_file(&path);
        assert!(loaded, "a well-formed PEM bundle must load");
    }

    #[test]
    fn missing_file_names_the_path() {
        // Failing at pool construction with the path in the message is the
        // whole point: the alternative is an opaque handshake error later.
        // `MakeTlsConnector` is not Debug, so unwrap the error by hand rather
        // than through expect_err.
        let msg = match make_tls_connector_with_root_cert(Some(std::path::Path::new(
            "/nonexistent/prax-no-such-ca.pem",
        ))) {
            Ok(_) => panic!("a missing bundle must fail"),
            Err(e) => e.to_string(),
        };
        assert!(msg.contains("sslrootcert"), "message was: {msg}");
        assert!(msg.contains("prax-no-such-ca.pem"), "message was: {msg}");
    }

    #[test]
    fn empty_bundle_is_rejected() {
        // An empty file parses to zero certificates; accepting it would build
        // a connector that trusts nothing and fails every handshake.
        let path = write_temp("# no certificates here\n", "empty");
        let result = make_tls_connector_with_root_cert(Some(&path));
        let _ = std::fs::remove_file(&path);
        let msg = match result {
            Ok(_) => panic!("an empty bundle must fail"),
            Err(e) => e.to_string(),
        };
        assert!(msg.contains("no certificates found"), "message was: {msg}");
    }
}
