//! Migration dialect trait for abstracting over SQL and CQL backends.

use crate::diff::SchemaDiff;
use crate::sql::{
    DuckDbSqlGenerator, MigrationSql, MssqlGenerator, MySqlGenerator, PostgresSqlGenerator,
    SqliteGenerator,
};

/// Marker trait — dialects that support computed columns of the form
/// `GENERATED ALWAYS AS (expr) STORED|VIRTUAL` (or vendor-specific
/// equivalent: MySQL `AS (expr) STORED|VIRTUAL`, MSSQL
/// `AS (expr) [PERSISTED]`). Implemented by every SQL generator;
/// not implemented by `CqlMigrationGenerator`.
pub trait SupportsGeneratedColumns {}

/// A migration dialect abstracts the schema diff type, migration output type,
/// and generator for a specific database backend.
pub trait MigrationDialect {
    /// The schema diff type for this dialect.
    type Diff: Default + Send + Sync;

    /// The migration output type for this dialect.
    type Migration: Send + Sync;

    /// Human-readable dialect name (e.g., "sql", "cql").
    fn name() -> &'static str;

    /// Generate a migration from a schema diff.
    fn generate(diff: &Self::Diff) -> Self::Migration;

    /// Event log table name used by this dialect.
    fn event_log_table() -> &'static str;
}

/// A specific SQL database backend for migration generation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum SqlBackend {
    /// PostgreSQL (the default; also used by the static
    /// [`MigrationDialect::generate`] for backward compatibility).
    #[default]
    Postgres,
    /// MySQL / MariaDB.
    MySql,
    /// SQLite.
    Sqlite,
    /// Microsoft SQL Server.
    Mssql,
    /// DuckDB.
    DuckDb,
}

/// The SQL dialect (PostgreSQL, MySQL, SQLite, MSSQL, DuckDB share this).
///
/// The dialect dispatches to the vendor-specific generators in
/// [`crate::sql`] based on a [`SqlBackend`]. Because
/// [`MigrationDialect::generate`] is a static trait method with no access
/// to instance state, it always routes to PostgreSQL for backward
/// compatibility — use [`SqlDialect::for_backend`] together with
/// [`SqlDialect::generate_migration`] to target any other backend.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SqlDialect {
    backend: SqlBackend,
}

impl SqlDialect {
    /// Create a SQL dialect that generates migrations for `backend`.
    pub fn for_backend(backend: SqlBackend) -> Self {
        Self { backend }
    }

    /// The backend this dialect generates migrations for.
    pub fn backend(&self) -> SqlBackend {
        self.backend
    }

    /// Generate a migration, dispatching to the generator matching
    /// this dialect's [`SqlBackend`].
    pub fn generate_migration(&self, diff: &SchemaDiff) -> MigrationSql {
        Self::generate_with(self.backend, diff)
    }

    /// Dispatch a diff to the matching vendor generator.
    fn generate_with(backend: SqlBackend, diff: &SchemaDiff) -> MigrationSql {
        match backend {
            SqlBackend::Postgres => PostgresSqlGenerator.generate(diff),
            SqlBackend::MySql => MySqlGenerator.generate(diff),
            SqlBackend::Sqlite => SqliteGenerator.generate(diff),
            SqlBackend::Mssql => MssqlGenerator.generate(diff),
            SqlBackend::DuckDb => DuckDbSqlGenerator.generate(diff),
        }
    }
}

impl MigrationDialect for SqlDialect {
    type Diff = SchemaDiff;
    type Migration = MigrationSql;

