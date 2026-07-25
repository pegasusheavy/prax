//! `prax migrate` commands - Database migration management.

use std::path::{Path, PathBuf};

use crate::cli::MigrateArgs;
use crate::commands::seed::{SeedRunner, find_seed_file, get_database_url};
use crate::config::{CONFIG_FILE_NAME, Config, MIGRATIONS_DIR, SCHEMA_FILE_PATH};
use crate::error::{CliError, CliResult};
use crate::output::{self, success, warn};

/// Run the migrate command
pub async fn run(args: MigrateArgs) -> CliResult<()> {
    match args.command {
        crate::cli::MigrateSubcommand::Dev(dev_args) => run_dev(dev_args).await,
        crate::cli::MigrateSubcommand::Deploy => run_deploy().await,
        crate::cli::MigrateSubcommand::Reset(reset_args) => run_reset(reset_args).await,
        crate::cli::MigrateSubcommand::Status => run_status().await,
        crate::cli::MigrateSubcommand::Resolve(resolve_args) => run_resolve(resolve_args).await,
        crate::cli::MigrateSubcommand::Diff(diff_args) => run_diff(diff_args).await,
        crate::cli::MigrateSubcommand::Rollback(rollback_args) => run_rollback(rollback_args).await,
        crate::cli::MigrateSubcommand::History(history_args) => run_history(history_args).await,
    }
}

/// Run `prax migrate dev` - development migration workflow
async fn run_dev(args: crate::cli::MigrateDevArgs) -> CliResult<()> {
    output::header("Migrate Dev");

    let cwd = std::env::current_dir()?;
    let config = load_config(&cwd)?;

    let migrations_dir = cwd.join(MIGRATIONS_DIR);

    let display_path = args
        .schema
        .as_deref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| SCHEMA_FILE_PATH.to_string());
    output::kv("Schema", &display_path);
    output::kv("Migrations", &migrations_dir.display().to_string());
    output::newline();

    // Determine total steps (5 or 6 depending on seed)
    let total_steps = if args.skip_seed { 5 } else { 6 };

    // 1. Parse and validate schema
    output::step(1, total_steps, "Parsing schema...");
    let loaded = crate::schema_loader::load_schema(args.schema.as_deref())?;
    let schema = loaded.schema;

    // 2. Check for pending migrations
    output::step(2, total_steps, "Checking migration status...");
    let pending = check_pending_migrations(&migrations_dir)?;

    if !pending.is_empty() {
        output::list(&format!("{} pending migrations found:", pending.len()));
        for migration in &pending {
            output::list_item(&migration.display().to_string());
        }
        output::newline();
    }

    // 3. Diff schema against database
    output::step(3, total_steps, "Comparing schema to database...");
    let migration_name = args
        .name
        .unwrap_or_else(|| format!("migration_{}", chrono::Utc::now().format("%Y%m%d%H%M%S")));

    // 4. Generate migration
    output::step(4, total_steps, "Generating migration...");
    let migration_path = create_migration(&migrations_dir, &migration_name, &schema)?;

    // 5. Apply migration (if not --create-only)
    if !args.create_only {
        output::step(5, total_steps, "Applying migration...");
        apply_migration(&migration_path, &config).await?;
    } else {
        output::step(5, total_steps, "Skipping apply (--create-only)...");
    }

    // 6. Run seed (if not --skip-seed)
    if !args.skip_seed && !args.create_only {
        output::step(6, total_steps, "Running seed...");

        if let Some(seed_path) = find_seed_file(&cwd, &config) {
            let database_url = get_database_url(&config)?;
            let runner = SeedRunner::new(
                seed_path,
                database_url,
                config.database.provider.clone(),
                cwd.clone(),
            )?;

            match runner.run().await {
                Ok(result) => {
                    output::list_item(&format!("Seeded {} records", result.records_affected));
                }
                Err(e) => {
                    output::warn(&format!("Seed failed: {}. Continuing...", e));
                }
            }
        } else {
            output::list_item("No seed file found, skipping");
        }
    }

    output::newline();
    success(&format!("Migration '{}' created", migration_name));

    output::newline();
    output::section("Next steps");
    output::list_item("Review the generated migration SQL");
    output::list_item("Run `prax generate` to update your client");

    Ok(())
}

