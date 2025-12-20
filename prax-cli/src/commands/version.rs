//! `prax version` command - Display version information.

use crate::error::CliResult;
use crate::output::{self, kv};

/// Package version
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Package name
const NAME: &str = env!("CARGO_PKG_NAME");

/// Run the version command
pub async fn run() -> CliResult<()> {
    output::logo();
    output::newline();

    kv("Version", VERSION);
    kv("Binary", NAME);

    // Get Rust version
    #[cfg(debug_assertions)]
    let build_mode = "debug";
    #[cfg(not(debug_assertions))]
    let build_mode = "release";

    kv("Build", build_mode);

    // Features
    let mut features = Vec::new();

    #[cfg(feature = "postgres")]
    features.push("postgres");

    #[cfg(feature = "mysql")]
    features.push("mysql");

    #[cfg(feature = "sqlite")]
    features.push("sqlite");

    if features.is_empty() {
        features.push("none");
    }

    kv("Features", &features.join(", "));

    output::newline();

    // Additional info
    output::section("Components");
    kv("prax-schema", env!("CARGO_PKG_VERSION"));
    kv("prax-query", env!("CARGO_PKG_VERSION"));
    kv("prax-codegen", env!("CARGO_PKG_VERSION"));
    kv("prax-migrate", env!("CARGO_PKG_VERSION"));

    output::newline();
    output::dim("https://prax.dev");

    Ok(())
}

