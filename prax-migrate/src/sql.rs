//! SQL generation for migrations.

use crate::diff::{
    EnumAlterDiff, EnumDiff, FieldAlterDiff, FieldDiff, IndexDiff, ModelAlterDiff, ModelDiff,
    SchemaDiff, ViewDiff,
};

/// SQL generator for PostgreSQL.
pub struct PostgresSqlGenerator;

impl PostgresSqlGenerator {
    /// Generate SQL for a schema diff.
    pub fn generate(&self, diff: &SchemaDiff) -> MigrationSql {
        let mut up = Vec::new();
        let mut down = Vec::new();

        // Create enums first (they might be used in tables)
        for enum_diff in &diff.create_enums {
            up.push(self.create_enum(enum_diff));
            down.push(self.drop_enum(&enum_diff.name));
        }

        // Drop enums (in reverse order)
        for name in &diff.drop_enums {
            up.push(self.drop_enum(name));
            // Can't easily recreate dropped enums without knowing values
        }

        // Alter enums
        for alter in &diff.alter_enums {
            up.extend(self.alter_enum(alter));
            // Reversing enum alterations is complex
        }

        // Create models
        for model in &diff.create_models {
            up.push(self.create_table(model));
            down.push(self.drop_table(&model.table_name));
        }

        // Drop models
        for name in &diff.drop_models {
            up.push(self.drop_table(name));
            // Can't easily recreate dropped tables
        }

        // Alter models
        for alter in &diff.alter_models {
            up.extend(self.alter_table(alter));
            // Reverse alterations could be generated but complex
        }

        // Create indexes
        for index in &diff.create_indexes {
            up.push(self.create_index(index));
            down.push(self.drop_index(&index.name, &index.table_name));
        }

        // Drop indexes
        for index in &diff.drop_indexes {
            up.push(self.drop_index(&index.name, &index.table_name));
        }

        // Create views (after tables they depend on)
        for view in &diff.create_views {
            up.push(self.create_view(view));
            down.push(self.drop_view(&view.view_name, view.is_materialized));
        }

        // Drop views
        for name in &diff.drop_views {
            // We don't know if it was materialized, so try both
            up.push(self.drop_view(name, false));
        }

        // Alter views (drop and recreate)
        for view in &diff.alter_views {
            // Drop the old view first
            up.push(self.drop_view(&view.view_name, view.is_materialized));
            // Then create the new one
            up.push(self.create_view(view));
        }

        MigrationSql {
            up: up.join("\n\n"),
            down: down.join("\n\n"),
        }
    }

    /// Generate CREATE TYPE for enum.
    fn create_enum(&self, enum_diff: &EnumDiff) -> String {
        let values: Vec<String> = enum_diff
            .values
            .iter()
            .map(|v| format!("'{}'", v))
            .collect();
        format!(
            "CREATE TYPE \"{}\" AS ENUM ({});",
            enum_diff.name,
            values.join(", ")
        )
    }

    /// Generate DROP TYPE.
    fn drop_enum(&self, name: &str) -> String {
        format!("DROP TYPE IF EXISTS \"{}\";", name)
    }

    /// Generate ALTER TYPE statements.
    fn alter_enum(&self, alter: &EnumAlterDiff) -> Vec<String> {
        let mut stmts = Vec::new();

        for value in &alter.add_values {
            stmts.push(format!(
                "ALTER TYPE \"{}\" ADD VALUE IF NOT EXISTS '{}';",
                alter.name, value
            ));
        }

        // Note: PostgreSQL doesn't support removing enum values directly
        // This would require recreating the type

        stmts
    }

