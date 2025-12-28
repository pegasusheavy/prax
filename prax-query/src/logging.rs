//! Logging infrastructure for Prax ORM.
//!
//! This module provides structured JSON logging controlled by the `PRAX_DEBUG` environment variable.
//!
//! # Environment Variables
//!
//! - `PRAX_DEBUG=true` - Enable debug logging
//! - `PRAX_DEBUG=1` - Enable debug logging
//! - `PRAX_LOG_LEVEL=debug|info|warn|error|trace` - Set specific log level
//! - `PRAX_LOG_FORMAT=json|pretty|compact` - Set output format (default: json)
//!
//! # Usage
//!
//! ```rust,no_run
//! use prax_query::logging;
//!
//! // Initialize logging (call once at startup)
//! logging::init();
//!
//! // Or with custom settings
//! logging::init_with_level("debug");
//! ```
//!
//! # Internal Logging
//!
//! Within Prax, use the standard tracing macros:
//!
//! ```rust,ignore
//! use tracing::{debug, info, warn, error, trace};
//!
//! debug!(filter = ?filter, "Building SQL for filter");
//! info!(table = %table, "Executing query");
//! warn!(latency_ms = %ms, "Slow query detected");
//! error!(error = %e, "Query failed");
//! ```

use std::env;
use std::sync::Once;

static INIT: Once = Once::new();

/// Check if debug logging is enabled via `PRAX_DEBUG` environment variable.
///
/// Returns `true` if `PRAX_DEBUG` is set to "true", "1", or "yes" (case-insensitive).
#[inline]
pub fn is_debug_enabled() -> bool {
    env::var("PRAX_DEBUG")
        .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"))
        .unwrap_or(false)
}

/// Get the configured log level from `PRAX_LOG_LEVEL` environment variable.
///
/// Defaults to "debug" if `PRAX_DEBUG` is enabled, otherwise "warn".
pub fn get_log_level() -> &'static str {
    if let Ok(level) = env::var("PRAX_LOG_LEVEL") {
        match level.to_lowercase().as_str() {
            "trace" => "trace",
            "debug" => "debug",
            "info" => "info",
            "warn" => "warn",
            "error" => "error",
            _ => {
                if is_debug_enabled() {
                    "debug"
                } else {
                    "warn"
                }
            }
        }
    } else if is_debug_enabled() {
        "debug"
    } else {
        "warn"
    }
}

/// Get the configured log format from `PRAX_LOG_FORMAT` environment variable.
///
/// Defaults to "json" for structured logging.
pub fn get_log_format() -> &'static str {
    env::var("PRAX_LOG_FORMAT")
        .map(|f| match f.to_lowercase().as_str() {
            "pretty" => "pretty",
            "compact" => "compact",
            _ => "json",
        })
        .unwrap_or("json")
}

/// Initialize the Prax logging system.
///
/// This should be called once at application startup. Subsequent calls are no-ops.
///
/// Logging is controlled by:
/// - `PRAX_DEBUG=true` - Enable debug-level logging
/// - `PRAX_LOG_LEVEL` - Override the log level (trace, debug, info, warn, error)
/// - `PRAX_LOG_FORMAT` - Output format (pretty, json, compact)
///
/// # Example
///
/// ```rust,no_run
/// use prax_query::logging;
///
/// // Initialize at the start of your application
/// logging::init();
/// ```
pub fn init() {
    INIT.call_once(|| {
        if !is_debug_enabled() && env::var("PRAX_LOG_LEVEL").is_err() {
            // No logging requested, skip initialization
            return;
        }

        #[cfg(feature = "tracing-subscriber")]
        {
            use tracing_subscriber::{EnvFilter, fmt, prelude::*};

            let level = get_log_level();
            let filter = EnvFilter::try_new(format!(
                "prax={},prax_query={},prax_schema={}",
                level, level, level
            ))
            .unwrap_or_else(|_| EnvFilter::new("warn"));

            match get_log_format() {
                "json" => {
                    tracing_subscriber::registry()
                        .with(filter)
                        .with(fmt::layer().json())
                        .init();
                }
                "compact" => {
                    tracing_subscriber::registry()
                        .with(filter)
                        .with(fmt::layer().compact())
                        .init();
                }
                _ => {
                    tracing_subscriber::registry()
                        .with(filter)
                        .with(fmt::layer().pretty())
                        .init();
                }
            }

            tracing::info!(
                level = level,
                format = get_log_format(),
                "Prax logging initialized"
            );
        }

        #[cfg(not(feature = "tracing-subscriber"))]
        {
            // Tracing subscriber not available, logging will be silent
            // unless the user sets up their own subscriber
        }
    });
}

/// Initialize logging with a specific level.
///
/// # Example
///
/// ```rust,no_run
/// use prax_query::logging;
///
/// // Enable trace-level logging
/// logging::init_with_level("trace");
/// ```
///
/// # Safety
///
/// This function modifies environment variables, which is unsafe in
/// multi-threaded programs. Call this early in your program before
/// spawning threads.
pub fn init_with_level(level: &str) {
    // SAFETY: This should only be called at program startup before threads are spawned.
    // The user is responsible for calling this safely.
    unsafe {
        env::set_var("PRAX_LOG_LEVEL", level);
    }
    init();
}

/// Initialize logging for debugging (convenience function).
///
/// Equivalent to setting `PRAX_DEBUG=true` and calling `init()`.
///
/// # Safety
///
/// This function modifies environment variables, which is unsafe in
/// multi-threaded programs. Call this early in your program before
/// spawning threads.
pub fn init_debug() {
    // SAFETY: This should only be called at program startup before threads are spawned.
    unsafe {
        env::set_var("PRAX_DEBUG", "true");
    }
    init();
}

/// Macro for conditional debug logging.
///
/// Only logs if `PRAX_DEBUG` is enabled at runtime.
#[macro_export]
macro_rules! prax_debug {
    ($($arg:tt)*) => {
        if $crate::logging::is_debug_enabled() {
            tracing::debug!($($arg)*);
        }
    };
}

/// Macro for conditional trace logging.
#[macro_export]
macro_rules! prax_trace {
    ($($arg:tt)*) => {
        if $crate::logging::is_debug_enabled() {
            tracing::trace!($($arg)*);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_disabled_by_default() {
        // Clear env var to test default behavior
        // SAFETY: Test runs in isolation
        unsafe {
            env::remove_var("PRAX_DEBUG");
        }
        assert!(!is_debug_enabled());
    }

    #[test]
    fn test_log_level_default() {
        // SAFETY: Test runs in isolation
        unsafe {
            env::remove_var("PRAX_DEBUG");
            env::remove_var("PRAX_LOG_LEVEL");
        }
        assert_eq!(get_log_level(), "warn");
    }
}
