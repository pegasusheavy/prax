//! Integration tests for configuration parsing and handling.
//!
//! These tests verify that the configuration system correctly handles
//! various configuration scenarios.

use prax::schema::PraxConfig;

/// Test minimal configuration
#[test]
fn test_config_minimal() {
    let config_str = r#"
        [database]
        url = "postgresql://localhost/test"
    "#;

    let config: PraxConfig = toml::from_str(config_str).expect("Failed to parse config");
    assert!(config.database.url.is_some());
}

/// Test full configuration with all options
#[test]
fn test_config_full() {
    let config_str = r#"
        [database]
        provider = "postgresql"
        url = "postgresql://user:pass@localhost:5432/mydb"

        [database.pool]
        min_connections = 2
        max_connections = 20
        connect_timeout = "30s"
        idle_timeout = "10m"
        max_lifetime = "30m"

        [schema]
        path = "prax/schema.prax"

        [generator.client]
        output = "./src/generated"
        async_client = true
        tracing = true

        [migrations]
        directory = "./prax/migrations"
        auto_migrate = false
        table_name = "_prax_migrations"

        [seed]
        script = "./seed.rs"
        auto_seed = false

        [debug]
        log_queries = false
        pretty_sql = true
        slow_query_threshold = 1000
    "#;

    let config: PraxConfig = toml::from_str(config_str).expect("Failed to parse config");

    // Database settings
    assert!(config.database.url.is_some());
    assert_eq!(config.database.pool.max_connections, 20);
}

/// Test environment variable interpolation syntax
#[test]
fn test_config_env_vars() {
    let config_str = r#"
        [database]
        url = "${DATABASE_URL}"
    "#;

    let config: PraxConfig = toml::from_str(config_str).expect("Failed to parse config");

    // Note: The value still has the ${} syntax until env expansion happens
    assert!(config.database.url.is_some());
}

/// Test schema path configuration
#[test]
fn test_config_schema_path() {
    let config_str = r#"
        [database]
        url = "postgresql://localhost/test"

        [schema]
        path = "custom/path/schema.prax"
    "#;

    let config: PraxConfig = toml::from_str(config_str).expect("Failed to parse config");
    assert_eq!(config.schema.path, "custom/path/schema.prax");
}

/// Test seed configuration
#[test]
fn test_config_seed() {
    let config_str = r#"
        [database]
        url = "postgresql://localhost/test"

        [seed]
        script = "./seed.ts"
        auto_seed = true
    "#;

    let config: PraxConfig = toml::from_str(config_str).expect("Failed to parse config");
    assert_eq!(config.seed.script, Some("./seed.ts".to_string()));
    assert!(config.seed.auto_seed);
}

/// Test debug/logging configuration
#[test]
fn test_config_debug() {
    let config_str = r#"
        [database]
        url = "postgresql://localhost/test"

        [debug]
        log_queries = true
        pretty_sql = true
        slow_query_threshold = 500
    "#;

    let config: PraxConfig = toml::from_str(config_str).expect("Failed to parse config");
    assert!(config.debug.log_queries);
    assert!(config.debug.pretty_sql);
    assert_eq!(config.debug.slow_query_threshold, 500);
}

/// Test connection pool timeout formats
#[test]
fn test_config_pool_timeouts() {
    let config_str = r#"
        [database]
        url = "postgresql://localhost/test"

        [database.pool]
        connect_timeout = "30s"
        idle_timeout = "10m"
        max_lifetime = "1h"
    "#;

    let config: PraxConfig = toml::from_str(config_str).expect("Failed to parse config");
    assert_eq!(config.database.pool.connect_timeout, "30s");
}

/// Test migration configuration
#[test]
fn test_config_migrations() {
    let config_str = r#"
        [database]
        url = "postgresql://localhost/test"

        [migrations]
        directory = "./prax/migrations"
        auto_migrate = false
        table_name = "_custom_migrations"
    "#;

    let config: PraxConfig = toml::from_str(config_str).expect("Failed to parse config");
    assert_eq!(config.migrations.directory, "./prax/migrations");
    assert!(!config.migrations.auto_migrate);
    assert_eq!(config.migrations.table_name, "_custom_migrations");
}

/// Test empty configuration sections
#[test]
fn test_config_empty_sections() {
    let config_str = r#"
        [database]
        url = "postgresql://localhost/test"

        [database.pool]
    "#;

    let config: PraxConfig = toml::from_str(config_str).expect("Failed to parse config");
    assert!(config.database.url.is_some());
}

/// Test generator configuration
#[test]
fn test_config_generator() {
    let config_str = r#"
        [database]
        url = "postgresql://localhost/test"

        [generator.client]
        output = "./src/generated/client"
        async_client = true
    "#;

    let config: PraxConfig = toml::from_str(config_str).expect("Failed to parse config");
    assert_eq!(config.generator.client.output, "./src/generated/client");
}