/// Run `prax migrate deploy` - production deployment
async fn run_deploy() -> CliResult<()> {
    output::header("Migrate Deploy");

    let cwd = std::env::current_dir()?;
    let config = load_config(&cwd)?;
    let migrations_dir = cwd.join(MIGRATIONS_DIR);

    output::kv("Migrations", &migrations_dir.display().to_string());
    output::newline();

    // Check for pending migrations
    output::step(1, 3, "Checking for pending migrations...");
    let pending = check_pending_migrations(&migrations_dir)?;

    if pending.is_empty() {
        output::newline();
        success("No pending migrations to apply.");
        return Ok(());
    }

    output::list(&format!("{} pending migrations:", pending.len()));
    for migration in &pending {
        output::list_item(&migration.file_name().unwrap().to_string_lossy());
    }
    output::newline();

    // Apply migrations
    output::step(2, 3, "Applying migrations...");
    for migration in &pending {
        output::list_item(&format!(
            "Applying {}",
            migration.file_name().unwrap().to_string_lossy()
        ));
        apply_migration(migration, &config).await?;
    }

    // Verify
    output::step(3, 3, "Verifying migrations...");

    output::newline();
    success(&format!(
        "Applied {} migrations successfully!",
        pending.len()
    ));

    Ok(())
}

/// Run `prax migrate reset` - reset database
async fn run_reset(args: crate::cli::MigrateResetArgs) -> CliResult<()> {
    output::header("Migrate Reset");

    let cwd = std::env::current_dir()?;
    let _config = load_config(&cwd)?;

    if !args.force {
        warn("This will delete all data in the database!");
        output::newline();
        if !output::confirm("Are you sure you want to reset the database?") {
            output::newline();
            output::info("Reset cancelled.");
            return Ok(());
        }
    }

    output::newline();

    // Honest failure: drop/create database and re-applying migrations require a
    // database executor that is not yet wired into the CLI. No changes are made.
    Err(CliError::Migration(
        "migrate reset is not yet implemented: dropping and recreating the database \
         requires a database executor that is not yet wired into the CLI. No changes \
         were made to the database."
            .to_string(),
    ))
}

/// Run `prax migrate status` - show migration status
async fn run_status() -> CliResult<()> {
    output::header("Migration Status");

    let cwd = std::env::current_dir()?;
    let _config = load_config(&cwd)?;
    let migrations_dir = cwd.join(MIGRATIONS_DIR);

    // List all migrations
    let mut migrations = Vec::new();
    if migrations_dir.exists() {
        for entry in std::fs::read_dir(&migrations_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                migrations.push(path);
            }
        }
    }
    migrations.sort();

    if migrations.is_empty() {
        output::info("No migrations found.");
        output::newline();
        output::section("Getting started");
        output::list_item("Run `prax migrate dev` to create your first migration");
        return Ok(());
    }

    output::section("Migrations");

    for (i, migration) in migrations.iter().enumerate() {
        let name = migration.file_name().unwrap().to_string_lossy();
        let applied = is_migration_applied(migration)?;

        let status = if applied {
            output::style_success("✓ Applied")
        } else {
            output::style_pending("○ Pending")
        };

        output::numbered_item(i + 1, &format!("{} - {}", name, status));
    }

    output::newline();

    let applied_count = migrations
        .iter()
        .filter(|m| is_migration_applied(m).unwrap_or(false))
        .count();
    let pending_count = migrations.len() - applied_count;

    output::kv("Total", &migrations.len().to_string());
    output::kv("Applied", &applied_count.to_string());
    output::kv("Pending", &pending_count.to_string());

    Ok(())
}

