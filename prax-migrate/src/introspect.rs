//! Database introspection for reverse-engineering schemas.
//!
//! This module provides functionality to introspect an existing database
//! and generate a Prax schema from its structure.

use std::collections::HashMap;

use prax_schema::Schema;
use prax_schema::ast::{
    Attribute, AttributeArg, AttributeValue, Enum, EnumVariant, Field, FieldType, Ident, Model,
    ScalarType, Span, TypeModifier,
};

use crate::error::{MigrateResult, MigrationError};

/// Result of introspecting a database.
#[derive(Debug, Clone)]
pub struct IntrospectionResult {
    /// The generated schema.
    pub schema: Schema,
    /// Tables that were skipped.
    pub skipped_tables: Vec<SkippedTable>,
    /// Warnings generated during introspection.
    pub warnings: Vec<String>,
}

/// A table that was skipped during introspection.
#[derive(Debug, Clone)]
pub struct SkippedTable {
    /// Table name.
    pub name: String,
    /// Reason it was skipped.
    pub reason: String,
}

/// Configuration for introspection.
#[derive(Debug, Clone)]
pub struct IntrospectionConfig {
    /// Schema to introspect (default: "public").
    pub database_schema: String,
    /// Tables to include (empty = all).
    pub include_tables: Vec<String>,
    /// Tables to exclude.
    pub exclude_tables: Vec<String>,
    /// Whether to include views.
    pub include_views: bool,
    /// Whether to include enums.
    pub include_enums: bool,
}

impl Default for IntrospectionConfig {
    fn default() -> Self {
        Self {
            database_schema: "public".to_string(),
            include_tables: Vec::new(),
            exclude_tables: vec![
                "_prax_migrations".to_string(),
                "_prisma_migrations".to_string(),
                "schema_migrations".to_string(),
            ],
            include_views: true,
            include_enums: true,
        }
    }
}

impl IntrospectionConfig {
    /// Create a new introspection config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the database schema to introspect.
    pub fn database_schema(mut self, schema: impl Into<String>) -> Self {
        self.database_schema = schema.into();
        self
    }

    /// Include only these tables.
    pub fn include_tables(mut self, tables: Vec<String>) -> Self {
        self.include_tables = tables;
        self
    }

    /// Exclude these tables.
    pub fn exclude_tables(mut self, tables: Vec<String>) -> Self {
        self.exclude_tables = tables;
        self
    }

    /// Whether to include views.
    pub fn include_views(mut self, include: bool) -> Self {
        self.include_views = include;
        self
    }

    /// Whether to include enums.
    pub fn include_enums(mut self, include: bool) -> Self {
        self.include_enums = include;
        self
    }

    /// Check if a table should be included.
    pub fn should_include_table(&self, name: &str) -> bool {
        if self.exclude_tables.contains(&name.to_string()) {
            return false;
        }
        if self.include_tables.is_empty() {
            return true;
        }
        self.include_tables.contains(&name.to_string())
    }
}

/// Raw table information from the database.
#[derive(Debug, Clone)]
pub struct TableInfo {
    /// Table name.
    pub name: String,
    /// Table schema (e.g., "public").
    pub schema: String,
    /// Table type ("BASE TABLE" or "VIEW").
    pub table_type: String,
    /// Table comment.
    pub comment: Option<String>,
}

/// Raw column information from the database.
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    /// Column name.
    pub name: String,
    /// Data type (e.g., "integer", "character varying").
    pub data_type: String,
    /// Full UDT name (e.g., "int4", "varchar").
    pub udt_name: String,
    /// Character maximum length (for varchar, etc.).
    pub character_maximum_length: Option<i32>,
    /// Numeric precision.
    pub numeric_precision: Option<i32>,
    /// Whether the column is nullable.
    pub is_nullable: bool,
    /// Default value expression.
    pub column_default: Option<String>,
    /// Ordinal position.
    pub ordinal_position: i32,
    /// Column comment.
    pub comment: Option<String>,
}

