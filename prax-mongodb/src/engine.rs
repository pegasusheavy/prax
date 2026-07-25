//! MongoDB query engine implementation.

use std::marker::PhantomData;

use bson::{Bson, Document, doc};
use futures::TryStreamExt;
use mongodb::Collection;
use prax_query::QueryResult;
use prax_query::filter::FilterValue;
use prax_query::traits::{BoxFuture, Model, QueryEngine};
use tracing::debug;

use crate::client::MongoClient;
use crate::error::MongoError;
use crate::types::{filter_value_to_bson, filter_value_to_bson_with_object_id};

/// MongoDB query engine that implements the Prax QueryEngine trait.
///
/// Note: MongoDB is a document database, so the SQL-oriented QueryEngine
/// trait methods are adapted to work with MongoDB operations.
#[derive(Clone)]
pub struct MongoEngine {
    client: MongoClient,
}

impl MongoEngine {
    /// Create a new MongoDB engine with the given client.
    pub fn new(client: MongoClient) -> Self {
        Self { client }
    }

    /// Get a reference to the client.
    pub fn client(&self) -> &MongoClient {
        &self.client
    }

    /// Get a typed collection for a model.
    pub fn collection<T>(&self) -> Collection<T>
    where
        T: Model + Send + Sync,
    {
        // Use the model's declared table name as the collection name,
        // consistent with every SQL engine. (`Model::TABLE_NAME` is a
        // required associated constant, always populated by codegen.)
        self.client.collection(T::TABLE_NAME)
    }

    /// Get a collection by explicit name.
    pub fn collection_by_name<T>(&self, name: &str) -> Collection<T>
    where
        T: Send + Sync,
    {
        self.client.collection(name)
    }

    /// Convert filter values to a MongoDB filter document.
    ///
    /// Accepts a JSON filter document, an empty string (explicit
    /// match-all), or the simple `field1 = $1 AND field2 = $2`
    /// fallback format. Only `=` equality is supported in the
    /// fallback: any segment that fails to parse — missing field,
    /// invalid parameter index, or a non-`=` operator such as `>`,
    /// `<`, `!=`, `>=`, `<=` — is an error, never silently dropped
    /// (a dropped condition would widen the filter, in the worst
    /// case to an accidental match-all).
    ///
    /// Field names in the fallback must be plain top-level
    /// identifiers: `$`-prefixed names (`$where`, `$regex`, `$expr`,
    /// … — operator injection / server-side JS) and `.` dot-notation
    /// paths are rejected with `invalid_input`, since this parser
    /// binds plain equality only. Binds for the `_id` field route
    /// through [`filter_value_to_bson_with_object_id`] so a 24-hex
    /// string matches the stored ObjectId.
    fn build_filter(sql: &str, params: &[FilterValue]) -> QueryResult<Document> {
        // For MongoDB, we expect the "sql" to actually be a JSON representation
        // of the filter document, or we parse it from a simple query format
        if sql.starts_with('{') {
            // JSON filter
            let filter: Document = serde_json::from_str(sql)
                .map_err(|e| MongoError::query(format!("invalid filter JSON: {}", e)))?;
            Ok(filter)
        } else if sql.is_empty() {
            // Empty filter - match all
            Ok(doc! {})
        } else {
            // Try to parse as a simple field=value format
            // For more complex queries, use the FilterBuilder
            let mut filter = Document::new();

            // Simple parsing: "field1=$1 AND field2=$2"
            for part in sql.split(" AND ") {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }

                let Some(eq_pos) = part.find('=') else {
                    return Err(MongoError::query(format!(
                        "unsupported filter segment `{part}`: \
                         only `field = value` equality is supported"
                    ))
                    .into());
                };
                let field = part[..eq_pos].trim();
                // `!=`, `>=`, `<=` all contain `=`; reject any segment
                // whose "field" still carries an operator character.
                if field.is_empty()
                    || field.ends_with('!')
                    || field.ends_with('>')
                    || field.ends_with('<')
                {
                    return Err(MongoError::query(format!(
                        "unsupported filter segment `{part}`: \
                         only `field = value` equality is supported"
                    ))
                    .into());
                }
                // Reject `$`-prefixed field names (`$where`, `$regex`,
                // `$expr`, …) — operator injection / server-side JS —
                // and `.` dot-notation paths: this parser binds plain
                // equality on top-level fields only.
                if field.starts_with('$') || field.contains('.') {
                    return Err(prax_query::QueryError::invalid_input(
                        field,
                        format!(
                            "invalid field name in filter segment `{part}`: \
                             `$` operators and `.` dot-notation are not supported"
                        ),
                    ));
                }
                let value_placeholder = part[eq_pos + 1..].trim();

                // Check if it's a parameter placeholder ($1, $2, etc.)
                if let Some(stripped) = value_placeholder.strip_prefix('$') {
                    let Ok(param_idx) = stripped.parse::<usize>() else {
                        return Err(MongoError::query(format!(
                            "invalid parameter placeholder `${stripped}` \
                             in filter segment `{part}`"
                        ))
                        .into());
                    };
                    if param_idx == 0 || param_idx > params.len() {
                        return Err(MongoError::query(format!(
                            "parameter index ${param_idx} out of range \
                             ({} params) in filter segment `{part}`",
                            params.len()
                        ))
                        .into());
                    }
                    let bson_value = bind_param_for_field(field, &params[param_idx - 1])?;
                    filter.insert(field, bson_value);
                } else {
                    // Direct value
                    filter.insert(field, value_placeholder);
                }
            }

            Ok(filter)
        }
    }
}

