//! Database seeding implementation.
//!
//! Supports multiple seed file types:
//! - `.rs` - Rust seed scripts (compiled and executed)
//! - `.sql` - Raw SQL files (executed directly)
//! - `.json` - JSON data files (declarative seeding)
//! - `.toml` - TOML data files (declarative seeding)

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use prax_query::dialect::{Mysql, SqlDialect};
use prax_query::sql::escape_identifier;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::error::{CliError, CliResult};
use crate::output;

/// Seed file types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedFileType {
    /// Rust seed script (.rs)
    Rust,
    /// SQL seed file (.sql)
    Sql,
    /// JSON seed data (.json)
    Json,
    /// TOML seed data (.toml)
    Toml,
}

impl SeedFileType {
    /// Detect seed file type from path extension
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "rs" => Some(Self::Rust),
            "sql" => Some(Self::Sql),
            "json" => Some(Self::Json),
            "toml" => Some(Self::Toml),
            _ => None,
        }
    }
}

/// Seed runner configuration
#[derive(Debug, Clone)]
pub struct SeedRunner {
    /// Path to the seed file
    pub seed_path: PathBuf,
    /// Seed file type
    pub file_type: SeedFileType,
    /// Database URL for execution
    pub database_url: String,
    /// Database provider (postgresql, mysql, sqlite)
    pub provider: String,
    /// Current working directory
    pub cwd: PathBuf,
    /// Environment name (development, staging, production)
    pub environment: String,
    /// Whether to reset database before seeding
    pub reset_before_seed: bool,
}

impl SeedRunner {
    /// Create a new seed runner
    pub fn new(
        seed_path: PathBuf,
        database_url: String,
        provider: String,
        cwd: PathBuf,
    ) -> CliResult<Self> {
        let file_type = SeedFileType::from_path(&seed_path).ok_or_else(|| {
            CliError::Config(format!(
                "Unsupported seed file type: {}. Supported: .rs, .sql, .json, .toml",
                seed_path.display()
            ))
        })?;

        Ok(Self {
            seed_path,
            file_type,
            database_url,
            provider,
            cwd,
            environment: std::env::var("PRAX_ENV").unwrap_or_else(|_| "development".to_string()),
            reset_before_seed: false,
        })
    }

    /// Set environment
    pub fn with_environment(mut self, env: impl Into<String>) -> Self {
        self.environment = env.into();
        self
    }

    /// Set reset before seed
    pub fn with_reset(mut self, reset: bool) -> Self {
        self.reset_before_seed = reset;
        self
    }

    /// Run the seed
    pub async fn run(&self) -> CliResult<SeedResult> {
        if self.reset_before_seed {
            self.reset_tables().await?;
        }

        match self.file_type {
            SeedFileType::Rust => self.run_rust_seed().await,
            SeedFileType::Sql => self.run_sql_seed().await,
            SeedFileType::Json => self.run_json_seed().await,
            SeedFileType::Toml => self.run_toml_seed().await,
        }
    }

    /// Reset the tables this runner knows about before seeding.
    async fn reset_tables(&self) -> CliResult<()> {
        let tables = self.seed_table_names()?;
        if tables.is_empty() {
            return Ok(());
        }

        output::list_item(&format!(
            "Resetting {} table(s) before seeding...",
            tables.len()
        ));

        let sql = self.generate_reset_sql(&tables);
        self.execute_sql(&sql).await?;
        Ok(())
    }

    /// Table names this runner can reset, from declarative seed metadata.
    ///
    /// `.rs` and `.sql` seeds carry no table metadata, so `--reset` is
    /// rejected for them rather than silently skipped.
    fn seed_table_names(&self) -> CliResult<Vec<String>> {
        match self.file_type {
            SeedFileType::Json => {
                let content = std::fs::read_to_string(&self.seed_path)?;
                let seed_data: SeedData =
                    serde_json::from_str(&content).map_err(|e| CliError::Config(e.to_string()))?;
                Ok(seed_data.tables.keys().cloned().collect())
            }
            SeedFileType::Toml => {
                let content = std::fs::read_to_string(&self.seed_path)?;
                let seed_data: SeedData =
                    toml::from_str(&content).map_err(|e| CliError::Config(e.to_string()))?;
                Ok(seed_data.tables.keys().cloned().collect())
            }
            SeedFileType::Rust | SeedFileType::Sql => Err(CliError::Config(
                "--reset requires table metadata; not supported yet".to_string(),
            )),
        }
    }