/// Raw constraint information from the database.
#[derive(Debug, Clone)]
pub struct ConstraintInfo {
    /// Constraint name.
    pub name: String,
    /// Constraint type (PRIMARY KEY, UNIQUE, FOREIGN KEY, CHECK).
    pub constraint_type: String,
    /// Table name.
    pub table_name: String,
    /// Columns in the constraint.
    pub columns: Vec<String>,
    /// Referenced table (for foreign keys).
    pub referenced_table: Option<String>,
    /// Referenced columns (for foreign keys).
    pub referenced_columns: Option<Vec<String>>,
    /// On delete action (for foreign keys).
    pub on_delete: Option<String>,
    /// On update action (for foreign keys).
    pub on_update: Option<String>,
}

/// Raw enum information from the database.
#[derive(Debug, Clone)]
pub struct EnumInfo {
    /// Enum name.
    pub name: String,
    /// Enum values.
    pub values: Vec<String>,
    /// Schema the enum belongs to.
    pub schema: String,
}

/// Raw index information from the database.
#[derive(Debug, Clone)]
pub struct IndexInfo {
    /// Index name.
    pub name: String,
    /// Table name.
    pub table_name: String,
    /// Columns in the index.
    pub columns: Vec<String>,
    /// Whether the index is unique.
    pub is_unique: bool,
    /// Whether this is a primary key index.
    pub is_primary: bool,
    /// Index method (btree, hash, etc.).
    pub index_method: String,
}

/// Trait for database introspection.
#[async_trait::async_trait]
pub trait Introspector: Send + Sync {
    /// Get all tables in the database.
    async fn get_tables(&self, config: &IntrospectionConfig) -> MigrateResult<Vec<TableInfo>>;

    /// Get columns for a table.
    async fn get_columns(&self, table: &str, schema: &str) -> MigrateResult<Vec<ColumnInfo>>;

    /// Get constraints for a table.
    async fn get_constraints(
        &self,
        table: &str,
        schema: &str,
    ) -> MigrateResult<Vec<ConstraintInfo>>;

    /// Get indexes for a table.
    async fn get_indexes(&self, table: &str, schema: &str) -> MigrateResult<Vec<IndexInfo>>;

    /// Get all enums in the database.
    async fn get_enums(&self, schema: &str) -> MigrateResult<Vec<EnumInfo>>;
}

/// Build a Prax schema from introspection data.
pub struct SchemaBuilder {
    config: IntrospectionConfig,
    tables: Vec<TableInfo>,
    columns: HashMap<String, Vec<ColumnInfo>>,
    constraints: HashMap<String, Vec<ConstraintInfo>>,
    indexes: HashMap<String, Vec<IndexInfo>>,
    enums: Vec<EnumInfo>,
}

impl SchemaBuilder {
    /// Create a new schema builder.
    pub fn new(config: IntrospectionConfig) -> Self {
        Self {
            config,
            tables: Vec::new(),
            columns: HashMap::new(),
            constraints: HashMap::new(),
            indexes: HashMap::new(),
            enums: Vec::new(),
        }
    }

    /// Add table information.
    pub fn with_tables(mut self, tables: Vec<TableInfo>) -> Self {
        self.tables = tables;
        self
    }

    /// Add column information for a table.
    pub fn with_columns(mut self, table: &str, columns: Vec<ColumnInfo>) -> Self {
        self.columns.insert(table.to_string(), columns);
        self
    }

    /// Add constraint information for a table.
    pub fn with_constraints(mut self, table: &str, constraints: Vec<ConstraintInfo>) -> Self {
        self.constraints.insert(table.to_string(), constraints);
        self
    }

    /// Add index information for a table.
    pub fn with_indexes(mut self, table: &str, indexes: Vec<IndexInfo>) -> Self {
        self.indexes.insert(table.to_string(), indexes);
        self
    }

    /// Add enum information.
    pub fn with_enums(mut self, enums: Vec<EnumInfo>) -> Self {
        self.enums = enums;
        self
    }