use crate::error::MongoResult;

impl QueryEngine for MongoEngine {
    fn query_many<T: Model + prax_query::row::FromRow + Send + 'static>(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> BoxFuture<'_, QueryResult<Vec<T>>> {
        let sql = sql.to_string();
        Box::pin(async move {
            debug!(filter = %sql, "Executing query_many");

            let filter = Self::build_filter(&sql, &params)?;

            let collection = self.client.collection_doc(T::TABLE_NAME);

            let cursor = collection
                .find(filter, None)
                .await
                .map_err(|e| prax_query::QueryError::database(e.to_string()))?;

            let docs: Vec<Document> = cursor
                .try_collect()
                .await
                .map_err(|e| prax_query::QueryError::database(e.to_string()))?;

            docs.iter()
                .map(|d| {
                    let row = crate::row_ref::BsonRowRef::new(d);
                    T::from_row(&row).map_err(|e| {
                        let msg = e.to_string();
                        prax_query::QueryError::deserialization(msg).with_source(e)
                    })
                })
                .collect()
        })
    }

    fn query_one<T: Model + prax_query::row::FromRow + Send + 'static>(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> BoxFuture<'_, QueryResult<T>> {
        let sql = sql.to_string();
        Box::pin(async move {
            debug!(filter = %sql, "Executing query_one");

            let filter = Self::build_filter(&sql, &params)?;

            let collection = self.client.collection_doc(T::TABLE_NAME);

            let doc = collection
                .find_one(filter, None)
                .await
                .map_err(|e| prax_query::QueryError::database(e.to_string()))?
                .ok_or_else(|| prax_query::QueryError::not_found(T::MODEL_NAME))?;

            let row = crate::row_ref::BsonRowRef::new(&doc);
            T::from_row(&row).map_err(|e| {
                let msg = e.to_string();
                prax_query::QueryError::deserialization(msg).with_source(e)
            })
        })
    }

    fn query_optional<T: Model + prax_query::row::FromRow + Send + 'static>(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> BoxFuture<'_, QueryResult<Option<T>>> {
        let sql = sql.to_string();
        Box::pin(async move {
            debug!(filter = %sql, "Executing query_optional");

            let filter = Self::build_filter(&sql, &params)?;

            let collection = self.client.collection_doc(T::TABLE_NAME);

            let doc = collection
                .find_one(filter, None)
                .await
                .map_err(|e| prax_query::QueryError::database(e.to_string()))?;

            match doc {
                Some(doc) => {
                    let row = crate::row_ref::BsonRowRef::new(&doc);
                    T::from_row(&row).map(Some).map_err(|e| {
                        let msg = e.to_string();
                        prax_query::QueryError::deserialization(msg).with_source(e)
                    })
                }
                None => Ok(None),
            }
        })
    }

