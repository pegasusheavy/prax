import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-advanced-connection',
  standalone: true,
  imports: [CommonModule, CodeBlockComponent],
  templateUrl: './advanced-connection.page.html',
})
export class AdvancedConnectionPage {
  parseUrl = `use prax_query::connection::{ConnectionString, Driver};

// Parse a connection URL
let conn = ConnectionString::parse(
    "postgres://user:pass@localhost:5432/mydb?sslmode=require"
)?;

// Access components
assert_eq!(conn.driver(), Driver::Postgres);
assert_eq!(conn.user(), Some("user"));
assert_eq!(conn.password(), Some("pass"));
assert_eq!(conn.host(), Some("localhost"));
assert_eq!(conn.port(), Some(5432));
assert_eq!(conn.database(), Some("mydb"));
assert_eq!(conn.param("sslmode"), Some("require"));

// From environment variable
let conn = ConnectionString::from_database_url()?;  // Uses DATABASE_URL
let conn = ConnectionString::from_env("MY_DB_URL")?;`;

  urlFormats = `// PostgreSQL
postgres://user:password@host:5432/database
postgresql://user:password@host:5432/database

// MySQL
mysql://user:password@host:3306/database
mariadb://user:password@host:3306/database

// SQLite
sqlite://./path/to/database.db
sqlite::memory:  // In-memory database

// With query parameters
postgres://user:pass@host/db?sslmode=require&connect_timeout=10`;

  builderPattern = `use prax_query::connection::{DatabaseConfig, SslMode};
use std::time::Duration;

// PostgreSQL configuration
let config = DatabaseConfig::postgres()
    .host("localhost")
    .port(5432)
    .database("mydb")
    .user("user")
    .password("pass")
    .ssl_mode(SslMode::Require)
    .connect_timeout(Duration::from_secs(10))
    .max_connections(20)
    .min_connections(5)
    .idle_timeout(Duration::from_secs(300))
    .application_name("my-app")
    .build()?;

// MySQL configuration
let config = DatabaseConfig::mysql()
    .host("localhost")
    .database("mydb")
    .user("root")
    .mysql_options(|opts| opts
        .charset("utf8mb4")
        .compression(true)
    )
    .build()?;

// SQLite configuration
let config = DatabaseConfig::sqlite()
    .database("./data/app.db")
    .sqlite_options(|opts| opts
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(5000)
    )
    .build()?;`;

  multiDatabase = `use prax_query::connection::{
    MultiDatabaseConfig, DatabaseConfig, LoadBalanceStrategy
};

// Configure primary + read replicas
let config = MultiDatabaseConfig::new()
    .primary(DatabaseConfig::from_url("postgres://primary/db")?)
    .replica(DatabaseConfig::from_url("postgres://replica1/db")?)
    .replica(DatabaseConfig::from_url("postgres://replica2/db")?)
    .load_balance(LoadBalanceStrategy::RoundRobin);

// Named databases for different purposes
let config = MultiDatabaseConfig::new()
    .primary(DatabaseConfig::from_url("postgres://main/db")?)
    .database("analytics", DatabaseConfig::from_url("postgres://analytics/db")?)
    .database("cache", DatabaseConfig::from_url("postgres://cache/db")?);

// Access specific database
let primary = config.get_primary();
let analytics = config.get("analytics");`;

  envExpansion = `use prax_query::connection::EnvExpander;

// Expand environment variables in connection strings
let expander = EnvExpander::new();

// Supported syntax:
// \${VAR}          - Required variable
// \${VAR:-default} - Variable with default value
// \${VAR:?error}   - Required with custom error message

let url = expander.expand(
    "postgres://\${DB_USER}:\${DB_PASS}@\${DB_HOST:-localhost}:\${DB_PORT:-5432}/\${DB_NAME}"
)?;

// Check for variables before expanding
if EnvExpander::has_variables(&url) {
    let expanded = expander.expand(&url)?;
}`;

  poolConfig = `use prax_query::connection::PoolConfig;
use std::time::Duration;

// Default configuration
let pool = PoolConfig::new()
    .max_connections(20)
    .min_connections(5)
    .connect_timeout(Duration::from_secs(30))
    .acquire_timeout(Duration::from_secs(30))
    .idle_timeout(Duration::from_secs(600))
    .max_lifetime(Duration::from_secs(1800))
    .retry_attempts(3)
    .retry_delay(Duration::from_millis(500))
    .health_check_interval(Duration::from_secs(30));

// Preset configurations
let pool = PoolConfig::low_latency();      // Optimized for fast responses
let pool = PoolConfig::high_throughput();  // Optimized for many connections
let pool = PoolConfig::development();      // Minimal config for dev`;

  sslConfig = `use prax_query::connection::{SslMode, SslConfig};

// SSL modes
SslMode::Disable     // No SSL
SslMode::Allow       // Allow but don't require
SslMode::Prefer      // Prefer SSL (default)
SslMode::Require     // Require SSL
SslMode::VerifyCa    // Verify server certificate
SslMode::VerifyFull  // Verify certificate + hostname

// Full SSL configuration
let ssl = SslConfig::new(SslMode::VerifyFull)
    .with_ca_cert("/path/to/ca.crt")
    .with_client_cert("/path/to/client.crt")
    .with_client_key("/path/to/client.key");

let config = DatabaseConfig::postgres()
    .host("secure-db.example.com")
    .ssl(ssl)
    .build()?;`;
}