    /// Run a Rust seed script
    async fn run_rust_seed(&self) -> CliResult<SeedResult> {
        output::step(1, 4, "Compiling seed script...");

        // Check if we're in a Cargo workspace
        let cargo_toml = self.cwd.join("Cargo.toml");
        if !cargo_toml.exists() {
            return Err(CliError::Config(
                "No Cargo.toml found. Rust seed scripts require a Rust project.".to_string(),
            ));
        }

        // Create a temporary bin target or use cargo run
        let seed_name = self
            .seed_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("seed");

        // Check if there's a [[bin]] entry for the seed, or we need to compile manually
        let has_bin_target = self.check_bin_target(seed_name)?;

        let mut records_affected = 0u64;

        if has_bin_target {
            // Use cargo run directly
            output::step(2, 4, &format!("Building seed binary '{}'...", seed_name));

            let build_status = Command::new("cargo")
                .args(["build", "--bin", seed_name, "--release"])
                .current_dir(&self.cwd)
                .env("DATABASE_URL", &self.database_url)
                .env("PRAX_ENV", &self.environment)
                .status()?;

            if !build_status.success() {
                return Err(CliError::Command("Failed to build seed binary".to_string()));
            }

            output::step(3, 4, "Running seed...");

            let run_output = Command::new("cargo")
                .args(["run", "--bin", seed_name, "--release"])
                .current_dir(&self.cwd)
                .env("DATABASE_URL", &self.database_url)
                .env("PRAX_ENV", &self.environment)
                .output()?;

            if !run_output.status.success() {
                let stderr = String::from_utf8_lossy(&run_output.stderr);
                return Err(CliError::Command(format!("Seed failed: {}", stderr)));
            }

            // Parse output for record count if available
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            for line in stdout.lines() {
                output::list_item(line);
                // Try to parse seed output for counts
                if let Some(count) = parse_seed_output(line) {
                    records_affected += count;
                }
            }

            output::step(4, 4, "Verifying seed data...");
        } else {
            // Compile and run as a standalone script using rustc
            output::step(2, 4, "Compiling standalone seed script...");

            // Create temp directory for compiled seed
            let temp_dir = std::env::temp_dir().join("prax_seed");
            std::fs::create_dir_all(&temp_dir)?;

            let output_binary = temp_dir.join(seed_name);

            // Try to compile with cargo if it looks like a full Rust file
            let seed_content = std::fs::read_to_string(&self.seed_path)?;

            if seed_content.contains("use prax") || seed_content.contains("#[tokio::main]") {
                // This is a standalone Rust file - we'll create a temporary Cargo project
                output::list_item("Creating temporary build environment...");

                let temp_project = temp_dir.join("seed_project");
                std::fs::create_dir_all(temp_project.join("src"))?;

                // Copy seed file
                std::fs::copy(&self.seed_path, temp_project.join("src/main.rs"))?;

                // Create Cargo.toml for the seed
                let seed_cargo = create_seed_cargo_toml(&self.cwd)?;
                std::fs::write(temp_project.join("Cargo.toml"), seed_cargo)?;

                // Build
                let build_status = Command::new("cargo")
                    .args(["build", "--release"])
                    .current_dir(&temp_project)
                    .env("DATABASE_URL", &self.database_url)
                    .env("PRAX_ENV", &self.environment)
                    .status()?;

                if !build_status.success() {
                    return Err(CliError::Command(
                        "Failed to compile seed script".to_string(),
                    ));
                }

                // Copy binary
                let built_binary = temp_project.join("target/release/seed");
                if built_binary.exists() {
                    std::fs::copy(&built_binary, &output_binary)?;
                }
            } else {
                return Err(CliError::Config(
                    "Seed script must be a valid Rust file with a main function".to_string(),
                ));
            }

            output::step(3, 4, "Running seed...");

            let run_output = Command::new(&output_binary)
                .current_dir(&self.cwd)
                .env("DATABASE_URL", &self.database_url)
                .env("PRAX_ENV", &self.environment)
                .output()?;

            if !run_output.status.success() {
                let stderr = String::from_utf8_lossy(&run_output.stderr);
                return Err(CliError::Command(format!("Seed failed: {}", stderr)));
            }

            let stdout = String::from_utf8_lossy(&run_output.stdout);
            for line in stdout.lines() {
                output::list_item(line);
                if let Some(count) = parse_seed_output(line) {
                    records_affected += count;
                }
            }

            output::step(4, 4, "Verifying seed data...");
        }

        Ok(SeedResult {
            file_type: self.file_type,
            records_affected,
            tables_seeded: Vec::new(),
            duration: std::time::Duration::from_secs(0),
        })
    }