    fn execute_insert<T: Model + prax_query::row::FromRow + Send + 'static>(
        &self,
        sql: &str,
        _params: Vec<FilterValue>,
    ) -> BoxFuture<'_, QueryResult<T>> {
        let sql = sql.to_string();
        Box::pin(async move {
            debug!(data = %sql, "Executing insert");

            let doc = build_insert_doc(&sql)?;

            let collection = self.client.collection_doc(T::TABLE_NAME);

            let result = collection
                .insert_one(doc.clone(), None)
                .await
                .map_err(|e| prax_query::QueryError::database(e.to_string()))?;

            // Re-fetch the inserted document keyed on the server-assigned
            // `_id` (or on the client-supplied `_id` if the caller set
            // one) so the return value is the actual persisted row,
            // including server-generated fields.
            let id_filter = bson::doc! { "_id": result.inserted_id };
            let inserted = collection
                .find_one(id_filter, None)
                .await
                .map_err(|e| prax_query::QueryError::database(e.to_string()))?
                .ok_or_else(|| prax_query::QueryError::not_found(T::MODEL_NAME))?;

            let row = crate::row_ref::BsonRowRef::new(&inserted);
            T::from_row(&row).map_err(|e| {
                let msg = e.to_string();
                prax_query::QueryError::deserialization(msg).with_source(e)
            })
        })
    }

    fn execute_update<T: Model + prax_query::row::FromRow + Send + 'static>(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> BoxFuture<'_, QueryResult<Vec<T>>> {
        let sql = sql.to_string();
        Box::pin(async move {
            debug!(data = %sql, "Executing update");

            // Only the WHERE body feeds the filter parser — feeding the
            // whole statement misparses SET assignments as filter fields
            // and degrades to a match-all filter, rewriting every
            // document in the collection.
            let filter = resolve_mutation_filter(&sql, &params, "update")?;
            let set_doc = build_set_doc(&sql, &params)
                .map_err(|e| prax_query::QueryError::database(e.to_string()))?;

            let collection = self.client.collection_doc(T::TABLE_NAME);

            // Mongo's `update_many` doesn't hand back the affected
            // documents, so we have to re-fetch. The original filter
            // still selects the same rows post-update (the SET can't
            // un-match them for the filters the Client API emits —
            // we set columns, not the filter columns). One update +
            // one find instead of three round-trips.
            if !set_doc.is_empty() {
                let update = doc! { "$set": set_doc };
                collection
                    .update_many(filter.clone(), update, None)
                    .await
                    .map_err(|e| prax_query::QueryError::database(e.to_string()))?;
            }

            let cursor = collection
                .find(filter, None)
                .await
                .map_err(|e| prax_query::QueryError::database(e.to_string()))?;
            let updated: Vec<Document> = cursor
                .try_collect()
                .await
                .map_err(|e| prax_query::QueryError::database(e.to_string()))?;

            updated
                .iter()
                .map(|d| {
                    let row = crate::row_ref::BsonRowRef::new(d);
                    T::from_row(&row).map_err(|e| {
                        let msg = e.to_string();
                        prax_query::QueryError::deserialization(msg).with_source(e)
                    })
                })
                .collect()
        })
    }

    fn execute_delete(
        &self,
        sql: &str,
        params: Vec<FilterValue>,
    ) -> BoxFuture<'_, QueryResult<u64>> {
        let sql = sql.to_string();
        Box::pin(async move {
            debug!(filter = %sql, "Executing delete");

            // The QueryEngine contract carries no model context here, so
            // the target collection is parsed from the statement itself.
            // If that fails, fail loudly rather than silently hitting a
            // hardcoded collection.
            let table = parse_from_table(&sql).ok_or_else(|| {
                prax_query::QueryError::invalid_input(
                    "delete",
                    format!("could not parse a collection name from DELETE statement `{sql}`"),
                )
            })?;
            let filter = resolve_mutation_filter(&sql, &params, "delete")?;

            let collection = self.client.collection_doc(table);

            let result = collection
                .delete_many(filter, None)
                .await
                .map_err(|e| prax_query::QueryError::database(e.to_string()))?;

            Ok(result.deleted_count)
        })
    }

    fn execute_raw(&self, sql: &str, _params: Vec<FilterValue>) -> BoxFuture<'_, QueryResult<u64>> {
        let sql = sql.to_string();
        Box::pin(async move {
            debug!(command = %sql, "Executing raw command");

            // For MongoDB, raw execution means running a database command
            let command: Document = serde_json::from_str(&sql)
                .map_err(|e| prax_query::QueryError::database(e.to_string()))?;

            let result = self
                .client
                .run_command(command)
                .await
                .map_err(|e| prax_query::QueryError::database(e.to_string()))?;

            // Every MongoDB command reply carries `ok` (1 success /
            // 0 failure); write-ish commands additionally report the
            // affected-document count in `n`. An unknown response shape
            // is an error — never fabricate a count.
            let ok = match result.get("ok") {
                Some(Bson::Double(ok)) => *ok != 0.0,
                Some(Bson::Int32(ok)) => *ok != 0,
                Some(Bson::Int64(ok)) => *ok != 0,
                _ => {
                    return Err(prax_query::QueryError::database(format!(
                        "command response is missing a usable `ok` field: {result:?}"
                    )));
                }
            };
            if !ok {
                let errmsg = result.get_str("errmsg").unwrap_or("unknown error");
                return Err(prax_query::QueryError::database(format!(
                    "command failed: {errmsg}"
                )));
            }

            match result.get("n") {
                Some(Bson::Int32(n)) => Ok(*n as u64),
                Some(Bson::Int64(n)) => Ok(*n as u64),
                Some(Bson::Double(n)) => Ok(*n as u64),
                Some(other) => Err(prax_query::QueryError::database(format!(
                    "command response `n` field has an unexpected type: {other:?}"
                ))),
                // Read-ish/admin commands (ping, buildInfo, …) affect no
                // documents.
                None => Ok(0),
            }
        })
    }

    fn count(&self, sql: &str, params: Vec<FilterValue>) -> BoxFuture<'_, QueryResult<u64>> {
        let sql = sql.to_string();
        Box::pin(async move {
            debug!(filter = %sql, "Executing count");

            // As with execute_delete, the target collection is parsed
            // from the statement (`SELECT COUNT(*) FROM <table> …`);
            // an unparseable statement is an error, never a silent
            // query against a hardcoded collection.
            let table = parse_from_table(&sql).ok_or_else(|| {
                prax_query::QueryError::invalid_input(
                    "count",
                    format!("could not parse a collection name from count statement `{sql}`"),
                )
            })?;
            let filter = resolve_mutation_filter(&sql, &params, "select")?;

            let collection = self.client.collection_doc(table);

            let count = collection
                .count_documents(filter, None)
                .await
                .map_err(|e| prax_query::QueryError::database(e.to_string()))?;

            Ok(count)
        })
    }
}

