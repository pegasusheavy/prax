//! MySQL connection wrapper.

use mysql_async::prelude::*;
use mysql_async::{Conn, Params, Row, Value};
use tracing::debug;

use crate::error::{MysqlError, MysqlResult};

/// A wrapper around a MySQL connection.
pub struct MysqlConnection {
    conn: Conn,
}

impl MysqlConnection {
    /// Create a new connection wrapper.
    pub fn new(conn: Conn) -> Self {
        Self { conn }
    }

    /// Execute a query and return all rows.
    pub async fn query<T>(&mut self, query: &str) -> MysqlResult<Vec<T>>
    where
        T: FromRow + Send + 'static,
    {
        debug!(query = %query, "Executing query");
        let rows: Vec<T> = self.conn.query(query).await?;
        Ok(rows)
    }

    /// Execute a query with parameters and return all rows.
    pub async fn query_params<T, P>(&mut self, query: &str, params: P) -> MysqlResult<Vec<T>>
    where
        T: FromRow + Send + 'static,
        P: Into<Params> + Send,
    {
        debug!(query = %query, "Executing parameterized query");
        let rows: Vec<T> = self.conn.exec(query, params).await?;
        Ok(rows)
    }

    /// Execute a query and return a single row.
    pub async fn query_one<T>(&mut self, query: &str) -> MysqlResult<T>
    where
        T: FromRow + Send + 'static,
    {
        debug!(query = %query, "Executing query_one");
        let row: Option<T> = self.conn.query_first(query).await?;
        row.ok_or_else(|| MysqlError::query("expected one row, got none"))
    }

    /// Execute a query with parameters and return a single row.
    pub async fn query_one_params<T, P>(&mut self, query: &str, params: P) -> MysqlResult<T>
    where
        T: FromRow + Send + 'static,
        P: Into<Params> + Send,
    {
        debug!(query = %query, "Executing parameterized query_one");
        let row: Option<T> = self.conn.exec_first(query, params).await?;
        row.ok_or_else(|| MysqlError::query("expected one row, got none"))
    }

    /// Execute a query and return an optional row.
    pub async fn query_optional<T>(&mut self, query: &str) -> MysqlResult<Option<T>>
    where
        T: FromRow + Send + 'static,
    {
        debug!(query = %query, "Executing query_optional");
        let row: Option<T> = self.conn.query_first(query).await?;
        Ok(row)
    }

    /// Execute a query with parameters and return an optional row.
    pub async fn query_optional_params<T, P>(
        &mut self,
        query: &str,
        params: P,
    ) -> MysqlResult<Option<T>>
    where
        T: FromRow + Send + 'static,
        P: Into<Params> + Send,
    {
        debug!(query = %query, "Executing parameterized query_optional");
        let row: Option<T> = self.conn.exec_first(query, params).await?;
        Ok(row)
    }

    /// Execute a statement and return the number of affected rows.
    pub async fn execute(&mut self, query: &str) -> MysqlResult<u64> {
        debug!(query = %query, "Executing statement");
        self.conn.query_drop(query).await?;
        Ok(self.conn.affected_rows())
    }

    /// Execute a statement with parameters and return the number of affected rows.
    pub async fn execute_params<P>(&mut self, query: &str, params: P) -> MysqlResult<u64>
    where
        P: Into<Params> + Send,
    {
        debug!(query = %query, "Executing parameterized statement");
        self.conn.exec_drop(query, params).await?;
        Ok(self.conn.affected_rows())
    }

    /// Execute a statement and return the last insert ID.
    pub async fn execute_insert(&mut self, query: &str) -> MysqlResult<u64> {
        debug!(query = %query, "Executing insert");
        self.conn.query_drop(query).await?;
        Ok(self.conn.last_insert_id().unwrap_or(0))
    }

    /// Execute a statement with parameters and return the last insert ID.
    pub async fn execute_insert_params<P>(&mut self, query: &str, params: P) -> MysqlResult<u64>
    where
        P: Into<Params> + Send,
    {
        debug!(query = %query, "Executing parameterized insert");
        self.conn.exec_drop(query, params).await?;
        Ok(self.conn.last_insert_id().unwrap_or(0))
    }

    /// Execute raw SQL returning rows.
    pub async fn query_raw(&mut self, query: &str) -> MysqlResult<Vec<Row>> {
        debug!(query = %query, "Executing raw query");
        let rows: Vec<Row> = self.conn.query(query).await?;
        Ok(rows)
    }

    /// Execute raw SQL with parameters returning rows.
    pub async fn query_raw_params<P>(&mut self, query: &str, params: P) -> MysqlResult<Vec<Row>>
    where
        P: Into<Params> + Send,
    {
        debug!(query = %query, "Executing parameterized raw query");
        let rows: Vec<Row> = self.conn.exec(query, params).await?;
        Ok(rows)
    }

    /// Get a scalar value from a query.
    pub async fn query_scalar<T>(&mut self, query: &str) -> MysqlResult<T>
    where
        T: FromValue + Send,
    {
        debug!(query = %query, "Executing scalar query");
        let value: Option<Value> = self.conn.query_first(query).await?;
        match value {
            Some(v) => Ok(T::from_value(v)),
            None => Err(MysqlError::query("expected scalar value, got none")),
        }
    }

    /// Get a scalar value from a query with parameters.
    pub async fn query_scalar_params<T, P>(&mut self, query: &str, params: P) -> MysqlResult<T>
    where
        T: FromValue + Send,
        P: Into<Params> + Send,
    {
        debug!(query = %query, "Executing parameterized scalar query");
        let value: Option<Value> = self.conn.exec_first(query, params).await?;
        match value {
            Some(v) => Ok(T::from_value(v)),
            None => Err(MysqlError::query("expected scalar value, got none")),
        }
    }

    /// Get the inner connection.
    pub fn inner(&self) -> &Conn {
        &self.conn
    }

    /// Get the inner connection mutably.
    pub fn inner_mut(&mut self) -> &mut Conn {
        &mut self.conn
    }

    /// Consume and return the inner connection.
    pub fn into_inner(self) -> Conn {
        self.conn
    }
}