    /// Build the schema from the collected information.
    pub fn build(self) -> MigrateResult<IntrospectionResult> {
        let mut schema = Schema::new();
        let mut skipped_tables = Vec::new();
        let mut warnings = Vec::new();

        // Add enums first (they may be referenced by columns)
        if self.config.include_enums {
            for enum_info in &self.enums {
                let prax_enum = self.build_enum(enum_info);
                schema.add_enum(prax_enum);
            }
        }

        // Build models from tables
        for table in &self.tables {
            if !self.config.should_include_table(&table.name) {
                skipped_tables.push(SkippedTable {
                    name: table.name.clone(),
                    reason: "Excluded by configuration".to_string(),
                });
                continue;
            }

            // Skip views if not configured
            if table.table_type == "VIEW" && !self.config.include_views {
                continue;
            }

            match self.build_model(table) {
                Ok(model) => {
                    schema.add_model(model);
                }
                Err(e) => {
                    warnings.push(format!("Failed to build model for '{}': {}", table.name, e));
                    skipped_tables.push(SkippedTable {
                        name: table.name.clone(),
                        reason: e.to_string(),
                    });
                }
            }
        }

        Ok(IntrospectionResult {
            schema,
            skipped_tables,
            warnings,
        })
    }

    /// Build an enum from database enum info.
    fn build_enum(&self, info: &EnumInfo) -> Enum {
        let span = Span::new(0, 0);
        let name = Ident::new(to_pascal_case(&info.name), span);
        let mut prax_enum = Enum::new(name, span);

        for value in &info.values {
            prax_enum.add_variant(EnumVariant::new(Ident::new(value.clone(), span), span));
        }

        prax_enum
    }

    /// Build a model from table info.
    fn build_model(&self, table: &TableInfo) -> MigrateResult<Model> {
        let span = Span::new(0, 0);
        let name = Ident::new(to_pascal_case(&table.name), span);
        let mut model = Model::new(name, span);

        // Add @@map attribute if table name differs from model name
        let model_name = to_pascal_case(&table.name);
        if table.name != model_name && table.name != to_snake_case(&model_name) {
            model.attributes.push(Attribute::new(
                Ident::new("map", span),
                vec![AttributeArg::positional(
                    AttributeValue::String(table.name.clone()),
                    span,
                )],
                span,
            ));
        }

        // Get columns for this table
        let columns = self.columns.get(&table.name).cloned().unwrap_or_default();

        // Get constraints for this table
        let constraints = self
            .constraints
            .get(&table.name)
            .cloned()
            .unwrap_or_default();

        // Find primary key columns
        let pk_columns: Vec<&str> = constraints
            .iter()
            .filter(|c| c.constraint_type == "PRIMARY KEY")
            .flat_map(|c| c.columns.iter().map(|s| s.as_str()))
            .collect();

        // Find unique columns
        let unique_columns: Vec<&str> = constraints
            .iter()
            .filter(|c| c.constraint_type == "UNIQUE")
            .filter(|c| c.columns.len() == 1)
            .flat_map(|c| c.columns.iter().map(|s| s.as_str()))
            .collect();

        // Build fields from columns
        for column in columns {
            let field = self.build_field(&column, &pk_columns, &unique_columns)?;
            model.add_field(field);
        }

        Ok(model)
    }