/// Run `prax migrate resolve` - resolve migration issues
async fn run_resolve(args: crate::cli::MigrateResolveArgs) -> CliResult<()> {
    output::header("Migrate Resolve");

    if !args.applied && !args.rolled_back {
        return Err(CliError::Command(
            "Must specify --applied or --rolled-back".to_string(),
        ));
    }

    // Honest failure: resolving requires writing to the migration history table,
    // which is not yet wired into the CLI. No changes are made.
    Err(CliError::Migration(format!(
        "migrate resolve is not yet implemented: marking migration '{}' as {} \
         requires updating the _prax_migrations history table, which is not yet \
         wired into the CLI. No changes were made.",
        args.migration,
        if args.applied {
            "applied"
        } else {
            "rolled back"
        }
    )))
}

/// Run `prax migrate diff` - generate schema DDL without applying
async fn run_diff(args: crate::cli::MigrateDiffArgs) -> CliResult<()> {
    output::header("Migrate Diff");

    let _cwd = std::env::current_dir()?;

    // Diffing against a stored migration requires database introspection and a
    // migration snapshot store, neither of which is wired into the CLI.
    if let Some(from_migration) = &args.from_migration {
        return Err(CliError::Migration(format!(
            "--from-migration '{}' is not supported: diffing against a specific \
             migration requires database introspection and a migration snapshot \
             store, which are not yet wired into the CLI.",
            from_migration
        )));
    }

    // Parse schema
    output::step(1, 2, "Parsing schema...");
    let loaded = crate::schema_loader::load_schema(args.schema.as_deref())?;
    let schema = loaded.schema;

    // Generate DDL. Note: this is NOT a database diff — database introspection
    // is not yet implemented, so nothing is compared against live state. The
    // output is PostgreSQL-flavored DDL for the full schema.
    output::step(2, 2, "Generating schema DDL (PostgreSQL dialect)...");
    let ddl_sql = generate_schema_diff(&schema)?;

    output::newline();
    output::info(
        "This generates PostgreSQL-flavored DDL for the entire schema; it is not a \
         diff against database state (database introspection is not yet implemented).",
    );

    output::newline();
    output::section("Generated DDL");
    output::code(&ddl_sql, "sql");

    if let Some(output_path) = args.output {
        std::fs::write(&output_path, &ddl_sql)?;
        output::newline();
        success(&format!("DDL written to {}", output_path.display()));
    }

    Ok(())
}

/// Run `prax migrate rollback` - rollback the last applied migration
async fn run_rollback(args: crate::cli::MigrateRollbackArgs) -> CliResult<()> {
    output::header("Migrate Rollback");

    output::newline();

    if let Some(to_migration) = &args.to {
        output::info(&format!("Rolling back to migration: {}", to_migration));
    } else {
        output::info("Rolling back last applied migration...");
    }

    if let Some(reason) = &args.reason {
        output::kv("Reason", reason);
    }

    if let Some(user) = &args.user {
        output::kv("User", user);
    }

    output::newline();

    // TODO: Implement actual rollback logic using event sourcing
    // The real implementation would:
    // 1. Load the event store
    // 2. Find the last applied migration (or specified migration)
    // 3. Append a RolledBack event
    // 4. Execute the down migration SQL
    // 5. Update migration state

    // Honest failure: rollback requires the prax-migrate event-sourcing engine,
    // which is not yet wired into the CLI. Exit non-zero — no changes are made.
    Err(CliError::Migration(
        "migrate rollback is not yet implemented: rolling back requires the \
         prax-migrate event-sourcing engine (event store and down-migration \
         execution), which is not yet wired into the CLI. No changes were made."
            .to_string(),
    ))
}

