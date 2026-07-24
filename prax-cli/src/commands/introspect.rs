//! Database introspection implementation.
//!
//! This module provides the actual database introspection functionality
//! using the `prax-query` introspection types.

use std::collections::HashMap;

use prax_query::introspection::{
    ColumnInfo, DatabaseSchema, EnumInfo, ForeignKeyInfo, IndexColumn, IndexInfo,
    ReferentialAction, SortOrder, TableInfo, ViewInfo, generate_prax_schema, normalize_type,
    queries,
};
use prax_query::sql::DatabaseType;

use crate::config::Config;
use crate::error::{CliError, CliResult};

/// Introspection options.
#[derive(Debug, Clone)]
pub struct IntrospectionOptions {
    /// Schema/namespace to introspect.
    pub schema: Option<String>,
    /// Include views.
    pub include_views: bool,
    /// Include materialized views.
    pub include_materialized_views: bool,
    /// Table filter pattern.
    pub table_filter: Option<String>,
    /// Tables to exclude.
    pub exclude_pattern: Option<String>,
    /// Include comments.
    pub include_comments: bool,
    /// Sample size for MongoDB.
    pub sample_size: usize,
}

impl Default for IntrospectionOptions {
    fn default() -> Self {
        Self {
            schema: None,
            include_views: false,
            include_materialized_views: false,
            table_filter: None,
            exclude_pattern: None,
            include_comments: true,
            sample_size: 100,
        }
    }
}

/// Database introspector trait.
#[allow(async_fn_in_trait)]
pub trait Introspector {
    /// Introspect the database and return schema information.
    async fn introspect(&self, options: &IntrospectionOptions) -> CliResult<DatabaseSchema>;
}

/// Get the database type from provider string.
pub fn get_database_type(provider: &str) -> CliResult<DatabaseType> {
    match provider.to_lowercase().as_str() {
        "postgresql" | "postgres" | "pg" => Ok(DatabaseType::PostgreSQL),
        "mysql" | "mariadb" => Ok(DatabaseType::MySQL),
        "sqlite" | "sqlite3" => Ok(DatabaseType::SQLite),
        "mssql" | "sqlserver" | "sql_server" => Ok(DatabaseType::MSSQL),
        _ => Err(CliError::Config(format!(
            "Unsupported database provider: {}",
            provider
        ))),
    }
}

/// Get default schema for database type.
pub fn default_schema(db_type: DatabaseType) -> &'static str {
    match db_type {
        DatabaseType::PostgreSQL => "public",
        DatabaseType::MySQL => "",
        DatabaseType::SQLite => "",
        DatabaseType::MSSQL => "dbo",
    }
}

// ============================================================================
// PostgreSQL Introspector
// ============================================================================

#[cfg(feature = "postgres")]
pub mod postgres {
    use super::*;
    use tokio_postgres::{Client, NoTls, Row};

    /// PostgreSQL introspector.
    pub struct PostgresIntrospector {
        connection_string: String,
    }

    impl PostgresIntrospector {
        /// Create a new PostgreSQL introspector.
        pub fn new(connection_string: String) -> Self {
            Self { connection_string }
        }