    /// Build a field from column info.
    fn build_field(
        &self,
        column: &ColumnInfo,
        pk_columns: &[&str],
        unique_columns: &[&str],
    ) -> MigrateResult<Field> {
        let span = Span::new(0, 0);
        let name = Ident::new(&column.name, span);

        // Map SQL type to Prax type
        let (field_type, needs_map) = self.sql_type_to_prax(&column.udt_name, &column.data_type)?;

        // Determine modifier
        let modifier = if column.is_nullable {
            TypeModifier::Optional
        } else {
            TypeModifier::Required
        };

        let mut attributes = Vec::new();

        // Add @id if this is a primary key
        if pk_columns.contains(&column.name.as_str()) {
            attributes.push(Attribute::simple(Ident::new("id", span), span));

            // Check for auto-increment
            if let Some(default) = &column.column_default
                && (default.contains("nextval") || default.contains("SERIAL"))
            {
                attributes.push(Attribute::simple(Ident::new("auto", span), span));
            }
        }

        // Add @unique if this is a unique column
        if unique_columns.contains(&column.name.as_str()) {
            attributes.push(Attribute::simple(Ident::new("unique", span), span));
        }

        // Add @default if there's a default value (skip auto-increment defaults)
        if let Some(default) = &column.column_default
            && !default.contains("nextval")
            && let Some(value) = parse_default_value(default)
        {
            attributes.push(Attribute::new(
                Ident::new("default", span),
                vec![AttributeArg::positional(value, span)],
                span,
            ));
        }

        // Add @map if column name differs from field name
        if needs_map {
            attributes.push(Attribute::new(
                Ident::new("map", span),
                vec![AttributeArg::positional(
                    AttributeValue::String(column.name.clone()),
                    span,
                )],
                span,
            ));
        }

        Ok(Field::new(name, field_type, modifier, attributes, span))
    }

    /// Convert SQL type to Prax field type.
    fn sql_type_to_prax(
        &self,
        udt_name: &str,
        data_type: &str,
    ) -> MigrateResult<(FieldType, bool)> {
        // Check if this is a known enum
        let enum_names: Vec<&str> = self.enums.iter().map(|e| e.name.as_str()).collect();
        if enum_names.contains(&udt_name) {
            return Ok((FieldType::Enum(to_pascal_case(udt_name).into()), false));
        }

        let scalar = match udt_name {
            "int2" | "int4" | "integer" | "smallint" => ScalarType::Int,
            "int8" | "bigint" => ScalarType::BigInt,
            "float4" | "float8" | "real" | "double precision" => ScalarType::Float,
            "numeric" | "decimal" | "money" => ScalarType::Decimal,
            "text" | "varchar" | "char" | "character varying" | "character" | "bpchar" => {
                ScalarType::String
            }
            "bool" | "boolean" => ScalarType::Boolean,
            "timestamp"
            | "timestamptz"
            | "timestamp with time zone"
            | "timestamp without time zone" => ScalarType::DateTime,
            "date" => ScalarType::Date,
            "time" | "timetz" | "time with time zone" | "time without time zone" => {
                ScalarType::Time
            }
            "json" | "jsonb" => ScalarType::Json,
            "bytea" => ScalarType::Bytes,
            "uuid" => ScalarType::Uuid,
            _ => {
                // Try to match by data_type as fallback
                match data_type {
                    "integer" | "smallint" => ScalarType::Int,
                    "bigint" => ScalarType::BigInt,
                    "real" | "double precision" => ScalarType::Float,
                    "numeric" => ScalarType::Decimal,
                    "character varying" | "character" | "text" => ScalarType::String,
                    "boolean" => ScalarType::Boolean,
                    "timestamp with time zone" | "timestamp without time zone" => {
                        ScalarType::DateTime
                    }
                    "date" => ScalarType::Date,
                    "time with time zone" | "time without time zone" => ScalarType::Time,
                    "json" | "jsonb" => ScalarType::Json,
                    "bytea" => ScalarType::Bytes,
                    "uuid" => ScalarType::Uuid,
                    "ARRAY" => {
                        // Arrays are complex - for now, treat as Json
                        ScalarType::Json
                    }
                    "USER-DEFINED" => {
                        // This might be an enum we haven't seen
                        return Err(MigrationError::InvalidMigration(format!(
                            "Unknown user-defined type: {}",
                            udt_name
                        )));
                    }
                    _ => {
                        return Err(MigrationError::InvalidMigration(format!(
                            "Unknown SQL type: {} ({})",
                            udt_name, data_type
                        )));
                    }
                }
            }
        };

        Ok((FieldType::Scalar(scalar), false))
    }
}