/// Run `prax migrate history` - view migration history
async fn run_history(args: crate::cli::MigrateHistoryArgs) -> CliResult<()> {
    output::header("Migration History");

    output::newline();

    if let Some(migration) = &args.migration {
        output::section(&format!("History for migration: {}", migration));
    } else {
        output::section("All migrations");
    }

    output::newline();

    // TODO: Implement actual history viewing using event sourcing
    // The real implementation would:
    // 1. Load the event store
    // 2. Query events for the specified migration (or all)
    // 3. Display events in chronological order
    // 4. Show event type, timestamp, and event-specific data

    // Honest failure: history requires reading the _prax_migrations event log,
    // which is not yet wired into the CLI. Exit non-zero.
    Err(CliError::Migration(
        "migrate history is not yet implemented: viewing history requires reading \
         the _prax_migrations event log, which is not yet wired into the CLI."
            .to_string(),
    ))
}

// =============================================================================
// Helper Functions
// =============================================================================

fn load_config(cwd: &Path) -> CliResult<Config> {
    let config_path = cwd.join(CONFIG_FILE_NAME);
    if config_path.exists() {
        Config::load(&config_path)
    } else {
        Ok(Config::default())
    }
}

fn check_pending_migrations(migrations_dir: &Path) -> CliResult<Vec<PathBuf>> {
    let mut pending = Vec::new();

    if !migrations_dir.exists() {
        return Ok(pending);
    }

    for entry in std::fs::read_dir(migrations_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && !is_migration_applied(&path)? {
            pending.push(path);
        }
    }

    pending.sort();
    Ok(pending)
}

fn is_migration_applied(migration_path: &Path) -> CliResult<bool> {
    // Check for a marker file indicating the migration has been applied
    // In production, this would check the migration history table
    let marker = migration_path.join(".applied");
    Ok(marker.exists())
}

fn create_migration(
    migrations_dir: &Path,
    name: &str,
    schema: &prax_schema::ast::Schema,
) -> CliResult<PathBuf> {
    // Create migration directory
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let migration_name = format!("{}_{}", timestamp, name);
    let migration_path = migrations_dir.join(&migration_name);

    std::fs::create_dir_all(&migration_path)?;

    // Generate migration SQL
    let sql = generate_schema_diff(schema)?;

    // Write migration.sql
    let sql_path = migration_path.join("migration.sql");
    std::fs::write(&sql_path, &sql)?;

    Ok(migration_path)
}