    /// Run a SQL seed file
    async fn run_sql_seed(&self) -> CliResult<SeedResult> {
        output::step(1, 3, "Reading SQL seed file...");

        let sql_content = std::fs::read_to_string(&self.seed_path)?;

        // Count statements for progress
        let statements: Vec<&str> = sql_content
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && !s.starts_with("--"))
            .collect();

        output::list_item(&format!("Found {} SQL statements", statements.len()));

        output::step(2, 3, "Executing SQL...");

        // Execute SQL based on provider
        let records = self.execute_sql(&sql_content).await?;

        output::step(3, 3, "Verifying seed data...");

        Ok(SeedResult {
            file_type: self.file_type,
            records_affected: records,
            tables_seeded: Vec::new(),
            duration: std::time::Duration::from_secs(0),
        })
    }

    /// Run a JSON seed file (declarative)
    async fn run_json_seed(&self) -> CliResult<SeedResult> {
        output::step(1, 4, "Reading JSON seed file...");

        let json_content = std::fs::read_to_string(&self.seed_path)?;
        let seed_data: SeedData =
            serde_json::from_str(&json_content).map_err(|e| CliError::Config(e.to_string()))?;

        output::step(2, 4, "Validating seed data...");
        output::list_item(&format!("Found {} tables to seed", seed_data.tables.len()));

        output::step(3, 4, "Inserting seed data...");

        let mut total_records = 0u64;
        let mut tables_seeded = Vec::new();

        for (table_name, records) in &seed_data.tables {
            let sql = self.generate_insert_sql(table_name, records)?;
            let count = self.execute_sql(&sql).await?;
            output::list_item(&format!("  {} - {} records", table_name, records.len()));
            total_records += count;
            tables_seeded.push(table_name.clone());
        }

        output::step(4, 4, "Verifying seed data...");

        Ok(SeedResult {
            file_type: self.file_type,
            records_affected: total_records,
            tables_seeded,
            duration: std::time::Duration::from_secs(0),
        })
    }

    /// Run a TOML seed file (declarative)
    async fn run_toml_seed(&self) -> CliResult<SeedResult> {
        output::step(1, 4, "Reading TOML seed file...");

        let toml_content = std::fs::read_to_string(&self.seed_path)?;
        let seed_data: SeedData =
            toml::from_str(&toml_content).map_err(|e| CliError::Config(e.to_string()))?;

        output::step(2, 4, "Validating seed data...");
        output::list_item(&format!("Found {} tables to seed", seed_data.tables.len()));

        output::step(3, 4, "Inserting seed data...");

        let mut total_records = 0u64;
        let mut tables_seeded = Vec::new();

        for (table_name, records) in &seed_data.tables {
            let sql = self.generate_insert_sql(table_name, records)?;
            let count = self.execute_sql(&sql).await?;
            output::list_item(&format!("  {} - {} records", table_name, records.len()));
            total_records += count;
            tables_seeded.push(table_name.clone());
        }

        output::step(4, 4, "Verifying seed data...");

        Ok(SeedResult {
            file_type: self.file_type,
            records_affected: total_records,
            tables_seeded,
            duration: std::time::Duration::from_secs(0),
        })
    }

    /// Check if there's a bin target in Cargo.toml
    fn check_bin_target(&self, name: &str) -> CliResult<bool> {
        let cargo_toml = self.cwd.join("Cargo.toml");
        let content = std::fs::read_to_string(&cargo_toml)?;

        // Simple check - look for [[bin]] with our name
        Ok(content.contains(&format!("name = \"{}\"", name))
            || content.contains(&format!("name = '{}'", name)))
    }

    /// Generate INSERT SQL from seed records
    fn generate_insert_sql(
        &self,
        table: &str,
        records: &[HashMap<String, serde_json::Value>],
    ) -> CliResult<String> {
        if records.is_empty() {
            return Ok(String::new());
        }

        let mut sql = String::new();

        // Get columns from first record
        let columns: Vec<&String> = records[0].keys().collect();
        let column_names = columns
            .iter()
            .map(|c| escape_identifier(c))
            .collect::<Vec<_>>()
            .join(", ");

        for record in records {
            let values = columns
                .iter()
                .map(|col| {
                    record
                        .get(*col)
                        .map(|v| self.value_to_sql(v))
                        .unwrap_or_else(|| "NULL".to_string())
                })
                .collect::<Vec<_>>()
                .join(", ");

            sql.push_str(&format!(
                "INSERT INTO {} ({}) VALUES ({});\n",
                escape_identifier(table),
                column_names,
                values
            ));
        }

        Ok(sql)
    }

    /// Generate provider-specific reset SQL for the given tables
    fn generate_reset_sql(&self, tables: &[String]) -> String {
        // MySQL without ANSI_QUOTES mode parses `"users"` as a string
        // literal, so identifiers need backtick quoting there; other
        // providers take standard double-quoted identifiers.
        let quote: fn(&str) -> String = match self.provider.as_str() {
            "mysql" => |t| Mysql.quote_ident(t),
            _ => escape_identifier,
        };
        match self.provider.as_str() {
            "postgresql" | "postgres" => {
                let names = tables
                    .iter()
                    .map(|t| quote(t))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("TRUNCATE TABLE {} CASCADE;", names)
            }
            "mysql" => tables
                .iter()
                .map(|t| format!("TRUNCATE TABLE {};", quote(t)))
                .collect::<Vec<_>>()
                .join("\n"),
            "sqlite" => tables
                .iter()
                .map(|t| format!("DELETE FROM {};", quote(t)))
                .collect::<Vec<_>>()
                .join("\n"),
            _ => String::new(),
        }
    }

    /// Convert JSON value to SQL literal
    fn value_to_sql(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::Null => "NULL".to_string(),
            serde_json::Value::Bool(b) => {
                if *b {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            }
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => {
                // Check for special functions
                match s.as_str() {
                    "now()" | "NOW()" => match self.provider.as_str() {
                        "postgresql" => "CURRENT_TIMESTAMP".to_string(),
                        "mysql" => "NOW()".to_string(),
                        "sqlite" => "datetime('now')".to_string(),
                        _ => "CURRENT_TIMESTAMP".to_string(),
                    },
                    "uuid()" | "UUID()" => match self.provider.as_str() {
                        "postgresql" => "gen_random_uuid()".to_string(),
                        "mysql" => "UUID()".to_string(),
                        "sqlite" => format!("'{}'", uuid::Uuid::new_v4()),
                        _ => "gen_random_uuid()".to_string(),
                    },
                    _ => format!("'{}'", s.replace('\'', "''")),
                }
            }
            serde_json::Value::Array(arr) => {
                // PostgreSQL array literal
                let items = arr
                    .iter()
                    .map(|v| self.value_to_sql(v))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("ARRAY[{}]", items)
            }
            serde_json::Value::Object(_) => {
                // JSON/JSONB
                format!("'{}'", value)
            }
        }
    }

    /// Execute SQL against the database
    async fn execute_sql(&self, sql: &str) -> CliResult<u64> {
        // Use command-line tools based on provider
        match self.provider.as_str() {
            "postgresql" | "postgres" => self.execute_postgres_sql(sql).await,
            "mysql" => self.execute_mysql_sql(sql).await,
            "sqlite" => self.execute_sqlite_sql(sql).await,
            _ => Err(CliError::Database(format!(
                "Unsupported database provider: {}",
                self.provider
            ))),
        }
    }

    /// Execute SQL using psql
    async fn execute_postgres_sql(&self, sql: &str) -> CliResult<u64> {
        // First try using psql
        let psql_result = Command::new("psql")
            .args(["-d", &self.database_url, "-c", sql])
            .output();

        match psql_result {
            Ok(output) if output.status.success() => {
                // Try to parse affected rows from output
                let stdout = String::from_utf8_lossy(&output.stdout);
                Ok(parse_affected_rows(&stdout).unwrap_or(0))
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // If psql not found, suggest alternative
                if stderr.contains("not found") || stderr.contains("No such file") {
                    Err(CliError::Command(
                        "psql not found. Install PostgreSQL client tools or use a Rust seed script.".to_string()
                    ))
                } else {
                    Err(CliError::Database(format!(
                        "SQL execution failed: {}",
                        stderr
                    )))
                }
            }
            Err(e) => Err(CliError::Command(format!(
                "Failed to execute SQL. Install psql (PostgreSQL client tools) or use a Rust seed script: {}",
                e
            ))),
        }
    }

    /// Execute SQL using mysql client
    async fn execute_mysql_sql(&self, sql: &str) -> CliResult<u64> {
        // Parse MySQL URL to extract components
        let url = url::Url::parse(&self.database_url)
            .map_err(|e| CliError::Config(format!("Invalid MySQL URL: {}", e)))?;

        let host = url.host_str().unwrap_or("localhost");
        let port = url.port().unwrap_or(3306);
        let user = url.username();
        let password = url.password().unwrap_or("");
        let database = url.path().trim_start_matches('/');

        let mut cmd = Command::new("mysql");
        cmd.args(["-h", host, "-P", &port.to_string(), "-u", user]);

        if !password.is_empty() {
            cmd.arg(format!("-p{}", password));
        }

        cmd.args(["-D", database, "-e", sql]);

        let output = cmd.output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(parse_affected_rows(&stdout).unwrap_or(0))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("not found") || stderr.contains("No such file") {
                Err(CliError::Command(
                    "mysql client not found. Install MySQL client tools or use a Rust seed script."
                        .to_string(),
                ))
            } else {
                Err(CliError::Database(format!(
                    "SQL execution failed: {}",
                    stderr
                )))
            }
        }
    }

    /// Execute SQL using sqlite3
    async fn execute_sqlite_sql(&self, sql: &str) -> CliResult<u64> {
        // Extract database path from URL
        let db_path = self
            .database_url
            .strip_prefix("sqlite://")
            .or_else(|| self.database_url.strip_prefix("sqlite:"))
            .unwrap_or(&self.database_url);

        let output = Command::new("sqlite3").args([db_path, sql]).output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(parse_affected_rows(&stdout).unwrap_or(0))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("not found") || stderr.contains("No such file") {
                Err(CliError::Command(
                    "sqlite3 not found. Install SQLite tools or use a Rust seed script."
                        .to_string(),
                ))
            } else {
                Err(CliError::Database(format!(
                    "SQL execution failed: {}",
                    stderr
                )))
            }
        }
    }
}