/// Parse a default value expression into an AttributeValue.
fn parse_default_value(default: &str) -> Option<AttributeValue> {
    let trimmed = default.trim();

    // Handle booleans
    if trimmed == "true" || trimmed == "TRUE" {
        return Some(AttributeValue::Boolean(true));
    }
    if trimmed == "false" || trimmed == "FALSE" {
        return Some(AttributeValue::Boolean(false));
    }

    // Handle NULL
    if trimmed.to_uppercase() == "NULL" {
        return None;
    }

    // Handle integers
    if let Ok(int) = trimmed.parse::<i64>() {
        return Some(AttributeValue::Int(int));
    }

    // Handle floats
    if let Ok(float) = trimmed.parse::<f64>() {
        return Some(AttributeValue::Float(float));
    }

    // Handle strings (enclosed in quotes)
    if (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        || (trimmed.starts_with('"') && trimmed.ends_with('"'))
    {
        let inner = &trimmed[1..trimmed.len() - 1];
        return Some(AttributeValue::String(inner.to_string()));
    }

    // Handle PostgreSQL type casts (e.g., 'value'::type)
    if let Some(pos) = trimmed.find("::") {
        return parse_default_value(&trimmed[..pos]);
    }

    // Handle function calls (e.g., now(), uuid_generate_v4())
    if trimmed.ends_with("()") || trimmed.contains('(') {
        let func_name = if let Some(paren_pos) = trimmed.find('(') {
            &trimmed[..paren_pos]
        } else {
            &trimmed[..trimmed.len() - 2]
        };
        return Some(AttributeValue::Function(
            func_name.to_string().into(),
            vec![],
        ));
    }

    // Unknown default - return as string
    Some(AttributeValue::String(trimmed.to_string()))
}

/// Convert snake_case to PascalCase.
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

/// Convert PascalCase to snake_case.
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap_or(ch));
        } else {
            result.push(ch);
        }
    }
    result
}

/// SQL queries for PostgreSQL introspection.
pub mod postgres_queries {
    /// Query to get all tables and views.
    pub const TABLES: &str = r#"
        SELECT
            table_name,
            table_schema,
            table_type
        FROM information_schema.tables
        WHERE table_schema = $1
        ORDER BY table_name
    "#;

    /// Query to get columns for a table.
    pub const COLUMNS: &str = r#"
        SELECT
            column_name,
            data_type,
            udt_name,
            character_maximum_length,
            numeric_precision,
            is_nullable = 'YES' as is_nullable,
            column_default,
            ordinal_position
        FROM information_schema.columns
        WHERE table_schema = $1 AND table_name = $2
        ORDER BY ordinal_position
    "#;

    /// Query to get constraints.
    pub const CONSTRAINTS: &str = r#"
        SELECT
            tc.constraint_name,
            tc.constraint_type,
            tc.table_name,
            kcu.column_name,
            ccu.table_name AS referenced_table,
            ccu.column_name AS referenced_column,
            rc.delete_rule,
            rc.update_rule
        FROM information_schema.table_constraints tc
        LEFT JOIN information_schema.key_column_usage kcu
            ON tc.constraint_name = kcu.constraint_name
            AND tc.table_schema = kcu.table_schema
        LEFT JOIN information_schema.constraint_column_usage ccu
            ON tc.constraint_name = ccu.constraint_name
            AND tc.table_schema = ccu.table_schema
            AND tc.constraint_type = 'FOREIGN KEY'
        LEFT JOIN information_schema.referential_constraints rc
            ON tc.constraint_name = rc.constraint_name
            AND tc.table_schema = rc.constraint_schema
        WHERE tc.table_schema = $1 AND tc.table_name = $2
        ORDER BY tc.constraint_name, kcu.ordinal_position
    "#;