fn generate_schema_diff(schema: &prax_schema::ast::Schema) -> CliResult<String> {
    use prax_schema::ast::{FieldType, ScalarType};

    let mut sql = String::new();

    sql.push_str("-- Migration generated by Prax\n\n");

    // Generate enums FIRST (before tables that reference them)
    if !schema.enums.is_empty() {
        sql.push_str("-- Enum types\n");
        for enum_def in schema.enums.values() {
            let enum_name = enum_def
                .attributes
                .iter()
                .find(|a| a.is("map"))
                .and_then(|a: &prax_schema::ast::Attribute| a.first_arg())
                .and_then(|v: &prax_schema::ast::AttributeValue| v.as_string())
                .map(|s| s.to_string())
                .unwrap_or_else(|| to_snake_case(enum_def.name()));

            sql.push_str(&format!(
                "DO $$ BEGIN\n    CREATE TYPE \"{}\" AS ENUM (",
                enum_name
            ));

            let variants: Vec<String> = enum_def
                .variants
                .iter()
                .map(|v| format!("'{}'", v.name()))
                .collect();

            sql.push_str(&variants.join(", "));
            sql.push_str(");\nEXCEPTION\n    WHEN duplicate_object THEN null;\nEND $$;\n\n");
        }
        sql.push('\n');
    }

    // Generate CREATE TABLE statements for each model
    sql.push_str("-- Tables\n");
    for model in schema.models.values() {
        let table_name = model.table_name();

        sql.push_str(&format!(
            "CREATE TABLE IF NOT EXISTS \"{}\" (\n",
            table_name
        ));

        let mut columns = Vec::new();
        let mut primary_keys = Vec::new();

        for field in model.fields.values() {
            if field.is_relation() {
                continue;
            }

            let column_name = field
                .get_attribute("map")
                .and_then(|a| a.first_arg())
                .and_then(|v| v.as_string())
                .map(|s| s.to_string())
                .unwrap_or_else(|| to_snake_case(field.name()));

            let sql_type = field_type_to_sql(&field.field_type);
            let mut column_def = format!("    \"{}\" {}", column_name, sql_type);

            // Add constraints
            if field.is_id() {
                primary_keys.push(column_name.clone());
            }

            if field.has_attribute("auto") || field.has_attribute("autoincrement") {
                // PostgreSQL uses SERIAL types
                column_def = format!("    \"{}\" SERIAL", column_name);
            }

            if field.has_attribute("unique") {
                column_def.push_str(" UNIQUE");
            }

            if !field.is_optional() && !field.is_id() {
                column_def.push_str(" NOT NULL");
            }

            // Default values
            if let Some(default_attr) = field.get_attribute("default")
                && let Some(value) = default_attr.first_arg()
            {
                let value_str = format_attribute_value(value);
                column_def.push_str(&format!(
                    " DEFAULT {}",
                    sql_default_value(&value_str, &field.field_type)
                ));
            }

            columns.push(column_def);
        }

        sql.push_str(&columns.join(",\n"));

        if !primary_keys.is_empty() {
            sql.push_str(",\n");
            sql.push_str(&format!(
                "    PRIMARY KEY (\"{}\")",
                primary_keys.join("\", \"")
            ));
        }

        sql.push_str("\n);\n\n");
    }

    return Ok(sql);

    fn field_type_to_sql(field_type: &FieldType) -> String {
        match field_type {
            FieldType::Scalar(scalar) => match scalar {
                ScalarType::Int => "INTEGER".to_string(),
                ScalarType::BigInt => "BIGINT".to_string(),
                ScalarType::Float => "DOUBLE PRECISION".to_string(),
                ScalarType::String => "TEXT".to_string(),
                ScalarType::Boolean => "BOOLEAN".to_string(),
                ScalarType::DateTime => "TIMESTAMP WITH TIME ZONE".to_string(),
                ScalarType::Date => "DATE".to_string(),
                ScalarType::Time => "TIME".to_string(),
                ScalarType::Json => "JSONB".to_string(),
                ScalarType::Bytes => "BYTEA".to_string(),
                ScalarType::Decimal => "DECIMAL".to_string(),
                ScalarType::Uuid => "UUID".to_string(),
                ScalarType::Cuid | ScalarType::Cuid2 | ScalarType::NanoId | ScalarType::Ulid => {
                    "TEXT".to_string()
                }
                ScalarType::Vector(dim) => match dim {
                    Some(d) => format!("vector({})", d),
                    None => "vector".to_string(),
                },
                ScalarType::HalfVector(dim) => match dim {
                    Some(d) => format!("halfvec({})", d),
                    None => "halfvec".to_string(),
                },
                ScalarType::SparseVector(dim) => match dim {
                    Some(d) => format!("sparsevec({})", d),
                    None => "sparsevec".to_string(),
                },
                ScalarType::Bit(dim) => match dim {
                    Some(d) => format!("bit({})", d),
                    None => "bit".to_string(),
                },
            },
            FieldType::Enum(name) => format!("\"{}\"", to_snake_case(name)),
            _ => "TEXT".to_string(),
        }
    }
}