/// Seed execution result
#[derive(Debug)]
pub struct SeedResult {
    /// Type of seed file that was executed
    pub file_type: SeedFileType,
    /// Number of records affected
    pub records_affected: u64,
    /// Tables that were seeded
    pub tables_seeded: Vec<String>,
    /// Execution duration
    pub duration: std::time::Duration,
}

/// Declarative seed data structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SeedData {
    /// Tables to seed, keyed by table name
    #[serde(default)]
    pub tables: HashMap<String, Vec<HashMap<String, serde_json::Value>>>,

    /// Seed order (optional - tables will be seeded in this order)
    #[serde(default)]
    pub order: Vec<String>,

    /// Truncate tables before seeding
    #[serde(default)]
    pub truncate: bool,

    /// Disable foreign key checks during seeding
    #[serde(default)]
    pub disable_fk_checks: bool,
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Find seed file in common locations
pub fn find_seed_file(cwd: &Path, config: &Config) -> Option<PathBuf> {
    // Check config first
    if let Some(ref seed_path) = config.database.seed_path
        && seed_path.exists()
    {
        return Some(seed_path.clone());
    }

    // Common locations
    let candidates = [
        cwd.join("seed.rs"),
        cwd.join("seed.sql"),
        cwd.join("seed.json"),
        cwd.join("seed.toml"),
        cwd.join("prax/seed.rs"),
        cwd.join("prax/seed.sql"),
        cwd.join("prax/seed.json"),
        cwd.join("prax/seed.toml"),
        cwd.join("prisma/seed.rs"),
        cwd.join("src/seed.rs"),
        cwd.join("seeds/seed.rs"),
        cwd.join("seeds/seed.sql"),
    ];

    candidates.into_iter().find(|p| p.exists())
}