    /// Generate CREATE TABLE statement.
    fn create_table(&self, model: &ModelDiff) -> String {
        let mut columns = Vec::new();

        for field in &model.fields {
            columns.push(self.column_definition(field));
        }

        // Add primary key constraint
        if !model.primary_key.is_empty() {
            let pk_cols: Vec<String> = model
                .primary_key
                .iter()
                .map(|c| format!("\"{}\"", c))
                .collect();
            columns.push(format!("PRIMARY KEY ({})", pk_cols.join(", ")));
        }

        // Add unique constraints
        for uc in &model.unique_constraints {
            let cols: Vec<String> = uc.columns.iter().map(|c| format!("\"{}\"", c)).collect();
            let constraint = if let Some(name) = &uc.name {
                format!("CONSTRAINT \"{}\" UNIQUE ({})", name, cols.join(", "))
            } else {
                format!("UNIQUE ({})", cols.join(", "))
            };
            columns.push(constraint);
        }

        format!(
            "CREATE TABLE \"{}\" (\n    {}\n);",
            model.table_name,
            columns.join(",\n    ")
        )
    }

    /// Generate column definition.
    fn column_definition(&self, field: &FieldDiff) -> String {
        let mut parts = vec![format!("\"{}\"", field.column_name), field.sql_type.clone()];

        if field.is_auto_increment {
            // Replace type with SERIAL variants
            if field.sql_type == "INTEGER" {
                parts[1] = "SERIAL".to_string();
            } else if field.sql_type == "BIGINT" {
                parts[1] = "BIGSERIAL".to_string();
            }
        }

        if !field.nullable && !field.is_primary_key {
            parts.push("NOT NULL".to_string());
        }

        if field.is_unique && !field.is_primary_key {
            parts.push("UNIQUE".to_string());
        }

        if let Some(default) = &field.default {
            parts.push(format!("DEFAULT {}", default));
        }

        parts.join(" ")
    }

    /// Generate DROP TABLE statement.
    fn drop_table(&self, name: &str) -> String {
        format!("DROP TABLE IF EXISTS \"{}\" CASCADE;", name)
    }

    /// Generate ALTER TABLE statements.
    fn alter_table(&self, alter: &ModelAlterDiff) -> Vec<String> {
        let mut stmts = Vec::new();

        // Add columns
        for field in &alter.add_fields {
            stmts.push(format!(
                "ALTER TABLE \"{}\" ADD COLUMN {};",
                alter.table_name,
                self.column_definition(field)
            ));
        }

        // Drop columns
        for name in &alter.drop_fields {
            stmts.push(format!(
                "ALTER TABLE \"{}\" DROP COLUMN IF EXISTS \"{}\";",
                alter.table_name, name
            ));
        }

        // Alter columns
        for field in &alter.alter_fields {
            stmts.extend(self.alter_column(&alter.table_name, field));
        }

        // Add indexes
        for index in &alter.add_indexes {
            stmts.push(self.create_index(index));
        }

        // Drop indexes
        for name in &alter.drop_indexes {
            stmts.push(format!("DROP INDEX IF EXISTS \"{}\";", name));
        }

        stmts
    }

    /// Generate ALTER COLUMN statements.
    fn alter_column(&self, table: &str, field: &FieldAlterDiff) -> Vec<String> {
        let mut stmts = Vec::new();

        if let Some(new_type) = &field.new_type {
            stmts.push(format!(
                "ALTER TABLE \"{}\" ALTER COLUMN \"{}\" TYPE {} USING \"{}\"::{};",
                table, field.column_name, new_type, field.column_name, new_type
            ));
        }

        if let Some(new_nullable) = field.new_nullable {
            if new_nullable {
                stmts.push(format!(
                    "ALTER TABLE \"{}\" ALTER COLUMN \"{}\" DROP NOT NULL;",
                    table, field.column_name
                ));
            } else {
                stmts.push(format!(
                    "ALTER TABLE \"{}\" ALTER COLUMN \"{}\" SET NOT NULL;",
                    table, field.column_name
                ));
            }
        }

        if let Some(new_default) = &field.new_default {
            stmts.push(format!(
                "ALTER TABLE \"{}\" ALTER COLUMN \"{}\" SET DEFAULT {};",
                table, field.column_name, new_default
            ));
        }

        stmts
    }