/// A typed query builder that uses the MongoDB engine.
pub struct MongoQueryBuilder<T: Model> {
    engine: MongoEngine,
    _marker: PhantomData<T>,
}

impl<T: Model> MongoQueryBuilder<T> {
    /// Create a new query builder.
    pub fn new(engine: MongoEngine) -> Self {
        Self {
            engine,
            _marker: PhantomData,
        }
    }

    /// Get the underlying engine.
    pub fn engine(&self) -> &MongoEngine {
        &self.engine
    }

    /// Get a typed collection for this model.
    pub fn collection(&self) -> Collection<T>
    where
        T: Send + Sync,
    {
        self.engine.collection::<T>()
    }
}

/// Bind a filter/SET parameter destined for `field`. Only the `_id`
/// field opts in to 24-hex-string → ObjectId coercion; every other
/// field binds the value verbatim, so a hex-looking string stored in a
/// genuine string column still matches as a string.
fn bind_param_for_field(field: &str, value: &FilterValue) -> MongoResult<Bson> {
    if field == "_id" {
        filter_value_to_bson_with_object_id(value)
    } else {
        filter_value_to_bson(value)
    }
}

/// Build the document to insert from the engine input. Only a JSON
/// document is accepted: positional params carry no field names, so
/// the previous fallback (fabricated `field0..N` keys) stored documents
/// bearing no relation to the model. Fail loudly instead.
fn build_insert_doc(sql: &str) -> QueryResult<Document> {
    if sql.starts_with('{') {
        serde_json::from_str(sql).map_err(|e| prax_query::QueryError::database(e.to_string()))
    } else {
        Err(prax_query::QueryError::invalid_input(
            "insert",
            "execute_insert expects a JSON document",
        ))
    }
}