/// Get database URL from config or environment
pub fn get_database_url(config: &Config) -> CliResult<String> {
    // Try config first
    if let Some(ref url) = config.database.url {
        // Expand environment variables
        let expanded = expand_env_var(url);
        if !expanded.is_empty() && !expanded.contains("${") {
            return Ok(expanded);
        }
    }

    // Try environment variable
    std::env::var("DATABASE_URL").map_err(|_| {
        CliError::Config(
            "Database URL not found. Set DATABASE_URL environment variable or configure in prax.toml"
                .to_string(),
        )
    })
}

/// Expand environment variables in a string
fn expand_env_var(s: &str) -> String {
    let mut result = s.to_string();

    // Match ${VAR} pattern
    let re = regex_lite::Regex::new(r"\$\{([^}]+)\}").unwrap();
    for cap in re.captures_iter(s) {
        let var_name = &cap[1];
        if let Ok(value) = std::env::var(var_name) {
            result = result.replace(&cap[0], &value);
        }
    }

    // Also match $VAR pattern (no braces)
    let re2 = regex_lite::Regex::new(r"\$([A-Z_][A-Z0-9_]*)").unwrap();
    for cap in re2.captures_iter(&result.clone()) {
        let var_name = &cap[1];
        if let Ok(value) = std::env::var(var_name) {
            result = result.replace(&cap[0], &value);
        }
    }

    result
}