    /// Generate CREATE INDEX statement.
    fn create_index(&self, index: &IndexDiff) -> String {
        let unique = if index.unique { "UNIQUE " } else { "" };
        let cols: Vec<String> = index.columns.iter().map(|c| format!("\"{}\"", c)).collect();
        format!(
            "CREATE {}INDEX \"{}\" ON \"{}\" ({});",
            unique,
            index.name,
            index.table_name,
            cols.join(", ")
        )
    }

    /// Generate DROP INDEX statement.
    fn drop_index(&self, name: &str, _table: &str) -> String {
        format!("DROP INDEX IF EXISTS \"{}\";", name)
    }

    /// Generate CREATE VIEW statement.
    fn create_view(&self, view: &ViewDiff) -> String {
        if view.is_materialized {
            format!(
                "CREATE MATERIALIZED VIEW \"{}\" AS\n{};",
                view.view_name, view.sql_query
            )
        } else {
            format!(
                "CREATE OR REPLACE VIEW \"{}\" AS\n{};",
                view.view_name, view.sql_query
            )
        }
    }

    /// Generate DROP VIEW statement.
    fn drop_view(&self, name: &str, is_materialized: bool) -> String {
        if is_materialized {
            format!("DROP MATERIALIZED VIEW IF EXISTS \"{}\" CASCADE;", name)
        } else {
            format!("DROP VIEW IF EXISTS \"{}\" CASCADE;", name)
        }
    }

    /// Generate REFRESH MATERIALIZED VIEW statement.
    #[allow(dead_code)]
    fn refresh_materialized_view(&self, name: &str, concurrently: bool) -> String {
        if concurrently {
            format!("REFRESH MATERIALIZED VIEW CONCURRENTLY \"{}\";", name)
        } else {
            format!("REFRESH MATERIALIZED VIEW \"{}\";", name)
        }
    }
}

/// Generated SQL for a migration.
#[derive(Debug, Clone)]
pub struct MigrationSql {
    /// SQL to apply the migration.
    pub up: String,
    /// SQL to rollback the migration.
    pub down: String,
}