/// Extract the top-level `WHERE` body from a SQL-ish statement
/// (`UPDATE … SET … WHERE <body>`, `DELETE FROM … WHERE <body>`,
/// `SELECT … FROM … WHERE <body>`). The body is cut at a trailing
/// `RETURNING` clause if one is present. Returns `None` when the
/// statement has no (or an empty) WHERE clause.
fn extract_where_clause(sql: &str) -> Option<&str> {
    let lower = sql.to_ascii_lowercase();
    let start = lower.find(" where ")? + " where ".len();
    let end = lower[start..]
        .find(" returning ")
        .map(|i| start + i)
        .unwrap_or(sql.len());
    let body = sql[start..end].trim();
    if body.is_empty() { None } else { Some(body) }
}

/// Extract the table/collection identifier following the top-level
/// `FROM` keyword in a SQL-ish statement (`DELETE FROM <t> …`,
/// `SELECT … FROM <t> …`). Accepts a bare identifier or one quoted
/// with double quotes, backticks, or square brackets, mirroring the
/// quoting styles the SQL dialects emit. Returns the unquoted name.
fn parse_from_table(sql: &str) -> Option<&str> {
    let lower = sql.to_ascii_lowercase();
    let from_start = lower.find(" from ")? + " from ".len();
    let token = sql[from_start..].split_whitespace().next()?;
    let name = token
        .trim_matches(|c| c == '"' || c == '`')
        .trim_start_matches('[')
        .trim_end_matches(']');
    if name.is_empty() { None } else { Some(name) }
}

/// Resolve the MongoDB filter for a mutation/count statement.
///
/// * `WHERE <body>` present → parse `<body>` (only `field = $n`
///   equality is supported by the fallback parser).
/// * No WHERE, and the statement begins with `keyword` (`update` /
///   `delete` / `select`) → explicit match-all, mirroring SQL
///   semantics.
/// * Otherwise the whole input is treated as a bare filter in the
///   mongo-native calling convention.
///
/// A non-empty input that parses to an EMPTY filter is refused with
/// `invalid_input`: a silently-widened filter would match (and
/// mutate) every document in the collection.
fn resolve_mutation_filter(
    sql: &str,
    params: &[FilterValue],
    keyword: &str,
) -> QueryResult<Document> {
    match extract_where_clause(sql) {
        Some(where_sql) => {
            let filter = MongoEngine::build_filter(where_sql, params)?;
            if filter.is_empty() {
                return Err(prax_query::QueryError::invalid_input(
                    keyword,
                    format!(
                        "WHERE clause `{where_sql}` produced an empty filter; \
                         refusing to match every document in the collection"
                    ),
                ));
            }
            Ok(filter)
        }
        None => {
            let trimmed = sql.trim_start();
            let first_word = trimmed.split_whitespace().next().unwrap_or("");
            if first_word.eq_ignore_ascii_case(keyword) {
                // e.g. `UPDATE … SET …` / `DELETE FROM …` with no
                // WHERE: a match-all is exactly what the statement
                // asks for.
                return Ok(Document::new());
            }
            // Bare filter input (mongo-native calling convention):
            // the whole string is the filter.
            let filter = MongoEngine::build_filter(sql, params)?;
            if filter.is_empty() && !trimmed.is_empty() {
                return Err(prax_query::QueryError::invalid_input(
                    keyword,
                    format!(
                        "filter `{sql}` produced an empty document; \
                         refusing to match every document in the collection"
                    ),
                ));
            }
            Ok(filter)
        }
    }
}