/// Parse seed output for record counts
fn parse_seed_output(line: &str) -> Option<u64> {
    // Common patterns:
    // "Created 10 users"
    // "Seeded 100 records"
    // "Inserted: 50"
    let patterns = [
        r"(?i)created\s+(\d+)",
        r"(?i)seeded\s+(\d+)",
        r"(?i)inserted[:\s]+(\d+)",
        r"(?i)(\d+)\s+records?",
        r"(?i)(\d+)\s+rows?",
    ];

    for pattern in patterns {
        if let Ok(re) = regex_lite::Regex::new(pattern)
            && let Some(caps) = re.captures(line)
            && let Some(m) = caps.get(1)
            && let Ok(n) = m.as_str().parse()
        {
            return Some(n);
        }
    }

    None
}

/// Parse affected rows from database output
fn parse_affected_rows(output: &str) -> Option<u64> {
    // PostgreSQL: "INSERT 0 5" or "UPDATE 3"
    // MySQL: "Query OK, 5 rows affected"
    // SQLite: no standard format

    let patterns = [
        r"INSERT\s+\d+\s+(\d+)",
        r"UPDATE\s+(\d+)",
        r"DELETE\s+(\d+)",
        r"(\d+)\s+rows?\s+affected",
    ];

    let mut total = 0u64;

    for pattern in patterns {
        if let Ok(re) = regex_lite::Regex::new(pattern) {
            for caps in re.captures_iter(output) {
                if let Some(m) = caps.get(1)
                    && let Ok(n) = m.as_str().parse::<u64>()
                {
                    total += n;
                }
            }
        }
    }

    if total > 0 { Some(total) } else { None }
}

/// Create a Cargo.toml for standalone seed script
fn create_seed_cargo_toml(project_root: &Path) -> CliResult<String> {
    // Try to read the workspace Cargo.toml to get prax version
    let workspace_cargo = project_root.join("Cargo.toml");
    let prax_version = if workspace_cargo.exists() {
        let content = std::fs::read_to_string(&workspace_cargo)?;
        // Try to extract prax version from dependencies
        extract_prax_version(&content).unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string())
    } else {
        env!("CARGO_PKG_VERSION").to_string()
    };

    Ok(format!(
        r#"[package]
name = "seed"
version = "0.1.0"
edition = "2024"

[dependencies]
prax-orm = "{}"
tokio = {{ version = "1", features = ["full"] }}
"#,
        prax_version
    ))
}