    /// Query to get indexes.
    pub const INDEXES: &str = r#"
        SELECT
            i.relname AS index_name,
            t.relname AS table_name,
            array_agg(a.attname ORDER BY array_position(ix.indkey, a.attnum)) AS columns,
            ix.indisunique AS is_unique,
            ix.indisprimary AS is_primary,
            am.amname AS index_method
        FROM pg_index ix
        JOIN pg_class i ON ix.indexrelid = i.oid
        JOIN pg_class t ON ix.indrelid = t.oid
        JOIN pg_namespace n ON t.relnamespace = n.oid
        JOIN pg_am am ON i.relam = am.oid
        JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey)
        WHERE n.nspname = $1 AND t.relname = $2
        GROUP BY i.relname, t.relname, ix.indisunique, ix.indisprimary, am.amname
    "#;

    /// Query to get enums.
    pub const ENUMS: &str = r#"
        SELECT
            t.typname AS enum_name,
            n.nspname AS schema_name,
            array_agg(e.enumlabel ORDER BY e.enumsortorder) AS enum_values
        FROM pg_type t
        JOIN pg_namespace n ON t.typnamespace = n.oid
        JOIN pg_enum e ON t.oid = e.enumtypid
        WHERE n.nspname = $1
        GROUP BY t.typname, n.nspname
    "#;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("user"), "User");
        assert_eq!(to_pascal_case("user_profile"), "UserProfile");
        assert_eq!(
            to_pascal_case("user_profile_settings"),
            "UserProfileSettings"
        );
        assert_eq!(to_pascal_case("_user_"), "User");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("User"), "user");
        assert_eq!(to_snake_case("UserProfile"), "user_profile");
        assert_eq!(to_snake_case("HTTPResponse"), "h_t_t_p_response");
    }

    #[test]
    fn test_parse_default_value_boolean() {
        assert!(matches!(
            parse_default_value("true"),
            Some(AttributeValue::Boolean(true))
        ));
        assert!(matches!(
            parse_default_value("false"),
            Some(AttributeValue::Boolean(false))
        ));
    }

    #[test]
    fn test_parse_default_value_int() {
        assert!(matches!(
            parse_default_value("42"),
            Some(AttributeValue::Int(42))
        ));
        assert!(matches!(
            parse_default_value("-5"),
            Some(AttributeValue::Int(-5))
        ));
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_parse_default_value_float() {
        if let Some(AttributeValue::Float(f)) = parse_default_value("3.14") {
            assert!((f - 3.14).abs() < 0.001);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn test_parse_default_value_string() {
        if let Some(AttributeValue::String(s)) = parse_default_value("'hello'") {
            assert_eq!(s.as_str(), "hello");
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_parse_default_value_function() {
        if let Some(AttributeValue::Function(name, args)) = parse_default_value("now()") {
            assert_eq!(name.as_str(), "now");
            assert!(args.is_empty());
        } else {
            panic!("Expected Function");
        }
    }

    #[test]
    fn test_parse_default_value_with_cast() {
        if let Some(AttributeValue::String(s)) = parse_default_value("'active'::status_type") {
            assert_eq!(s.as_str(), "active");
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_config_should_include_table() {
        let config = IntrospectionConfig::default();
        assert!(config.should_include_table("users"));
        assert!(!config.should_include_table("_prax_migrations"));
    }

    #[test]
    fn test_config_include_specific_tables() {
        let config = IntrospectionConfig::new().include_tables(vec!["users".to_string()]);
        assert!(config.should_include_table("users"));
        assert!(!config.should_include_table("posts"));
    }

    #[test]
    fn test_sql_type_mapping() {
        let builder = SchemaBuilder::new(IntrospectionConfig::default());

        let (ft, _) = builder.sql_type_to_prax("int4", "integer").unwrap();
        assert!(matches!(ft, FieldType::Scalar(ScalarType::Int)));

        let (ft, _) = builder.sql_type_to_prax("text", "text").unwrap();
        assert!(matches!(ft, FieldType::Scalar(ScalarType::String)));

        let (ft, _) = builder.sql_type_to_prax("bool", "boolean").unwrap();
        assert!(matches!(ft, FieldType::Scalar(ScalarType::Boolean)));

        let (ft, _) = builder
            .sql_type_to_prax("timestamptz", "timestamp with time zone")
            .unwrap();
        assert!(matches!(ft, FieldType::Scalar(ScalarType::DateTime)));

        let (ft, _) = builder.sql_type_to_prax("uuid", "uuid").unwrap();
        assert!(matches!(ft, FieldType::Scalar(ScalarType::Uuid)));
    }
}
