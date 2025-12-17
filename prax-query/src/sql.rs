//! SQL generation utilities.

use crate::filter::FilterValue;

/// Escape a string for use in SQL (for identifiers, not values).
pub fn escape_identifier(name: &str) -> String {
    // Double any existing quotes
    let escaped = name.replace('"', "\"\"");
    format!("\"{}\"", escaped)
}

/// Check if an identifier needs quoting.
pub fn needs_quoting(name: &str) -> bool {
    // Reserved keywords or names with special characters need quoting
    let reserved = [
        "user", "order", "group", "select", "from", "where", "table", "index",
        "key", "primary", "foreign", "check", "default", "null", "not", "and",
        "or", "in", "is", "like", "between", "case", "when", "then", "else",
        "end", "as", "on", "join", "left", "right", "inner", "outer", "cross",
        "natural", "using", "limit", "offset", "union", "intersect", "except",
        "all", "distinct", "having", "create", "alter", "drop", "insert",
        "update", "delete", "into", "values", "set", "returning",
    ];

    // Check for reserved words
    if reserved.contains(&name.to_lowercase().as_str()) {
        return true;
    }

    // Check for special characters
    !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Quote an identifier if needed.
pub fn quote_identifier(name: &str) -> String {
    if needs_quoting(name) {
        escape_identifier(name)
    } else {
        name.to_string()
    }
}

/// Build a parameter placeholder for a given database type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseType {
    /// PostgreSQL uses $1, $2, etc.
    PostgreSQL,
    /// MySQL uses ?, ?, etc.
    MySQL,
    /// SQLite uses ?, ?, etc.
    SQLite,
}

impl DatabaseType {
    /// Get the parameter placeholder for this database type.
    pub fn placeholder(&self, index: usize) -> String {
        match self {
            Self::PostgreSQL => format!("${}", index),
            Self::MySQL | Self::SQLite => "?".to_string(),
        }
    }
}

impl Default for DatabaseType {
    fn default() -> Self {
        Self::PostgreSQL
    }
}

/// A SQL builder for constructing queries.
#[derive(Debug, Clone)]
pub struct SqlBuilder {
    db_type: DatabaseType,
    parts: Vec<String>,
    params: Vec<FilterValue>,
}

impl SqlBuilder {
    /// Create a new SQL builder.
    pub fn new(db_type: DatabaseType) -> Self {
        Self {
            db_type,
            parts: Vec::new(),
            params: Vec::new(),
        }
    }

    /// Create a PostgreSQL SQL builder.
    pub fn postgres() -> Self {
        Self::new(DatabaseType::PostgreSQL)
    }

    /// Create a MySQL SQL builder.
    pub fn mysql() -> Self {
        Self::new(DatabaseType::MySQL)
    }

    /// Create a SQLite SQL builder.
    pub fn sqlite() -> Self {
        Self::new(DatabaseType::SQLite)
    }

    /// Push a literal SQL string.
    pub fn push(&mut self, sql: impl AsRef<str>) -> &mut Self {
        self.parts.push(sql.as_ref().to_string());
        self
    }

    /// Push a SQL string with a parameter.
    pub fn push_param(&mut self, value: impl Into<FilterValue>) -> &mut Self {
        let index = self.params.len() + 1;
        self.parts.push(self.db_type.placeholder(index));
        self.params.push(value.into());
        self
    }

    /// Push an identifier (properly quoted if needed).
    pub fn push_identifier(&mut self, name: &str) -> &mut Self {
        self.parts.push(quote_identifier(name));
        self
    }

    /// Push a separator between parts.
    pub fn push_sep(&mut self, sep: &str) -> &mut Self {
        self.parts.push(sep.to_string());
        self
    }

    /// Build the final SQL string and parameters.
    pub fn build(self) -> (String, Vec<FilterValue>) {
        (self.parts.join(""), self.params)
    }

    /// Get the current SQL string (without consuming).
    pub fn sql(&self) -> String {
        self.parts.join("")
    }

    /// Get the current parameters.
    pub fn params(&self) -> &[FilterValue] {
        &self.params
    }

    /// Get the next parameter index.
    pub fn next_param_index(&self) -> usize {
        self.params.len() + 1
    }
}

impl Default for SqlBuilder {
    fn default() -> Self {
        Self::postgres()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_identifier() {
        assert_eq!(escape_identifier("user"), "\"user\"");
        assert_eq!(escape_identifier("my_table"), "\"my_table\"");
        assert_eq!(escape_identifier("has\"quote"), "\"has\"\"quote\"");
    }

    #[test]
    fn test_needs_quoting() {
        assert!(needs_quoting("user"));
        assert!(needs_quoting("order"));
        assert!(needs_quoting("has space"));
        assert!(!needs_quoting("my_table"));
        assert!(!needs_quoting("users"));
    }

    #[test]
    fn test_quote_identifier() {
        assert_eq!(quote_identifier("user"), "\"user\"");
        assert_eq!(quote_identifier("my_table"), "my_table");
    }

    #[test]
    fn test_database_placeholder() {
        assert_eq!(DatabaseType::PostgreSQL.placeholder(1), "$1");
        assert_eq!(DatabaseType::PostgreSQL.placeholder(5), "$5");
        assert_eq!(DatabaseType::MySQL.placeholder(1), "?");
        assert_eq!(DatabaseType::SQLite.placeholder(1), "?");
    }

    #[test]
    fn test_sql_builder() {
        let mut builder = SqlBuilder::postgres();
        builder
            .push("SELECT * FROM ")
            .push_identifier("user")
            .push(" WHERE ")
            .push_identifier("id")
            .push(" = ")
            .push_param(42i32);

        let (sql, params) = builder.build();
        assert_eq!(sql, "SELECT * FROM \"user\" WHERE id = $1");
        assert_eq!(params.len(), 1);
    }
}