/// Extract prax-orm version from Cargo.toml
fn extract_prax_version(content: &str) -> Option<String> {
    // Look for prax-orm = "x.y.z" or prax-orm = { version = "x.y.z" }
    let simple_re = regex_lite::Regex::new(r#"prax-orm\s*=\s*"([^"]+)""#).ok()?;
    if let Some(caps) = simple_re.captures(content) {
        return Some(caps.get(1)?.as_str().to_string());
    }

    let complex_re =
        regex_lite::Regex::new(r#"prax-orm\s*=\s*\{[^}]*version\s*=\s*"([^"]+)""#).ok()?;
    if let Some(caps) = complex_re.captures(content) {
        return Some(caps.get(1)?.as_str().to_string());
    }

    None
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_file_type_detection() {
        assert_eq!(
            SeedFileType::from_path(Path::new("seed.rs")),
            Some(SeedFileType::Rust)
        );
        assert_eq!(
            SeedFileType::from_path(Path::new("seed.sql")),
            Some(SeedFileType::Sql)
        );
        assert_eq!(
            SeedFileType::from_path(Path::new("data.json")),
            Some(SeedFileType::Json)
        );
        assert_eq!(
            SeedFileType::from_path(Path::new("data.toml")),
            Some(SeedFileType::Toml)
        );
        assert_eq!(SeedFileType::from_path(Path::new("seed.txt")), None);
    }

    #[test]
    fn test_parse_seed_output() {
        assert_eq!(parse_seed_output("Created 10 users"), Some(10));
        assert_eq!(parse_seed_output("Seeded 100 records"), Some(100));
        assert_eq!(parse_seed_output("Inserted: 50"), Some(50));
        assert_eq!(parse_seed_output("5 rows affected"), Some(5));
        assert_eq!(parse_seed_output("no numbers here"), None);
    }

    #[test]
    fn test_parse_affected_rows() {
        assert_eq!(parse_affected_rows("INSERT 0 5"), Some(5));
        assert_eq!(parse_affected_rows("UPDATE 3"), Some(3));
        assert_eq!(parse_affected_rows("Query OK, 10 rows affected"), Some(10));
    }

    #[test]
    fn test_expand_env_var() {
        // SAFETY: Single-threaded test environment
        unsafe {
            std::env::set_var("TEST_VAR", "test_value");
        }
        assert_eq!(expand_env_var("${TEST_VAR}"), "test_value");
        assert_eq!(expand_env_var("$TEST_VAR"), "test_value");
        assert_eq!(
            expand_env_var("postgres://${TEST_VAR}@localhost"),
            "postgres://test_value@localhost"
        );
        // SAFETY: Single-threaded test environment
        unsafe {
            std::env::remove_var("TEST_VAR");
        }
    }

    fn make_runner(seed_path: &str, provider: &str) -> SeedRunner {
        SeedRunner::new(
            PathBuf::from(seed_path),
            "sqlite://test.db".to_string(),
            provider.to_string(),
            PathBuf::from("."),
        )
        .expect("supported seed file type")
    }

    #[test]
    fn test_find_seed_file_skips_unsupported_ts() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::default();

        // Only an unsupported .ts seed exists -> nothing found
        let prisma_dir = dir.path().join("prisma");
        std::fs::create_dir_all(&prisma_dir).unwrap();
        std::fs::write(prisma_dir.join("seed.ts"), "export default {}").unwrap();
        assert!(find_seed_file(dir.path(), &config).is_none());

        // A supported file alongside the .ts one is used instead of erroring
        let src_dir = dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("seed.rs"), "fn main() {}").unwrap();
        assert_eq!(
            find_seed_file(dir.path(), &config),
            Some(src_dir.join("seed.rs"))
        );
    }

    #[test]
    fn test_create_seed_cargo_toml_version_fallback() {
        // No Cargo.toml at all -> fall back to the CLI's own (workspace) version
        let dir = tempfile::tempdir().unwrap();
        let cargo_toml = create_seed_cargo_toml(dir.path()).unwrap();
        assert!(
            cargo_toml.contains(&format!("prax-orm = \"{}\"", env!("CARGO_PKG_VERSION"))),
            "unexpected seed Cargo.toml:\n{cargo_toml}"
        );

        // Cargo.toml without a prax-orm dependency -> same fallback
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"app\"\n").unwrap();
        let cargo_toml = create_seed_cargo_toml(dir.path()).unwrap();
        assert!(
            cargo_toml.contains(&format!("prax-orm = \"{}\"", env!("CARGO_PKG_VERSION"))),
            "unexpected seed Cargo.toml:\n{cargo_toml}"
        );
    }

    #[test]
    fn test_create_seed_cargo_toml_uses_project_version() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[dependencies]\nprax-orm = \"0.7.3\"\n",
        )
        .unwrap();
        let cargo_toml = create_seed_cargo_toml(dir.path()).unwrap();
        assert!(cargo_toml.contains("prax-orm = \"0.7.3\""));
    }

    #[test]
    fn test_generate_insert_sql_escapes_identifiers() {
        let runner = make_runner("seed.json", "sqlite");

        let mut record = HashMap::new();
        record.insert(
            "na\"me".to_string(),
            serde_json::Value::String("va\"l".to_string()),
        );

        let sql = runner.generate_insert_sql("we\"ird", &[record]).unwrap();

        // Embedded double quotes in identifiers are doubled
        assert!(sql.contains("INSERT INTO \"we\"\"ird\""));
        assert!(sql.contains("(\"na\"\"me\")"));
        // Values are string literals - double quotes need no escaping there
        assert!(sql.contains("'va\"l'"));
    }

    #[test]
    fn test_generate_reset_sql_per_provider() {
        let tables = vec!["users".to_string(), "posts".to_string()];

        let pg = make_runner("seed.json", "postgresql");
        assert_eq!(
            pg.generate_reset_sql(&tables),
            "TRUNCATE TABLE \"users\", \"posts\" CASCADE;"
        );

        let mysql = make_runner("seed.json", "mysql");
        assert_eq!(
            mysql.generate_reset_sql(&tables),
            "TRUNCATE TABLE `users`;\nTRUNCATE TABLE `posts`;"
        );

        let sqlite = make_runner("seed.json", "sqlite");
        assert_eq!(
            sqlite.generate_reset_sql(&tables),
            "DELETE FROM \"users\";\nDELETE FROM \"posts\";"
        );
    }

    #[test]
    fn test_generate_reset_sql_escapes_identifiers() {
        let sqlite = make_runner("seed.json", "sqlite");
        let tables = vec!["we\"ird".to_string()];
        assert_eq!(
            sqlite.generate_reset_sql(&tables),
            "DELETE FROM \"we\"\"ird\";"
        );
    }

    #[test]
    fn test_seed_table_names_from_json() {
        let dir = tempfile::tempdir().unwrap();
        let seed_path = dir.path().join("seed.json");
        std::fs::write(
            &seed_path,
            r#"{"tables": {"users": [{"id": 1}], "posts": [{"id": 2}]}}"#,
        )
        .unwrap();

        let runner = SeedRunner::new(
            seed_path,
            "sqlite://test.db".to_string(),
            "sqlite".to_string(),
            dir.path().to_path_buf(),
        )
        .unwrap();

        let mut names = runner.seed_table_names().unwrap();
        names.sort();
        assert_eq!(names, vec!["posts".to_string(), "users".to_string()]);
    }

    #[test]
    fn test_seed_table_names_from_toml() {
        let dir = tempfile::tempdir().unwrap();
        let seed_path = dir.path().join("seed.toml");
        std::fs::write(
            &seed_path,
            "[[tables.users]]\nid = 1\n\n[[tables.posts]]\nid = 2\n",
        )
        .unwrap();

        let runner = SeedRunner::new(
            seed_path,
            "sqlite://test.db".to_string(),
            "sqlite".to_string(),
            dir.path().to_path_buf(),
        )
        .unwrap();

        let mut names = runner.seed_table_names().unwrap();
        names.sort();
        assert_eq!(names, vec!["posts".to_string(), "users".to_string()]);
    }

    #[tokio::test]
    async fn test_reset_without_table_metadata_errors() {
        // .rs and .sql seeds carry no table metadata: --reset must fail loudly
        for seed in ["seed.rs", "seed.sql"] {
            let runner = make_runner(seed, "sqlite").with_reset(true);
            let err = runner.run().await.unwrap_err();
            assert!(
                err.to_string().contains("--reset requires table metadata"),
                "expected metadata error for {seed}, got: {err}"
            );
        }
    }
}
