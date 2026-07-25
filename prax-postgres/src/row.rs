//! PostgreSQL row types and deserialization.

use tokio_postgres::Row;

use crate::error::{PgError, PgResult};

/// Extension trait for PostgreSQL rows.
pub trait PgRow {
    /// Get a column value by name.
    fn get_value<T>(&self, column: &str) -> PgResult<T>
    where
        T: for<'a> tokio_postgres::types::FromSql<'a>;

    /// Get an optional column value by name.
    fn get_opt<T>(&self, column: &str) -> PgResult<Option<T>>
    where
        T: for<'a> tokio_postgres::types::FromSql<'a>;

    /// Try to get a column value, returning None if the column doesn't exist.
    fn try_get<T>(&self, column: &str) -> Option<T>
    where
        T: for<'a> tokio_postgres::types::FromSql<'a>;
}

impl PgRow for Row {
    fn get_value<T>(&self, column: &str) -> PgResult<T>
    where
        T: for<'a> tokio_postgres::types::FromSql<'a>,
    {
        self.try_get(column).map_err(|e| {
            PgError::deserialization(format!("failed to get column '{}': {}", column, e))
        })
    }

    fn get_opt<T>(&self, column: &str) -> PgResult<Option<T>>
    where
        T: for<'a> tokio_postgres::types::FromSql<'a>,
    {
        // Decoding through `Option<T>` makes SQL NULL yield `None` natively,
        // so any error here is a genuine failure (type mismatch, missing
        // column) and must propagate rather than be pattern-matched away.
        self.try_get(column).map_err(|e| {
            PgError::deserialization(format!("failed to get column '{}': {}", column, e))
        })
    }

    fn try_get<T>(&self, column: &str) -> Option<T>
    where
        T: for<'a> tokio_postgres::types::FromSql<'a>,
    {
        Row::try_get(self, column).ok()
    }
}

/// Trait for deserializing a PostgreSQL row into a type.
pub trait FromPgRow: Sized {
    /// Deserialize from a PostgreSQL row.
    fn from_row(row: &Row) -> PgResult<Self>;
}

/// Macro to implement FromPgRow for simple structs.
///
/// Usage:
/// ```rust,ignore
/// impl_from_row!(User {
///     id: i32,
///     email: String,
///     name: Option<String>,
/// });
/// ```
#[macro_export]
macro_rules! impl_from_row {
    ($type:ident { $($field:ident : $field_type:ty),* $(,)? }) => {
        impl $crate::row::FromPgRow for $type {
            fn from_row(row: &tokio_postgres::Row) -> $crate::error::PgResult<Self> {
                use $crate::row::PgRow;
                Ok(Self {
                    $(
                        $field: row.get_value(stringify!($field))?,
                    )*
                })
            }
        }
    };
}

#[cfg(test)]
mod tests {
    // Row tests require integration testing with a real database
}