async fn apply_migration(migration_path: &Path, _config: &Config) -> CliResult<()> {
    let sql_path = migration_path.join("migration.sql");

    if !sql_path.exists() {
        return Err(CliError::Migration(format!(
            "Migration file not found: {}",
            sql_path.display()
        )));
    }

    // Honest failure: applying migrations requires a database executor (driver /
    // prax-migrate engine) that is not yet wired into the CLI. Do NOT write the
    // `.applied` marker or report success for work that was not performed.
    Err(CliError::Migration(format!(
        "Applying migration '{}' is not yet implemented: executing migration SQL \
         requires a database executor that is not yet wired into the CLI. The \
         migration SQL is at {}; apply it with an external tool for now.",
        migration_path.display(),
        sql_path.display()
    )))
}

fn sql_default_value(value: &str, field_type: &prax_schema::ast::FieldType) -> String {
    use prax_schema::ast::{FieldType, ScalarType};

    // Handle enum defaults - need to be quoted as strings
    if matches!(field_type, FieldType::Enum(_)) {
        return format!("'{}'", value);
    }

    match value.to_lowercase().as_str() {
        "now()" => "CURRENT_TIMESTAMP".to_string(),
        "uuid()" => "gen_random_uuid()".to_string(),
        "cuid()" | "cuid2()" | "nanoid()" | "ulid()" => {
            // These need application-level generation
            "''".to_string()
        }
        "true" => "TRUE".to_string(),
        "false" => "FALSE".to_string(),
        _ => {
            // String-typed defaults arrive double-quoted from
            // `format_attribute_value`. In SQL, double quotes denote an
            // identifier, so re-quote as a string literal with single quotes
            // and escape embedded single quotes (' -> '').
            let is_string_typed = matches!(
                field_type,
                FieldType::Scalar(
                    ScalarType::String
                        | ScalarType::Cuid
                        | ScalarType::Cuid2
                        | ScalarType::NanoId
                        | ScalarType::Ulid
                )
            );

            if is_string_typed {
                let inner = value
                    .strip_prefix('"')
                    .and_then(|v| v.strip_suffix('"'))
                    .unwrap_or(value);
                format!("'{}'", inner.replace('\'', "''"))
            } else {
                value.to_string()
            }
        }
    }
}

fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