        /// Connect to the database.
        async fn connect(&self) -> CliResult<Client> {
            // Parse the DSN the same way tokio-postgres will, so the sslmode
            // it carries is honored: anything but `disable` goes through the
            // workspace's shared rustls connector (chain + hostname verified
            // against the Mozilla root store). `prefer` still falls back to
            // plaintext when the server declines TLS.
            let config = self
                .connection_string
                .parse::<tokio_postgres::Config>()
                .map_err(|e| CliError::Config(format!("Invalid connection string: {}", e)))?;

            let tls_disabled = matches!(
                config.get_ssl_mode(),
                tokio_postgres::config::SslMode::Disable
            );

            if tls_disabled && !config.get_hosts().iter().all(is_local_host) {
                crate::output::warn(
                    "sslmode=disable with a non-local host: credentials and data will be \
                     sent in plaintext.",
                );
            }

            // The two connector types produce different `Connection`
            // generics, so drive each arm independently and unify on the
            // stream-agnostic `Client`.
            let client = if tls_disabled {
                let (client, connection) = tokio_postgres::connect(&self.connection_string, NoTls)
                    .await
                    .map_err(|e| CliError::Database(format!("Failed to connect: {}", e)))?;
                tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        eprintln!("Connection error: {}", e);
                    }
                });
                client
            } else {
                let (client, connection) = tokio_postgres::connect(
                    &self.connection_string,
                    prax_postgres::tls::make_tls_connector(),
                )
                .await
                .map_err(|e| CliError::Database(format!("Failed to connect: {}", e)))?;
                tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        eprintln!("Connection error: {}", e);
                    }
                });
                client
            };

            Ok(client)
        }
    }

    impl Introspector for PostgresIntrospector {
        async fn introspect(&self, options: &IntrospectionOptions) -> CliResult<DatabaseSchema> {
            let client = self.connect().await?;
            let schema_name = options.schema.as_deref().unwrap_or("public");

            let mut db_schema = DatabaseSchema {
                name: "database".to_string(),
                schema: Some(schema_name.to_string()),
                ..Default::default()
            };

            // Get tables
            let tables_sql = queries::tables_query(DatabaseType::PostgreSQL, Some(schema_name));
            let table_rows = client
                .query(&tables_sql, &[])
                .await
                .map_err(|e| CliError::Database(format!("Failed to query tables: {}", e)))?;

            for row in table_rows {
                let table_name: String = row.get(0);

                // Apply filters
                if let Some(ref pattern) = options.table_filter
                    && !matches_pattern(&table_name, pattern)
                {
                    continue;
                }
                if let Some(ref exclude) = options.exclude_pattern
                    && matches_pattern(&table_name, exclude)
                {
                    continue;
                }

                let comment: Option<String> = row.try_get(1).ok();

                db_schema.tables.push(TableInfo {
                    name: table_name,
                    schema: Some(schema_name.to_string()),
                    comment: if options.include_comments {
                        comment
                    } else {
                        None
                    },
                    ..Default::default()
                });
            }

            // Fetch columns, primary keys, foreign keys, and indexes for the
            // whole schema in one query each, then group rows by table in
            // memory: 4 round-trips total instead of 4 per table. The table
            // name is appended as the last selected column and used as the
            // first ORDER BY key so grouped rows keep the exact per-table
            // ordering of the original per-table queries.
            let cols_sql = "SELECT \
                    c.column_name, \
                    c.data_type, \
                    c.udt_name, \
                    c.is_nullable = 'YES' as nullable, \
                    c.column_default, \
                    c.character_maximum_length, \
                    c.numeric_precision, \
                    c.numeric_scale, \
                    col_description((quote_ident(c.table_schema) || '.' || quote_ident(c.table_name))::regclass, c.ordinal_position) as comment, \
                    CASE WHEN c.column_default LIKE 'nextval%' THEN true ELSE false END as auto_increment, \
                    c.table_name \
                 FROM information_schema.columns c \
                 WHERE c.table_schema = $1 \
                 ORDER BY c.table_name, c.ordinal_position";
            let col_rows = client
                .query(cols_sql, &[&schema_name])
                .await
                .map_err(|e| CliError::Database(format!("Failed to query columns: {}", e)))?;

            let mut columns_by_table: HashMap<String, Vec<Row>> = HashMap::new();
            for col_row in col_rows {
                let table_name: String = col_row.get(10);
                columns_by_table
                    .entry(table_name)
                    .or_default()
                    .push(col_row);
            }

            let pk_sql = "SELECT a.attname as column_name, c.relname as table_name \
                 FROM pg_index i \
                 JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey) \
                 JOIN pg_class c ON c.oid = i.indrelid \
                 JOIN pg_namespace n ON n.oid = c.relnamespace \
                 WHERE i.indisprimary AND n.nspname = $1 \
                 ORDER BY c.relname, array_position(i.indkey, a.attnum)";
            let pk_rows = client
                .query(pk_sql, &[&schema_name])
                .await
                .map_err(|e| CliError::Database(format!("Failed to query primary keys: {}", e)))?;

            let mut pks_by_table: HashMap<String, Vec<Row>> = HashMap::new();
            for pk_row in pk_rows {
                let table_name: String = pk_row.get(1);
                pks_by_table.entry(table_name).or_default().push(pk_row);
            }

            let fk_sql = "SELECT \
                    tc.constraint_name, \
                    kcu.column_name, \
                    ccu.table_name as referenced_table, \
                    ccu.table_schema as referenced_schema, \
                    ccu.column_name as referenced_column, \
                    rc.delete_rule, \
                    rc.update_rule, \
                    tc.table_name \
                 FROM information_schema.table_constraints tc \
                 JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name \
                 JOIN information_schema.constraint_column_usage ccu ON ccu.constraint_name = tc.constraint_name \
                 JOIN information_schema.referential_constraints rc ON rc.constraint_name = tc.constraint_name \
                 WHERE tc.constraint_type = 'FOREIGN KEY' AND tc.table_schema = $1 \
                 ORDER BY tc.table_name, tc.constraint_name, kcu.ordinal_position";
            let fk_rows = client
                .query(fk_sql, &[&schema_name])
                .await
                .map_err(|e| CliError::Database(format!("Failed to query foreign keys: {}", e)))?;

            let mut fks_by_table: HashMap<String, Vec<Row>> = HashMap::new();
            for fk_row in fk_rows {
                let table_name: String = fk_row.get(7);
                fks_by_table.entry(table_name).or_default().push(fk_row);
            }

            let idx_sql = "SELECT \
                    i.relname as index_name, \
                    a.attname as column_name, \
                    ix.indisunique as is_unique, \
                    ix.indisprimary as is_primary, \
                    am.amname as index_type, \
                    pg_get_expr(ix.indpred, ix.indrelid) as filter, \
                    t.relname as table_name \
                 FROM pg_index ix \
                 JOIN pg_class t ON t.oid = ix.indrelid \
                 JOIN pg_class i ON i.oid = ix.indexrelid \
                 JOIN pg_namespace n ON n.oid = t.relnamespace \
                 JOIN pg_am am ON i.relam = am.oid \
                 JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey) \
                 WHERE n.nspname = $1 \
                 ORDER BY t.relname, i.relname, array_position(ix.indkey, a.attnum)";
            let idx_rows = client
                .query(idx_sql, &[&schema_name])
                .await
                .map_err(|e| CliError::Database(format!("Failed to query indexes: {}", e)))?;

            let mut indexes_by_table: HashMap<String, Vec<Row>> = HashMap::new();
            for idx_row in idx_rows {
                let table_name: String = idx_row.get(6);
                indexes_by_table
                    .entry(table_name)
                    .or_default()
                    .push(idx_row);
            }

            // Populate each table from the pre-fetched rows.
            for table in &mut db_schema.tables {
                for col_row in columns_by_table.remove(&table.name).unwrap_or_default() {
                    let col_name: String = col_row.get(0);
                    let data_type: String = col_row.get(1);
                    let udt_name: String = col_row.get(2);
                    let nullable: bool = col_row.get(3);
                    let default: Option<String> = col_row.try_get(4).ok();
                    let max_length: Option<i32> = col_row.try_get(5).ok();
                    let precision: Option<i32> = col_row.try_get(6).ok();
                    let scale: Option<i32> = col_row.try_get(7).ok();
                    let comment: Option<String> = col_row.try_get(8).ok();
                    let auto_increment: bool = col_row.try_get(9).unwrap_or(false);

                    let normalized = normalize_type(
                        DatabaseType::PostgreSQL,
                        &udt_name,
                        max_length,
                        precision,
                        scale,
                    );

                    table.columns.push(ColumnInfo {
                        name: col_name,
                        db_type: data_type,
                        normalized_type: normalized,
                        nullable,
                        default,
                        auto_increment,
                        max_length,
                        precision,
                        scale,
                        comment: if options.include_comments {
                            comment
                        } else {
                            None
                        },
                        ..Default::default()
                    });
                }

                for pk_row in pks_by_table.remove(&table.name).unwrap_or_default() {
                    let col_name: String = pk_row.get(0);
                    table.primary_key.push(col_name.clone());

                    // Mark column as primary key
                    if let Some(col) = table.columns.iter_mut().find(|c| c.name == col_name) {
                        col.is_primary_key = true;
                    }
                }

                let mut fk_map: HashMap<String, ForeignKeyInfo> = HashMap::new();
                for fk_row in fks_by_table.remove(&table.name).unwrap_or_default() {
                    let constraint_name: String = fk_row.get(0);
                    let column_name: String = fk_row.get(1);
                    let ref_table: String = fk_row.get(2);
                    let ref_schema: Option<String> = fk_row.try_get(3).ok();
                    let ref_column: String = fk_row.get(4);
                    let delete_rule: String = fk_row.get(5);
                    let update_rule: String = fk_row.get(6);

                    let fk =
                        fk_map
                            .entry(constraint_name.clone())
                            .or_insert_with(|| ForeignKeyInfo {
                                name: constraint_name,
                                columns: Vec::new(),
                                referenced_table: ref_table,
                                referenced_schema: ref_schema,
                                referenced_columns: Vec::new(),
                                on_delete: ReferentialAction::from_str(&delete_rule),
                                on_update: ReferentialAction::from_str(&update_rule),
                            });

                    fk.columns.push(column_name);
                    fk.referenced_columns.push(ref_column);
                }

                table.foreign_keys = fk_map.into_values().collect();

                let mut idx_map: HashMap<String, IndexInfo> = HashMap::new();
                for idx_row in indexes_by_table.remove(&table.name).unwrap_or_default() {
                    let idx_name: String = idx_row.get(0);
                    let col_name: String = idx_row.get(1);
                    let is_unique: bool = idx_row.get(2);
                    let is_primary: bool = idx_row.get(3);
                    let idx_type: Option<String> = idx_row.try_get(4).ok();
                    let filter: Option<String> = idx_row.try_get(5).ok();

                    let idx = idx_map
                        .entry(idx_name.clone())
                        .or_insert_with(|| IndexInfo {
                            name: idx_name,
                            columns: Vec::new(),
                            is_unique,
                            is_primary,
                            index_type: idx_type,
                            filter,
                        });

                    idx.columns.push(IndexColumn {
                        name: col_name,
                        order: SortOrder::Asc,
                        ..Default::default()
                    });
                }

                table.indexes = idx_map.into_values().collect();
            }

            // Get enums
            let enums_sql = queries::enums_query(Some(schema_name));
            let enum_rows = client
                .query(&enums_sql, &[])
                .await
                .map_err(|e| CliError::Database(format!("Failed to query enums: {}", e)))?;

            let mut enum_map: HashMap<String, EnumInfo> = HashMap::new();
            for enum_row in enum_rows {
                let enum_name: String = enum_row.get(0);
                let enum_value: String = enum_row.get(1);

                let enum_info = enum_map
                    .entry(enum_name.clone())
                    .or_insert_with(|| EnumInfo {
                        name: enum_name,
                        schema: Some(schema_name.to_string()),
                        values: Vec::new(),
                    });

                enum_info.values.push(enum_value);
            }

            db_schema.enums = enum_map.into_values().collect();

            // Get views
            if options.include_views || options.include_materialized_views {
                let views_sql = queries::views_query(DatabaseType::PostgreSQL, Some(schema_name));
                let view_rows = client
                    .query(&views_sql, &[])
                    .await
                    .map_err(|e| CliError::Database(format!("Failed to query views: {}", e)))?;

                for view_row in view_rows {
                    let view_name: String = view_row.get(0);
                    let definition: Option<String> = view_row.try_get(1).ok();
                    let is_materialized: bool = view_row.get(2);

                    if is_materialized && !options.include_materialized_views {
                        continue;
                    }
                    if !is_materialized && !options.include_views {
                        continue;
                    }

                    db_schema.views.push(ViewInfo {
                        name: view_name,
                        schema: Some(schema_name.to_string()),
                        definition,
                        is_materialized,
                        columns: Vec::new(),
                    });
                }
            }

            Ok(db_schema)
        }
    }

    /// Whether a parsed DSN host is local (loopback TCP or a Unix socket).
    fn is_local_host(host: &tokio_postgres::config::Host) -> bool {
        match host {
            tokio_postgres::config::Host::Tcp(name) => {
                name == "localhost" || name == "127.0.0.1" || name == "::1"
            }
            tokio_postgres::config::Host::Unix(_) => true,
        }
    }

    /// Simple glob-style pattern matching.
    fn matches_pattern(name: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if pattern.starts_with('*') && pattern.ends_with('*') {
            let middle = &pattern[1..pattern.len() - 1];
            return name.contains(middle);
        }

        if let Some(suffix) = pattern.strip_prefix('*') {
            return name.ends_with(suffix);
        }

        if let Some(prefix) = pattern.strip_suffix('*') {
            return name.starts_with(prefix);
        }

        name == pattern
    }
}