impl MigrationSql {
    /// Check if the migration is empty.
    pub fn is_empty(&self) -> bool {
        self.up.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_enum() {
        let generator = PostgresSqlGenerator;
        let enum_diff = EnumDiff {
            name: "Status".to_string(),
            values: vec!["PENDING".to_string(), "ACTIVE".to_string()],
        };

        let sql = generator.create_enum(&enum_diff);
        assert!(sql.contains("CREATE TYPE"));
        assert!(sql.contains("Status"));
        assert!(sql.contains("PENDING"));
        assert!(sql.contains("ACTIVE"));
    }

    #[test]
    fn test_create_table() {
        let generator = PostgresSqlGenerator;
        let model = ModelDiff {
            name: "User".to_string(),
            table_name: "users".to_string(),
            fields: vec![
                FieldDiff {
                    name: "id".to_string(),
                    column_name: "id".to_string(),
                    sql_type: "INTEGER".to_string(),
                    nullable: false,
                    default: None,
                    is_primary_key: true,
                    is_auto_increment: true,
                    is_unique: false,
                },
                FieldDiff {
                    name: "email".to_string(),
                    column_name: "email".to_string(),
                    sql_type: "TEXT".to_string(),
                    nullable: false,
                    default: None,
                    is_primary_key: false,
                    is_auto_increment: false,
                    is_unique: true,
                },
            ],
            primary_key: vec!["id".to_string()],
            indexes: Vec::new(),
            unique_constraints: Vec::new(),
        };

        let sql = generator.create_table(&model);
        assert!(sql.contains("CREATE TABLE"));
        assert!(sql.contains("users"));
        assert!(sql.contains("SERIAL"));
        assert!(sql.contains("email"));
        assert!(sql.contains("UNIQUE"));
        assert!(sql.contains("PRIMARY KEY"));
    }

    #[test]
    fn test_create_index() {
        let generator = PostgresSqlGenerator;
        let index = IndexDiff {
            name: "idx_users_email".to_string(),
            table_name: "users".to_string(),
            columns: vec!["email".to_string()],
            unique: true,
        };

        let sql = generator.create_index(&index);
        assert!(sql.contains("CREATE UNIQUE INDEX"));
        assert!(sql.contains("idx_users_email"));
        assert!(sql.contains("users"));
    }

    #[test]
    fn test_alter_table_add_column() {
        let generator = PostgresSqlGenerator;
        let alter = ModelAlterDiff {
            name: "User".to_string(),
            table_name: "users".to_string(),
            add_fields: vec![FieldDiff {
                name: "age".to_string(),
                column_name: "age".to_string(),
                sql_type: "INTEGER".to_string(),
                nullable: true,
                default: None,
                is_primary_key: false,
                is_auto_increment: false,
                is_unique: false,
            }],
            drop_fields: Vec::new(),
            alter_fields: Vec::new(),
            add_indexes: Vec::new(),
            drop_indexes: Vec::new(),
        };

        let stmts = generator.alter_table(&alter);
        assert_eq!(stmts.len(), 1);
        assert!(stmts[0].contains("ADD COLUMN"));
        assert!(stmts[0].contains("age"));
    }

    #[test]
    fn test_create_view() {
        let generator = PostgresSqlGenerator;
        let view = ViewDiff {
            name: "UserStats".to_string(),
            view_name: "user_stats".to_string(),
            sql_query: "SELECT id, COUNT(*) as post_count FROM users GROUP BY id".to_string(),
            is_materialized: false,
            refresh_interval: None,
            fields: vec![],
        };

        let sql = generator.create_view(&view);
        assert!(sql.contains("CREATE OR REPLACE VIEW"));
        assert!(sql.contains("user_stats"));
        assert!(sql.contains("SELECT id"));
        assert!(sql.contains("post_count"));
    }

    #[test]
    fn test_create_materialized_view() {
        let generator = PostgresSqlGenerator;
        let view = ViewDiff {
            name: "UserStats".to_string(),
            view_name: "user_stats".to_string(),
            sql_query: "SELECT id, COUNT(*) as post_count FROM users GROUP BY id".to_string(),
            is_materialized: true,
            refresh_interval: Some("1h".to_string()),
            fields: vec![],
        };

        let sql = generator.create_view(&view);
        assert!(sql.contains("CREATE MATERIALIZED VIEW"));
        assert!(sql.contains("user_stats"));
        assert!(!sql.contains("OR REPLACE")); // Materialized views don't support OR REPLACE
    }

    #[test]
    fn test_drop_view() {
        let generator = PostgresSqlGenerator;

        let sql = generator.drop_view("user_stats", false);
        assert!(sql.contains("DROP VIEW"));
        assert!(sql.contains("user_stats"));
        assert!(sql.contains("CASCADE"));

        let sql_mat = generator.drop_view("user_stats", true);
        assert!(sql_mat.contains("DROP MATERIALIZED VIEW"));
        assert!(sql_mat.contains("user_stats"));
    }

    #[test]
    fn test_refresh_materialized_view() {
        let generator = PostgresSqlGenerator;

        let sql = generator.refresh_materialized_view("user_stats", false);
        assert!(sql.contains("REFRESH MATERIALIZED VIEW"));
        assert!(sql.contains("user_stats"));
        assert!(!sql.contains("CONCURRENTLY"));

        let sql_concurrent = generator.refresh_materialized_view("user_stats", true);
        assert!(sql_concurrent.contains("CONCURRENTLY"));
    }

    #[test]
    fn test_generate_with_views() {
        use crate::diff::SchemaDiff;

        let generator = PostgresSqlGenerator;
        let mut diff = SchemaDiff::default();
        diff.create_views.push(ViewDiff {
            name: "ActiveUsers".to_string(),
            view_name: "active_users".to_string(),
            sql_query: "SELECT * FROM users WHERE active = true".to_string(),
            is_materialized: false,
            refresh_interval: None,
            fields: vec![],
        });

        let sql = generator.generate(&diff);
        assert!(!sql.is_empty());
        assert!(sql.up.contains("CREATE OR REPLACE VIEW"));
        assert!(sql.up.contains("active_users"));
        assert!(sql.down.contains("DROP VIEW"));
    }
}