fn format_attribute_value(value: &prax_schema::ast::AttributeValue) -> String {
    use prax_schema::ast::AttributeValue;

    match value {
        AttributeValue::String(s) => format!("\"{}\"", s),
        AttributeValue::Int(i) => i.to_string(),
        AttributeValue::Float(f) => f.to_string(),
        AttributeValue::Boolean(b) => b.to_string(),
        AttributeValue::Ident(id) => id.to_string(),
        AttributeValue::Function(name, args) => {
            if args.is_empty() {
                format!("{}()", name)
            } else {
                let arg_strs: Vec<String> = args.iter().map(format_attribute_value).collect();
                format!("{}({})", name, arg_strs.join(", "))
            }
        }
        AttributeValue::Array(items) => {
            let item_strs: Vec<String> = items.iter().map(format_attribute_value).collect();
            format!("[{}]", item_strs.join(", "))
        }
        AttributeValue::FieldRef(field) => field.to_string(),
        AttributeValue::FieldRefList(fields) => {
            format!(
                "[{}]",
                fields
                    .iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_schema::ast::{FieldType, ScalarType};

    // -- sql_default_value --------------------------------------------------

    #[test]
    fn test_sql_default_value_string_single_quoted() {
        // format_attribute_value emits string defaults double-quoted; SQL string
        // literals must be single-quoted (double quotes denote an identifier).
        let ty = FieldType::Scalar(ScalarType::String);
        assert_eq!(sql_default_value("\"active\"", &ty), "'active'");
    }

    #[test]
    fn test_sql_default_value_string_escapes_quotes() {
        let ty = FieldType::Scalar(ScalarType::String);
        assert_eq!(sql_default_value("\"it's\"", &ty), "'it''s'");
    }

    #[test]
    fn test_sql_default_value_non_string_passthrough() {
        let ty = FieldType::Scalar(ScalarType::Int);
        assert_eq!(sql_default_value("42", &ty), "42");
    }

    #[test]
    fn test_sql_default_value_enum_single_quoted() {
        let ty = FieldType::Enum("Role".into());
        assert_eq!(sql_default_value("ADMIN", &ty), "'ADMIN'");
    }

    #[test]
    fn test_sql_default_value_function_and_boolean_defaults() {
        let ty = FieldType::Scalar(ScalarType::DateTime);
        assert_eq!(sql_default_value("now()", &ty), "CURRENT_TIMESTAMP");

        let ty = FieldType::Scalar(ScalarType::Boolean);
        assert_eq!(sql_default_value("true", &ty), "TRUE");
        assert_eq!(sql_default_value("false", &ty), "FALSE");
    }

    // -- honest-error paths ---------------------------------------------------

    #[tokio::test]
    async fn test_apply_migration_fails_without_executor() {
        let dir = tempfile::tempdir().unwrap();
        let migration_path = dir.path().join("20240101000000_init");
        std::fs::create_dir_all(&migration_path).unwrap();
        std::fs::write(
            migration_path.join("migration.sql"),
            "CREATE TABLE t (id INT);",
        )
        .unwrap();

        let result = apply_migration(&migration_path, &Config::default()).await;

        match result {
            Err(CliError::Migration(msg)) => {
                assert!(
                    msg.contains("not yet implemented"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected CliError::Migration, got {other:?}"),
        }

        // The .applied marker must NOT be written for work that was not done.
        assert!(!migration_path.join(".applied").exists());
    }

    #[tokio::test]
    async fn test_apply_migration_missing_file() {
        let dir = tempfile::tempdir().unwrap();

        match apply_migration(dir.path(), &Config::default()).await {
            Err(CliError::Migration(msg)) => assert!(msg.contains("not found")),
            other => panic!("expected CliError::Migration, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_run_reset_not_implemented() {
        let args = crate::cli::MigrateResetArgs {
            force: true,
            seed: false,
            skip_migrations: false,
        };

        match run_reset(args).await {
            Err(CliError::Migration(msg)) => {
                assert!(
                    msg.contains("not yet implemented"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected CliError::Migration, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_run_resolve_not_implemented() {
        let args = crate::cli::MigrateResolveArgs {
            migration: "20240101000000_init".to_string(),
            applied: true,
            rolled_back: false,
        };

        match run_resolve(args).await {
            Err(CliError::Migration(msg)) => {
                assert!(
                    msg.contains("not yet implemented"),
                    "unexpected message: {msg}"
                );
                assert!(msg.contains("20240101000000_init"));
            }
            other => panic!("expected CliError::Migration, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_run_resolve_requires_a_flag() {
        let args = crate::cli::MigrateResolveArgs {
            migration: "m".to_string(),
            applied: false,
            rolled_back: false,
        };

        match run_resolve(args).await {
            Err(CliError::Command(msg)) => assert!(msg.contains("--applied or --rolled-back")),
            other => panic!("expected CliError::Command, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_run_rollback_not_implemented() {
        let args = crate::cli::MigrateRollbackArgs {
            reason: None,
            user: None,
            to: None,
        };

        match run_rollback(args).await {
            Err(CliError::Migration(msg)) => {
                assert!(
                    msg.contains("not yet implemented"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected CliError::Migration, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_run_history_not_implemented() {
        let args = crate::cli::MigrateHistoryArgs { migration: None };

        match run_history(args).await {
            Err(CliError::Migration(msg)) => {
                assert!(
                    msg.contains("_prax_migrations"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected CliError::Migration, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_run_diff_from_migration_unsupported() {
        let args = crate::cli::MigrateDiffArgs {
            schema: None,
            output: None,
            from_migration: Some("20240101000000_init".to_string()),
        };

        match run_diff(args).await {
            Err(CliError::Migration(msg)) => {
                assert!(
                    msg.contains("--from-migration"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected CliError::Migration, got {other:?}"),
        }
    }
}