    fn name() -> &'static str {
        "sql"
    }

    /// Compatibility entry point: as a static trait method it cannot see a
    /// chosen backend, so it always generates PostgreSQL-flavored SQL. Use
    /// `SqlDialect::for_backend(backend).generate_migration(diff)` for
    /// MySQL, SQLite, MSSQL, or DuckDB output.
    fn generate(diff: &SchemaDiff) -> MigrationSql {
        Self::generate_with(SqlBackend::Postgres, diff)
    }

    fn event_log_table() -> &'static str {
        "_prax_migrations"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{FieldDiff, ModelDiff};

    fn sample_user_diff() -> SchemaDiff {
        let mut diff = SchemaDiff::default();
        diff.create_models.push(ModelDiff {
            name: "User".to_string(),
            table_name: "users".to_string(),
            fields: vec![FieldDiff {
                name: "id".to_string(),
                column_name: "id".to_string(),
                sql_type: "BIGINT".to_string(),
                nullable: false,
                default: None,
                is_primary_key: true,
                is_auto_increment: true,
                is_unique: false,
                vector: None,
                enum_name: None,
                generated: None,
            }],
            primary_key: vec!["id".to_string()],
            indexes: Vec::new(),
            unique_constraints: Vec::new(),
            foreign_keys: Vec::new(),
        });
        diff
    }

    #[test]
    fn test_sql_dialect_name() {
        assert_eq!(SqlDialect::name(), "sql");
    }

    #[test]
    fn test_sql_dialect_event_log_table() {
        assert_eq!(SqlDialect::event_log_table(), "_prax_migrations");
    }

    #[test]
    fn test_sql_dialect_generates_empty_migration_from_empty_diff() {
        let diff = SchemaDiff::default();
        let migration = SqlDialect::generate(&diff);
        assert!(migration.is_empty());
    }

    #[test]
    fn test_cql_dialect_name() {
        use crate::cql::CqlDialect;
        assert_eq!(CqlDialect::name(), "cql");
    }

    #[test]
    fn test_cql_dialect_event_log_table() {
        use crate::cql::CqlDialect;
        assert_eq!(CqlDialect::event_log_table(), "_prax_cql_migrations");
    }

    #[test]
    fn test_cql_dialect_generates_empty_migration_from_empty_diff() {
        use crate::cql::{CqlDialect, CqlSchemaDiff};
        let diff = CqlSchemaDiff::default();
        let migration = CqlDialect::generate(&diff);
        assert!(migration.is_empty());
    }

    #[test]
    fn test_sql_dialect_matches_postgres_generator_directly() {
        let diff = sample_user_diff();

        let via_trait = SqlDialect::generate(&diff);
        let via_direct = PostgresSqlGenerator.generate(&diff);

        assert_eq!(via_trait.up, via_direct.up);
        assert_eq!(via_trait.down, via_direct.down);
        assert_eq!(via_trait.warnings, via_direct.warnings);
    }

    #[test]
    fn test_sql_dialect_default_backend_is_postgres() {
        assert_eq!(SqlBackend::default(), SqlBackend::Postgres);
        assert_eq!(SqlDialect::default().backend(), SqlBackend::Postgres);
        assert_eq!(
            SqlDialect::for_backend(SqlBackend::MySql).backend(),
            SqlBackend::MySql
        );
    }

    #[test]
    fn test_sql_dialect_for_backend_routes_to_each_generator() {
        let diff = sample_user_diff();

        let cases = [
            (SqlBackend::Postgres, PostgresSqlGenerator.generate(&diff)),
            (SqlBackend::MySql, MySqlGenerator.generate(&diff)),
            (SqlBackend::Sqlite, SqliteGenerator.generate(&diff)),
            (SqlBackend::Mssql, MssqlGenerator.generate(&diff)),
            (SqlBackend::DuckDb, DuckDbSqlGenerator.generate(&diff)),
        ];

        for (backend, expected) in cases {
            let via_dialect = SqlDialect::for_backend(backend).generate_migration(&diff);
            assert_eq!(via_dialect.up, expected.up, "up mismatch for {backend:?}");
            assert_eq!(
                via_dialect.down, expected.down,
                "down mismatch for {backend:?}"
            );
            assert_eq!(
                via_dialect.warnings, expected.warnings,
                "warnings mismatch for {backend:?}"
            );
        }
    }
}
