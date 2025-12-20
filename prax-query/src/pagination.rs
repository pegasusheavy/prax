//! Pagination types for query results.
//!
//! This module provides types for implementing both offset-based and cursor-based pagination.
//!
//! # Offset-Based Pagination (Skip/Take)
//!
//! Simple pagination using skip and take:
//!
//! ```rust
//! use prax_query::Pagination;
//!
//! // Skip 10, take 20
//! let pagination = Pagination::new()
//!     .skip(10)
//!     .take(20);
//!
//! assert_eq!(pagination.skip, Some(10));
//! assert_eq!(pagination.take, Some(20));
//! assert_eq!(pagination.to_sql(), "LIMIT 20 OFFSET 10");
//!
//! // First N records
//! let first_10 = Pagination::first(10);
//! assert_eq!(first_10.to_sql(), "LIMIT 10");
//!
//! // Page-based pagination (1-indexed)
//! let page_3 = Pagination::page(3, 25);  // Page 3 with 25 items per page
//! assert_eq!(page_3.skip, Some(50));   // Skip first 2 pages (50 items)
//! assert_eq!(page_3.take, Some(25));
//! ```
//!
//! # Checking Pagination State
//!
//! ```rust
//! use prax_query::Pagination;
//!
//! let empty = Pagination::new();
//! assert!(empty.is_empty());
//!
//! let with_limit = Pagination::new().take(10);
//! assert!(!with_limit.is_empty());
//! ```

use serde::{Deserialize, Serialize};
use std::fmt::Write;

/// Pagination configuration for queries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pagination {
    /// Number of records to skip.
    pub skip: Option<u64>,
    /// Maximum number of records to take.
    pub take: Option<u64>,
    /// Cursor for cursor-based pagination.
    pub cursor: Option<Cursor>,
}

impl Pagination {
    /// Create a new pagination with no limits.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the number of records to skip.
    pub fn skip(mut self, skip: u64) -> Self {
        self.skip = Some(skip);
        self
    }

    /// Set the maximum number of records to take.
    pub fn take(mut self, take: u64) -> Self {
        self.take = Some(take);
        self
    }

    /// Set cursor for cursor-based pagination.
    pub fn cursor(mut self, cursor: Cursor) -> Self {
        self.cursor = Some(cursor);
        self
    }

    /// Check if pagination is specified.
    pub fn is_empty(&self) -> bool {
        self.skip.is_none() && self.take.is_none() && self.cursor.is_none()
    }

    /// Generate SQL LIMIT/OFFSET clause.
    ///
    /// Optimized to avoid intermediate allocations by writing directly to a buffer.
    pub fn to_sql(&self) -> String {
        // Estimate capacity: "LIMIT " (6) + number (up to 20) + " OFFSET " (8) + number (up to 20)
        let mut sql = String::with_capacity(54);

        if let Some(take) = self.take {
            let _ = write!(sql, "LIMIT {}", take);
        }

        if let Some(skip) = self.skip {
            if !sql.is_empty() {
                sql.push(' ');
            }
            let _ = write!(sql, "OFFSET {}", skip);
        }

        sql
    }

    /// Write SQL LIMIT/OFFSET clause directly to a buffer (zero allocation).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use prax_query::Pagination;
    ///
    /// let pagination = Pagination::new().skip(10).take(20);
    /// let mut buffer = String::with_capacity(64);
    /// buffer.push_str("SELECT * FROM users ");
    /// pagination.write_sql(&mut buffer);
    /// assert!(buffer.ends_with("LIMIT 20 OFFSET 10"));
    /// ```
    #[inline]
    pub fn write_sql(&self, buffer: &mut String) {
        if let Some(take) = self.take {
            let _ = write!(buffer, "LIMIT {}", take);
        }

        if let Some(skip) = self.skip {
            if self.take.is_some() {
                buffer.push(' ');
            }
            let _ = write!(buffer, "OFFSET {}", skip);
        }
    }

    /// Get pagination for the first N records.
    pub fn first(n: u64) -> Self {
        Self::new().take(n)
    }

    /// Get pagination for a page (1-indexed).
    pub fn page(page: u64, page_size: u64) -> Self {
        let skip = (page.saturating_sub(1)) * page_size;
        Self::new().skip(skip).take(page_size)
    }
}

/// Cursor for cursor-based pagination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    /// The column to use for cursor.
    pub column: String,
    /// The cursor value.
    pub value: CursorValue,
    /// Direction of pagination.
    pub direction: CursorDirection,
}

impl Cursor {
    /// Create a new cursor.
    pub fn new(
        column: impl Into<String>,
        value: CursorValue,
        direction: CursorDirection,
    ) -> Self {
        Self {
            column: column.into(),
            value,
            direction,
        }
    }