/// Parse `SET col1 = $1, col2 = $2` from a SQL-ish UPDATE statement
/// and bind each placeholder to the matching entry in `params`. Returns
/// a BSON `$set` document suitable for [`update_many`]. Tolerant of
/// an absent SET clause (returns empty doc, caller treats that as a
/// filter-only "update nothing" no-op).
fn build_set_doc(sql: &str, params: &[FilterValue]) -> MongoResult<Document> {
    // Locate the SET … WHERE window in the SQL string.
    let lower = sql.to_ascii_lowercase();
    let Some(set_start) = lower.find(" set ") else {
        return Ok(Document::new());
    };
    let set_body_start = set_start + " set ".len();
    let set_body_end = lower[set_body_start..]
        .find(" where ")
        .map(|i| set_body_start + i)
        .unwrap_or(sql.len());
    let body = &sql[set_body_start..set_body_end];

    let mut out = Document::new();
    for assignment in body.split(',') {
        let assignment = assignment.trim();
        let Some(eq) = assignment.find('=') else {
            continue;
        };
        let col = assignment[..eq].trim();
        let rhs = assignment[eq + 1..].trim();
        let Some(idx_str) = rhs.strip_prefix('$') else {
            continue;
        };
        let Ok(idx) = idx_str.parse::<usize>() else {
            continue;
        };
        if idx == 0 || idx > params.len() {
            continue;
        }
        let value = bind_param_for_field(col, &params[idx - 1])?;
        out.insert(col, value);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_filter_json() {
        let filter = MongoEngine::build_filter(r#"{"name": "Alice"}"#, &[]).unwrap();
        assert_eq!(filter.get_str("name").unwrap(), "Alice");
    }

    #[test]
    fn test_build_filter_empty() {
        let filter = MongoEngine::build_filter("", &[]).unwrap();
        assert!(filter.is_empty());
    }

    #[test]
    fn test_build_filter_simple_params() {
        let params = vec![
            FilterValue::String("Alice".to_string()),
            FilterValue::Int(30),
        ];
        let filter = MongoEngine::build_filter("name = $1 AND age = $2", &params).unwrap();
        assert_eq!(filter.get_str("name").unwrap(), "Alice");
        assert_eq!(filter.get_i64("age").unwrap(), 30);
    }

    #[test]
    fn test_build_filter_direct_value() {
        let filter = MongoEngine::build_filter("name = Alice", &[]).unwrap();
        assert_eq!(filter.get_str("name").unwrap(), "Alice");
    }

    #[test]
    fn test_build_filter_rejects_unparseable_segment() {
        let err = MongoEngine::build_filter("no operator here", &[]).unwrap_err();
        assert!(err.to_string().contains("only `field = value`"));
    }

    #[test]
    fn test_build_filter_rejects_non_equality_operators() {
        let params = vec![FilterValue::Int(30)];
        assert!(MongoEngine::build_filter("age > $1", &params).is_err());
        assert!(MongoEngine::build_filter("age != $1", &params).is_err());
        assert!(MongoEngine::build_filter("age >= $1", &params).is_err());
        assert!(MongoEngine::build_filter("age <= $1", &params).is_err());
    }

    #[test]
    fn test_build_filter_rejects_bad_param_index() {
        let params = vec![FilterValue::Int(1)];
        assert!(MongoEngine::build_filter("name = $x", &params).is_err());
        assert!(MongoEngine::build_filter("name = $0", &params).is_err());
        assert!(MongoEngine::build_filter("name = $2", &params).is_err());
    }

    #[test]
    fn test_build_filter_id_param_coerces_to_object_id() {
        let oid = bson::oid::ObjectId::new();
        let params = vec![FilterValue::String(oid.to_hex())];
        let filter = MongoEngine::build_filter("_id = $1", &params).unwrap();
        assert_eq!(filter.get("_id"), Some(&Bson::ObjectId(oid)));
    }

    #[test]
    fn test_build_filter_non_id_hex_string_stays_string() {
        let hex = bson::oid::ObjectId::new().to_hex();
        let params = vec![FilterValue::String(hex.clone())];
        let filter = MongoEngine::build_filter("name = $1", &params).unwrap();
        assert_eq!(filter.get("name"), Some(&Bson::String(hex)));
    }

    #[test]
    fn test_build_filter_rejects_operator_injection_field_names() {
        let params = vec![FilterValue::String("x".to_string())];
        // `$`-prefixed names: operator injection / server-side JS.
        assert!(MongoEngine::build_filter("$where = $1", &params).is_err());
        assert!(MongoEngine::build_filter("$expr = $1", &params).is_err());
        assert!(MongoEngine::build_filter("$where = 1", &[]).is_err());
        // Dot-notation paths: the parser is equality-only on
        // top-level fields.
        assert!(MongoEngine::build_filter("a.b = $1", &params).is_err());
        // Plain top-level field names still accepted.
        assert!(MongoEngine::build_filter("name = $1", &params).is_ok());
        assert!(MongoEngine::build_filter("_id = $1", &params).is_ok());
    }

    #[test]
    fn test_extract_where_clause() {
        assert_eq!(
            extract_where_clause("UPDATE users SET name = $1 WHERE id = $2"),
            Some("id = $2")
        );
        assert_eq!(
            extract_where_clause("DELETE FROM users WHERE id = $1 RETURNING *"),
            Some("id = $1")
        );
        assert_eq!(
            extract_where_clause("SELECT COUNT(*) FROM users WHERE active = $1"),
            Some("active = $1")
        );
        assert_eq!(extract_where_clause("UPDATE users SET name = $1"), None);
        assert_eq!(
            extract_where_clause("UPDATE users SET name = $1 WHERE "),
            None
        );
    }

    #[test]
    fn test_parse_from_table() {
        assert_eq!(
            parse_from_table("DELETE FROM users WHERE id = $1"),
            Some("users")
        );
        assert_eq!(
            parse_from_table("SELECT COUNT(*) FROM posts WHERE published = $1"),
            Some("posts")
        );
        assert_eq!(parse_from_table("DELETE FROM `users`"), Some("users"));
        assert_eq!(parse_from_table("DELETE FROM \"users\""), Some("users"));
        assert_eq!(parse_from_table("DELETE FROM [users]"), Some("users"));
        assert_eq!(parse_from_table("DELETE users"), None);
    }

    #[test]
    fn test_update_filter_matches_where_document_only() {
        // Regression test for the collection-wide rewrite: with two
        // documents (id 1 and id 2) and an UPDATE targeting id 2, the
        // filter must be built from the WHERE body only — the SET field
        // must not leak in — so update_many matches exactly the one
        // document with id = 2.
        let params = vec![
            FilterValue::String("new-name".to_string()),
            FilterValue::Int(2),
        ];
        let filter = resolve_mutation_filter(
            "UPDATE users SET name = $1 WHERE id = $2",
            &params,
            "update",
        )
        .unwrap();
        assert_eq!(filter.len(), 1);
        assert_eq!(filter.get_i64("id").unwrap(), 2);
        assert!(!filter.contains_key("name"));
    }

    #[test]
    fn test_resolve_mutation_filter_refuses_degenerate_filter() {
        // A non-empty input that parses down to an empty filter must
        // not degrade to a match-all.
        let err = resolve_mutation_filter(" AND ", &[], "update").unwrap_err();
        assert!(err.to_string().contains("refusing to match every document"));
    }

    #[test]
    fn test_resolve_mutation_filter_statement_without_where_is_match_all() {
        // `UPDATE … SET …` with no WHERE is an explicit match-all,
        // mirroring SQL semantics.
        let params = vec![FilterValue::String("x".to_string())];
        let filter =
            resolve_mutation_filter("UPDATE users SET name = $1", &params, "update").unwrap();
        assert!(filter.is_empty());
    }

    #[test]
    fn test_resolve_mutation_filter_bare_filter_convention() {
        // Mongo-native calling convention: the whole input is a filter.
        let params = vec![FilterValue::Int(2)];
        let filter = resolve_mutation_filter("id = $1", &params, "update").unwrap();
        assert_eq!(filter.get_i64("id").unwrap(), 2);
    }

    #[test]
    fn test_build_insert_doc_rejects_non_json() {
        let err = build_insert_doc("INSERT INTO users (name) VALUES ($1)").unwrap_err();
        assert!(err.to_string().contains("JSON"));
    }

    #[test]
    fn test_build_insert_doc_accepts_json() {
        let doc = build_insert_doc(r#"{"name": "Alice"}"#).unwrap();
        assert_eq!(doc.get_str("name").unwrap(), "Alice");
    }
}