// ============================================================================
// Output Formatters
// ============================================================================

/// Generate Prax schema output.
pub fn format_as_prax(schema: &DatabaseSchema, config: &Config) -> String {
    let mut output = String::new();

    output.push_str("// Generated by `prax db pull`\n");
    output.push_str("// Edit this file to customize your schema\n\n");

    output.push_str("datasource db {\n");
    output.push_str(&format!(
        "    provider = \"{}\"\n",
        config.database.provider
    ));
    output.push_str("    url      = env(\"DATABASE_URL\")\n");
    output.push_str("}\n\n");

    output.push_str("generator client {\n");
    output.push_str("    provider = \"prax-client-rust\"\n");
    output.push_str("    output   = \"./src/generated\"\n");
    output.push_str("}\n\n");

    // Use the generate_prax_schema function
    output.push_str(&generate_prax_schema(schema));

    output
}

/// Generate JSON output.
pub fn format_as_json(schema: &DatabaseSchema) -> CliResult<String> {
    serde_json::to_string_pretty(schema)
        .map_err(|e| CliError::Config(format!("Failed to serialize schema: {}", e)))
}

/// Generate SQL DDL output.
pub fn format_as_sql(schema: &DatabaseSchema, db_type: DatabaseType) -> String {
    let mut output = String::new();

    output.push_str("-- Generated by `prax db pull`\n");
    output.push_str(&format!("-- Database: {}\n\n", db_type_name(db_type)));

    // Generate enums (PostgreSQL only)
    if db_type == DatabaseType::PostgreSQL {
        for enum_info in &schema.enums {
            output.push_str(&format!("CREATE TYPE {} AS ENUM (\n", enum_info.name));
            let values: Vec<String> = enum_info
                .values
                .iter()
                .map(|v| format!("    '{}'", v))
                .collect();
            output.push_str(&values.join(",\n"));
            output.push_str("\n);\n\n");
        }
    }

    // Generate tables
    for table in &schema.tables {
        output.push_str(&format!(
            "CREATE TABLE {} (\n",
            quote_identifier(&table.name, db_type)
        ));

        let mut col_defs: Vec<String> = Vec::new();

        for col in &table.columns {
            let mut def = format!(
                "    {} {}",
                quote_identifier(&col.name, db_type),
                col.db_type
            );

            if !col.nullable {
                def.push_str(" NOT NULL");
            }

            if let Some(ref default) = col.default {
                def.push_str(&format!(" DEFAULT {}", default));
            }

            col_defs.push(def);
        }

        // Primary key
        if !table.primary_key.is_empty() {
            let pk_cols: Vec<String> = table
                .primary_key
                .iter()
                .map(|c| quote_identifier(c, db_type))
                .collect();
            col_defs.push(format!("    PRIMARY KEY ({})", pk_cols.join(", ")));
        }

        output.push_str(&col_defs.join(",\n"));
        output.push_str("\n);\n\n");

        // Indexes
        for idx in &table.indexes {
            if idx.is_primary {
                continue;
            }

            let unique = if idx.is_unique { "UNIQUE " } else { "" };
            let cols: Vec<String> = idx
                .columns
                .iter()
                .map(|c| quote_identifier(&c.name, db_type))
                .collect();

            output.push_str(&format!(
                "CREATE {}INDEX {} ON {} ({});\n",
                unique,
                quote_identifier(&idx.name, db_type),
                quote_identifier(&table.name, db_type),
                cols.join(", ")
            ));
        }

        output.push('\n');
    }

    output
}

fn db_type_name(db_type: DatabaseType) -> &'static str {
    match db_type {
        DatabaseType::PostgreSQL => "PostgreSQL",
        DatabaseType::MySQL => "MySQL",
        DatabaseType::SQLite => "SQLite",
        DatabaseType::MSSQL => "SQL Server",
    }
}

fn quote_identifier(name: &str, db_type: DatabaseType) -> String {
    match db_type {
        DatabaseType::PostgreSQL => format!("\"{}\"", name),
        DatabaseType::MySQL => format!("`{}`", name),
        DatabaseType::SQLite => format!("\"{}\"", name),
        DatabaseType::MSSQL => format!("[{}]", name),
    }
}