    /// Create a cursor for fetching records after this value.
    pub fn after(column: impl Into<String>, value: impl Into<CursorValue>) -> Self {
        Self::new(column, value.into(), CursorDirection::After)
    }

    /// Create a cursor for fetching records before this value.
    pub fn before(column: impl Into<String>, value: impl Into<CursorValue>) -> Self {
        Self::new(column, value.into(), CursorDirection::Before)
    }

    /// Generate the WHERE clause for cursor-based pagination.
    ///
    /// Optimized to write directly to a pre-sized buffer.
    pub fn to_sql_condition(&self) -> String {
        // Estimate: column + " " + op + " $cursor" = column.len() + 10
        let mut sql = String::with_capacity(self.column.len() + 12);
        sql.push_str(&self.column);
        sql.push(' ');
        sql.push_str(match self.direction {
            CursorDirection::After => "> $cursor",
            CursorDirection::Before => "< $cursor",
        });
        sql
    }

    /// Write the cursor condition directly to a buffer (zero allocation).
    #[inline]
    pub fn write_sql_condition(&self, buffer: &mut String) {
        buffer.push_str(&self.column);
        buffer.push(' ');
        buffer.push_str(match self.direction {
            CursorDirection::After => "> $cursor",
            CursorDirection::Before => "< $cursor",
        });
    }

    /// Get the operator for this cursor direction.
    #[inline]
    pub const fn operator(&self) -> &'static str {
        match self.direction {
            CursorDirection::After => ">",
            CursorDirection::Before => "<",
        }
    }
}

/// Cursor value type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorValue {
    /// Integer cursor (e.g., auto-increment ID).
    Int(i64),
    /// String cursor (e.g., UUID).
    String(String),
}

impl From<i32> for CursorValue {
    fn from(v: i32) -> Self {
        Self::Int(v as i64)
    }
}

impl From<i64> for CursorValue {
    fn from(v: i64) -> Self {
        Self::Int(v)
    }
}

impl From<String> for CursorValue {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

impl From<&str> for CursorValue {
    fn from(v: &str) -> Self {
        Self::String(v.to_string())
    }
}

/// Direction for cursor-based pagination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CursorDirection {
    /// Fetch records after the cursor.
    After,
    /// Fetch records before the cursor.
    Before,
}

/// Result of a paginated query with metadata.
#[derive(Debug, Clone)]
pub struct PaginatedResult<T> {
    /// The query results.
    pub data: Vec<T>,
    /// Whether there are more records after these.
    pub has_next: bool,
    /// Whether there are more records before these.
    pub has_previous: bool,
    /// The cursor for the next page (last item's cursor).
    pub next_cursor: Option<CursorValue>,
    /// The cursor for the previous page (first item's cursor).
    pub previous_cursor: Option<CursorValue>,
    /// Total count (if requested).
    pub total_count: Option<u64>,
}

impl<T> PaginatedResult<T> {
    /// Create a new paginated result.
    pub fn new(data: Vec<T>) -> Self {
        Self {
            data,
            has_next: false,
            has_previous: false,
            next_cursor: None,
            previous_cursor: None,
            total_count: None,
        }
    }

    /// Set pagination metadata.
    pub fn with_pagination(
        mut self,
        has_next: bool,
        has_previous: bool,
    ) -> Self {
        self.has_next = has_next;
        self.has_previous = has_previous;
        self
    }

    /// Set total count.
    pub fn with_total(mut self, total: u64) -> Self {
        self.total_count = Some(total);
        self
    }

    /// Set cursors.
    pub fn with_cursors(
        mut self,
        next: Option<CursorValue>,
        previous: Option<CursorValue>,
    ) -> Self {
        self.next_cursor = next;
        self.previous_cursor = previous;
        self
    }

    /// Get the number of records in this result.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the result is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<T> IntoIterator for PaginatedResult<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_skip_take() {
        let pagination = Pagination::new().skip(10).take(20);
        assert_eq!(pagination.to_sql(), "LIMIT 20 OFFSET 10");
    }

    #[test]
    fn test_pagination_page() {
        let pagination = Pagination::page(3, 10);
        assert_eq!(pagination.skip, Some(20));
        assert_eq!(pagination.take, Some(10));
    }

    #[test]
    fn test_cursor_after() {
        let cursor = Cursor::after("id", 100i64);
        assert_eq!(cursor.to_sql_condition(), "id > $cursor");
    }

    #[test]
    fn test_cursor_before() {
        let cursor = Cursor::before("id", 100i64);
        assert_eq!(cursor.to_sql_condition(), "id < $cursor");
    }

    #[test]
    fn test_paginated_result() {
        let result = PaginatedResult::new(vec![1, 2, 3])
            .with_pagination(true, false)
            .with_total(100);

        assert_eq!(result.len(), 3);
        assert!(result.has_next);
        assert!(!result.has_previous);
        assert_eq!(result.total_count, Some(100));
    }
}

