//! Tenant middleware for automatic query filtering.

use super::config::TenantConfig;
use super::context::TenantContext;
use super::strategy::ColumnType;
use super::task_local;
use crate::error::{QueryError, QueryResult};
use crate::middleware::{BoxFuture, Middleware, Next, QueryContext, QueryResponse, QueryType};
use std::sync::{Arc, RwLock};

/// Middleware that automatically applies tenant filtering to queries.
pub struct TenantMiddleware {
    config: TenantConfig,
    current_tenant: Arc<RwLock<Option<TenantContext>>>,
}

impl TenantMiddleware {
    /// Create a new tenant middleware with the given config.
    pub fn new(config: TenantConfig) -> Self {
        Self {
            config,
            current_tenant: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the current tenant context.
    ///
    /// This writes a process-wide slot shared by every clone of this
    /// middleware, so concurrent requests would overwrite each other's
    /// tenant. It is a single-threaded escape hatch — prefer
    /// `task_local::with_tenant`, which this middleware resolves per query
    /// before falling back to this slot.
    pub fn set_tenant(&self, ctx: TenantContext) {
        *self.current_tenant.write().expect("lock poisoned") = Some(ctx);
    }

    /// Clear the current tenant context.
    pub fn clear_tenant(&self) {
        *self.current_tenant.write().expect("lock poisoned") = None;
    }

    /// Get the current tenant context.
    pub fn current_tenant(&self) -> Option<TenantContext> {
        self.current_tenant.read().expect("lock poisoned").clone()
    }

    /// Create a scoped tenant context (automatically clears on drop).
    pub fn scoped(&self, ctx: TenantContext) -> TenantScope {
        self.set_tenant(ctx);
        TenantScope {
            middleware: Arc::new(self.clone()),
        }
    }

    /// Apply row-level filtering to a SQL query.
    fn apply_row_level_filter(&self, sql: &str, tenant_id: &str) -> QueryResult<String> {
        let config = match self.config.row_level_config() {
            Some(c) => c,
            None => return Ok(sql.to_string()),
        };

        let tenant_value = validated_tenant_value(&config.column, config.column_type, tenant_id)?;

        // Parse and modify SQL
        self.inject_tenant_filter(sql, &config.column, &tenant_value)
    }

    /// Inject tenant filter into SQL.
    fn inject_tenant_filter(&self, sql: &str, column: &str, value: &str) -> QueryResult<String> {
        let filter = format!("{} = {}", column, value);
        let body = sql.trim();
        if body.is_empty() {
            return Ok(sql.to_string());
        }

        match classify_statement(body) {
            // SELECT / UPDATE / DELETE (including CTE-wrapped writes):
            // inject the filter into the top-level WHERE clause.
            shape @ (StatementShape::Select | StatementShape::Write) => {
                // Tolerate a single trailing semicolon when rewriting.
                let (body, semi) = match body.strip_suffix(';') {
                    Some(b) => (b.trim_end(), ";"),
                    None => (body, ""),
                };
                let rewritten = if shape == StatementShape::Select {
                    inject_where_filter(
                        body,
                        &filter,
                        &[
                            "GROUP BY", "HAVING", "WINDOW", "ORDER BY", "LIMIT", "OFFSET", "FETCH",
                        ],
                        &["UNION", "INTERSECT", "EXCEPT", "FOR"],
                    )?
                } else {
                    inject_where_filter(body, &filter, &["RETURNING"], &[])?
                };
                Ok(format!("{}{}", rewritten, semi))
            }

            // INSERT: add the tenant column/value when configured to do so.
            StatementShape::Insert => {
                if self
                    .config
                    .row_level_config()
                    .is_some_and(|c| c.auto_insert)
                {
                    inject_insert_column(body, column, value)
                } else {
                    Ok(sql.to_string())
                }
            }

            // WITH … SELECT: tenant-scoping a CTE requires rewriting inside
            // subqueries — refuse loudly rather than filter incompletely.
            StatementShape::WithSelect => Err(QueryError::invalid_input(
                "sql",
                "cannot safely apply a tenant filter to a CTE-wrapped SELECT statement",
            )),

            // Anything else (DDL, transactions, MERGE, REPLACE INTO, …):
            // fail closed instead of passing through unfiltered.
            StatementShape::Other => Err(QueryError::invalid_input(
                "sql",
                "cannot safely apply a tenant filter: unrecognized statement shape",
            )),
        }
    }

    /// Apply schema-based isolation.
    fn apply_schema_isolation(&self, tenant_id: &str) -> Option<String> {
        self.config
            .schema_config()
            .map(|c| c.search_path(tenant_id))
    }
}

impl Clone for TenantMiddleware {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            current_tenant: Arc::clone(&self.current_tenant),
        }
    }
}

impl std::fmt::Debug for TenantMiddleware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TenantMiddleware")
            .field("config", &self.config)
            .field("has_tenant", &self.current_tenant().is_some())
            .finish()
    }
}

impl Middleware for TenantMiddleware {
    fn handle<'a>(
        &'a self,
        mut ctx: QueryContext,
        next: Next<'a>,
    ) -> BoxFuture<'a, QueryResult<QueryResponse>> {
        Box::pin(async move {
            // Resolve the tenant for this query: the task-local context (set
            // per request via `task_local::with_tenant`) takes precedence; the
            // middleware-wide slot is a backward-compatible fallback.
            let tenant_ctx = match task_local::current_tenant().or_else(|| self.current_tenant()) {
                Some(ctx) => ctx,
                None => {
                    // No tenant context
                    if self.config.require_tenant {
                        if let Some(default) = &self.config.default_tenant {
                            TenantContext::new(default.clone())
                        } else {
                            return Err(QueryError::internal(
                                "Tenant context required but not provided",
                            ));
                        }
                    } else {
                        // No tenant filtering
                        return next.run(ctx).await;
                    }
                }
            };

            // Check for bypass
            if self.config.allow_bypass && tenant_ctx.should_bypass() {
                if self.config.log_tenant_context {
                    tracing::debug!(
                        tenant_id = %tenant_ctx.id,
                        bypass = true,
                        "Tenant filter bypassed"
                    );
                }
                return next.run(ctx).await;
            }

            // Apply row-level filtering if configured
            if self.config.strategy.is_row_level() {
                let query_type = ctx.query_type();

                // Apply filter to query
                let modified_sql =
                    self.apply_row_level_filter(ctx.sql(), tenant_ctx.id.as_str())?;

                // Update context with modified SQL
                ctx = ctx.with_sql(modified_sql);

                // Enforce tenant scoping on writes: with write validation
                // enabled, an UPDATE/DELETE that still does not reference the
                // tenant column after rewriting is rejected loudly instead of
                // running unscoped. CTE-wrapped writes (`WITH … UPDATE/DELETE`)
                // are classified as `Unknown` by `QueryType::from_sql`, so the
                // rewritten statement is classified here as well to keep the
                // guard firing for them.
                let is_write = matches!(query_type, QueryType::Update | QueryType::Delete)
                    || classify_statement(ctx.sql()) == StatementShape::Write;
                if self.config.enforce_on_writes
                    && is_write
                    && let Some(row_level) = self.config.row_level_config()
                    && row_level.validate_writes
                    && !references_identifier(ctx.sql(), &row_level.column)
                {
                    return Err(QueryError::invalid_input(
                        &row_level.column,
                        "UPDATE/DELETE does not reference the tenant column",
                    ));
                }
            }

            // Apply schema-based isolation if configured
            if self.config.strategy.is_schema_based()
                && let Some(search_path) = self.apply_schema_isolation(tenant_ctx.id.as_str())
            {
                // The search_path should be set on the connection
                // This is typically done by the connection manager
                ctx.metadata_mut().set_schema_override(Some(
                    self.config
                        .schema_config()
                        .unwrap()
                        .schema_name(tenant_ctx.id.as_str()),
                ));

                // Log the schema setting
                if self.config.log_tenant_context {
                    tracing::debug!(
                        tenant_id = %tenant_ctx.id,
                        search_path = %search_path,
                        "Setting schema for tenant"
                    );
                }
            }

            // Log tenant context
            if self.config.log_tenant_context {
                tracing::debug!(
                    tenant_id = %tenant_ctx.id,
                    strategy = ?self.config.strategy,
                    sql = %ctx.sql(),
                    "Executing query with tenant context"
                );
            }

            // Set tenant in metadata for downstream middleware
            ctx.metadata_mut().tenant_id = Some(tenant_ctx.id.to_string());

            // Continue with modified query
            next.run(ctx).await
        })
    }

    fn name(&self) -> &'static str {
        "TenantMiddleware"
    }
}

// ============================================================================
// SQL rewriting helpers
// ============================================================================
//
// The middleware rewrites SQL textually — parameter binding is not possible
// here without changing public signatures — so the tenant VALUE is always
// validated against the declared column type and escaped (see
// `validated_tenant_value`), and every rewrite below is conservative: a
// statement whose shape is not recognized is rejected with an error instead
// of being passed through silently or emitting invalid SQL.
//
// The scanner understands string literals ('...'), PostgreSQL dollar-quoted
// strings ($$…$$ / $tag$…$tag$), quoted identifiers ("...", `...`, [...]),
// line and (nested) block comments, and parenthesis depth. All byte offsets
// index the ORIGINAL string; case-insensitive keyword matching is done
// byte-wise so offsets never misalign on non-ASCII input (unlike matching
// against an uppercased copy of the SQL).

/// Validate `tenant_id` against the declared column type and return its safe
/// SQL literal form. Delegates to [`ColumnType::try_format_value`] — the
/// single source of truth for tenant value validation — and re-annotates the
/// error with the tenant column and the offending value.
fn validated_tenant_value(
    column: &str,
    column_type: ColumnType,
    tenant_id: &str,
) -> QueryResult<String> {
    column_type.try_format_value(tenant_id).map_err(|_| {
        let expectation = match column_type {
            ColumnType::String => "a string tenant id matching ^[A-Za-z0-9_:-.@]+$",
            ColumnType::Uuid => "a valid UUID",
            ColumnType::Integer | ColumnType::BigInt => "a valid integer",
        };
        QueryError::invalid_input(
            column,
            format!("tenant id is not {expectation}: {tenant_id:?}"),
        )
    })
}

/// Inject `filter` into the WHERE clause of a SELECT/UPDATE/DELETE body.
///
/// `terminators` are the clause keywords that may follow the WHERE clause —
/// the filter is inserted before the first of them (so a WHERE never lands
/// after GROUP BY/HAVING). `forbidden` are keywords whose presence makes the
/// statement unsafe to rewrite textually (compound SELECTs, locking reads);
/// such statements are rejected.
fn inject_where_filter(
    sql: &str,
    filter: &str,
    terminators: &[&str],
    forbidden: &[&str],
) -> QueryResult<String> {
    if find_top_level_keyword(sql, forbidden).is_some() {
        return Err(QueryError::invalid_input(
            "sql",
            "cannot safely apply a tenant filter to a compound or locking statement",
        ));
    }

    let where_hit = find_top_level_keyword(sql, &["WHERE"]);
    let terminator_hit = find_top_level_keyword(sql, terminators);

    match (where_hit, terminator_hit) {
        // A following-clause keyword before the WHERE — the statement shape
        // is not recognized, so reject rather than emit invalid SQL.
        (Some((where_start, _)), Some((term_start, _))) if term_start < where_start => {
            Err(QueryError::invalid_input(
                "sql",
                "cannot safely apply a tenant filter: unrecognized clause ordering",
            ))
        }
        // Existing WHERE: `… WHERE <filter> AND (<existing>) <rest>`. The
        // existing predicate is parenthesized so OR branches cannot bypass
        // the tenant filter.
        (Some((_, where_end)), terminator) => {
            let close = terminator.map_or(sql.len(), |(start, _)| start);
            let existing = sql[where_end..close].trim();
            if existing.is_empty() {
                return Err(QueryError::invalid_input(
                    "sql",
                    "cannot safely apply a tenant filter: empty WHERE clause",
                ));
            }
            Ok(format!(
                "{} {} AND ({}) {}",
                sql[..where_end].trim_end(),
                filter,
                existing,
                sql[close..].trim_start()
            ))
        }
        // No WHERE yet: add one before the next clause (GROUP BY / …) …
        (None, Some((term_start, _))) => Ok(format!(
            "{} WHERE {} {}",
            sql[..term_start].trim_end(),
            filter,
            sql[term_start..].trim_start()
        )),
        // … or at the end of the statement.
        (None, None) => Ok(format!("{} WHERE {}", sql.trim_end(), filter)),
    }
}

/// Add the tenant column/value to an INSERT of the shape
/// `INSERT INTO <table> (<cols>) VALUES (<exprs>) <rest>`.
///
/// The parse is deliberately conservative: statements without an explicit
/// column list, with a value/source other than a single `VALUES (...)` group,
/// with multi-row VALUES, or with a column/expression count mismatch are
/// rejected instead of passing through unscoped. If the column list already
/// contains the tenant column, the supplied value expression must be exactly
/// the current tenant's validated literal (after whitespace normalization) —
/// otherwise one tenant could write rows owned by another, so a mismatch is
/// rejected.
fn inject_insert_column(sql: &str, column: &str, value: &str) -> QueryResult<String> {
    let reject = || {
        QueryError::invalid_input(
            "sql",
            "cannot safely auto-insert the tenant column into this INSERT statement",
        )
    };

    let Some((_, into_end)) = find_top_level_keyword(sql, &["INTO"]) else {
        return Err(reject());
    };
    let name_start = skip_ws(sql, into_end);
    let Some(name_end) = parse_table_name(sql, name_start) else {
        return Err(reject());
    };

    let cols_open = skip_ws(sql, name_end);
    if sql.as_bytes().get(cols_open) != Some(&b'(') {
        // No explicit column list (VALUES/DEFAULT VALUES/SELECT directly).
        return Err(reject());
    }
    let Some(cols_close) = find_matching_paren(sql, cols_open) else {
        return Err(reject());
    };

    let columns = split_top_level_commas(&sql[cols_open + 1..cols_close]);
    if columns.iter().all(|c| c.trim().is_empty()) {
        return Err(reject());
    }
    let tenant_column_index = columns
        .iter()
        .position(|c| unquote_ident(c.trim()).eq_ignore_ascii_case(column));

    let after_cols = skip_ws(sql, cols_close + 1);
    let Some((_, values_end)) = match_keyword_at(sql, after_cols, &["VALUES"]) else {
        return Err(reject());
    };
    let vals_open = skip_ws(sql, values_end);
    if sql.as_bytes().get(vals_open) != Some(&b'(') {
        return Err(reject());
    }
    let Some(vals_close) = find_matching_paren(sql, vals_open) else {
        return Err(reject());
    };

    let values = split_top_level_commas(&sql[vals_open + 1..vals_close]);
    if values.len() != columns.len() {
        return Err(reject());
    }
    // Multi-row VALUES — too fragile to rewrite safely.
    let after_vals = skip_ws(sql, vals_close + 1);
    if sql.as_bytes().get(after_vals) == Some(&b',') {
        return Err(reject());
    }

    // The tenant column is already present: pass through only when the
    // supplied value is exactly the current tenant's validated literal.
    if let Some(index) = tenant_column_index {
        if !sql_literal_eq(values[index], value) {
            return Err(QueryError::invalid_input(
                column,
                "INSERT supplies a tenant column value that does not match the current tenant",
            ));
        }
        return Ok(sql.to_string());
    }

    Ok(format!(
        "{}, {}{}, {}{}",
        &sql[..cols_close],
        column,
        &sql[cols_close..vals_close],
        value,
        &sql[vals_close..]
    ))
}

/// Compare two SQL value expressions for exact equality after normalizing
/// whitespace (leading/trailing trimmed, internal runs collapsed to one
/// space).
fn sql_literal_eq(a: &str, b: &str) -> bool {
    a.split_ascii_whitespace().collect::<Vec<_>>().join(" ")
        == b.split_ascii_whitespace().collect::<Vec<_>>().join(" ")
}

/// The shape of a statement, for tenant-filter dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatementShape {
    /// `SELECT …`
    Select,
    /// `UPDATE`/`DELETE`, including CTE-wrapped writes
    /// (`WITH … UPDATE/DELETE`), which are rewritten and guarded exactly
    /// like plain writes.
    Write,
    /// `INSERT …`
    Insert,
    /// `WITH … SELECT` — rejected: tenant-scoping a CTE requires rewriting
    /// inside subqueries, so it is refused loudly rather than filtered
    /// incompletely.
    WithSelect,
    /// Anything else — rejected (fail closed).
    Other,
}

/// Classify a statement for tenant-filter dispatch, skipping leading
/// whitespace and line/block comments before the first keyword and looking
/// through a `WITH …` prefix to the body keyword.
fn classify_statement(sql: &str) -> StatementShape {
    let pos = skip_ws(sql, 0);
    if match_keyword_at(sql, pos, &["SELECT"]).is_some() {
        StatementShape::Select
    } else if match_keyword_at(sql, pos, &["UPDATE", "DELETE"]).is_some() {
        StatementShape::Write
    } else if match_keyword_at(sql, pos, &["INSERT"]).is_some() {
        StatementShape::Insert
    } else if let Some((_, with_end)) = match_keyword_at(sql, pos, &["WITH"]) {
        match cte_body_offset(sql, with_end) {
            Some(body) if match_keyword_at(sql, body, &["SELECT"]).is_some() => {
                StatementShape::WithSelect
            }
            Some(body) if match_keyword_at(sql, body, &["UPDATE", "DELETE"]).is_some() => {
                StatementShape::Write
            }
            _ => StatementShape::Other,
        }
    } else {
        StatementShape::Other
    }
}

/// Given `pos` just past the `WITH` keyword, parse the CTE list
/// (`name [(columns)] AS (<subquery>)` entries separated by commas) and
/// return the offset where the statement body starts. Returns `None` when
/// the CTE list cannot be parsed conservatively.
fn cte_body_offset(sql: &str, mut pos: usize) -> Option<usize> {
    let bytes = sql.as_bytes();
    // Optional RECURSIVE modifier.
    pos = skip_ws(sql, pos);
    if let Some((_, end)) = match_keyword_at(sql, pos, &["RECURSIVE"]) {
        pos = end;
    }
    loop {
        // CTE name: a bare or quoted identifier.
        pos = skip_ws(sql, pos);
        match bytes.get(pos) {
            Some(b'"') | Some(b'`') | Some(b'[') => pos = skip_quoted(sql, pos),
            Some(b) if b.is_ascii_alphabetic() || *b == b'_' => {
                pos += 1;
                while pos < bytes.len() && is_ident_byte(bytes[pos]) {
                    pos += 1;
                }
            }
            _ => return None,
        }
        pos = skip_ws(sql, pos);
        // Optional column list.
        if bytes.get(pos) == Some(&b'(') {
            pos = find_matching_paren(sql, pos)? + 1;
            pos = skip_ws(sql, pos);
        }
        // AS keyword, then the parenthesized subquery.
        let (_, as_end) = match_keyword_at(sql, pos, &["AS"])?;
        pos = skip_ws(sql, as_end);
        if bytes.get(pos) != Some(&b'(') {
            return None;
        }
        pos = find_matching_paren(sql, pos)? + 1;
        pos = skip_ws(sql, pos);
        match bytes.get(pos) {
            Some(&b',') => pos += 1,
            _ => return Some(pos),
        }
    }
}

/// Identifier characters for word-boundary checks.
fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

/// If a line (`--`) or block (`/* */`) comment starts at `pos`, return the
/// offset just past it. Block comments nest (PostgreSQL rules), so text
/// after an inner `*/` but still inside the outer comment is skipped as
/// comment. On dialects without nesting the worst case is extra skipped
/// text, which surfaces as a database syntax error — fail closed.
fn skip_comment(sql: &str, pos: usize) -> Option<usize> {
    let bytes = sql.as_bytes();
    if pos + 1 < bytes.len() && bytes[pos] == b'-' && bytes[pos + 1] == b'-' {
        let mut i = pos + 2;
        while i < bytes.len() && bytes[i] != b'\n' {
            i += 1;
        }
        Some(i)
    } else if pos + 1 < bytes.len() && bytes[pos] == b'/' && bytes[pos + 1] == b'*' {
        let mut depth = 1usize;
        let mut i = pos + 2;
        while i + 1 < bytes.len() && depth > 0 {
            if bytes[i] == b'/' && bytes[i + 1] == b'*' {
                depth += 1;
                i += 2;
            } else if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                depth -= 1;
                i += 2;
            } else {
                i += 1;
            }
        }
        Some(i.min(bytes.len()))
    } else {
        None
    }
}

/// Skip whitespace and comments starting at byte offset `pos`.
fn skip_ws(sql: &str, mut pos: usize) -> usize {
    let bytes = sql.as_bytes();
    loop {
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        match skip_comment(sql, pos) {
            Some(next) => pos = next,
            None => return pos,
        }
    }
}

/// Return the offset just past the string literal / quoted identifier that
/// opens at `pos` (`sql` must start with a quote byte there). Handles
/// doubled-quote escapes (`''`, `""`, and `]]` — the last being exactly
/// T-SQL's bracket escape). Returns `sql.len()` if unterminated.
fn skip_quoted(sql: &str, pos: usize) -> usize {
    let bytes = sql.as_bytes();
    let close = match bytes[pos] {
        b'[' => b']',
        quote => quote,
    };
    let mut i = pos + 1;
    while i < bytes.len() {
        if bytes[i] == close {
            if i + 1 < bytes.len() && bytes[i + 1] == close {
                i += 2; // doubled-quote escape
                continue;
            }
            return i + 1;
        }
        i += 1;
    }
    bytes.len()
}

/// If a PostgreSQL dollar-quoted string (`$$…$$`, `$tag$…$tag$`) opens at
/// `pos`, return the offset just past its closing delimiter. Returns `None`
/// when the `$` does not open a dollar quote (e.g. a `$1` parameter
/// placeholder). Dollar quotes never nest and the closing tag must match
/// exactly; an unterminated quote consumes to end of input, matching
/// `skip_quoted`'s fail-at-the-database stance.
fn skip_dollar_quoted(sql: &str, pos: usize) -> Option<usize> {
    let bytes = sql.as_bytes();
    if bytes.get(pos) != Some(&b'$') {
        return None;
    }
    // The tag follows unquoted-identifier rules (no leading digit), so a
    // digit right after `$` means a parameter placeholder, not a quote.
    if bytes.get(pos + 1).is_some_and(u8::is_ascii_digit) {
        return None;
    }
    let mut i = pos + 1;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }
    if bytes.get(i) != Some(&b'$') {
        return None;
    }
    let tag = &sql[pos..=i];
    match sql[i + 1..].find(tag) {
        Some(off) => Some(i + 1 + off + tag.len()),
        None => Some(bytes.len()),
    }
}

/// Try to match one of `keywords` (ASCII, possibly multi-word like
/// "GROUP BY") at byte offset `pos`, case-insensitively. Words may be
/// separated by any run of whitespace/comments. Returns the matched keyword
/// and the offset just past its last word.
fn match_keyword_at<'k>(sql: &str, pos: usize, keywords: &[&'k str]) -> Option<(&'k str, usize)> {
    let bytes = sql.as_bytes();
    'keywords: for keyword in keywords {
        let mut p = pos;
        for (index, word) in keyword.split_ascii_whitespace().enumerate() {
            if index > 0 {
                p = skip_ws(sql, p);
            }
            let end = p + word.len();
            if end > bytes.len() || !bytes[p..end].eq_ignore_ascii_case(word.as_bytes()) {
                continue 'keywords;
            }
            // Must not be glued to an identifier character ("ORDERS" ≠ "ORDER").
            if end < bytes.len() && is_ident_byte(bytes[end]) {
                continue 'keywords;
            }
            p = end;
        }
        return Some((keyword, p));
    }
    None
}

/// Find the first occurrence of any of `keywords` at parenthesis depth 0,
/// outside string literals, quoted identifiers and comments. Returns the
/// start offset of the match and the offset just past it; both index the
/// original string.
fn find_top_level_keyword(sql: &str, keywords: &[&str]) -> Option<(usize, usize)> {
    if keywords.is_empty() {
        return None;
    }
    let bytes = sql.as_bytes();
    let mut depth = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if matches!(b, b'\'' | b'"' | b'`' | b'[') {
            i = skip_quoted(sql, i);
        } else if b == b'$' {
            i = skip_dollar_quoted(sql, i).unwrap_or(i + 1);
        } else if let Some(next) = skip_comment(sql, i) {
            i = next;
        } else if b == b'(' {
            depth += 1;
            i += 1;
        } else if b == b')' {
            depth = depth.saturating_sub(1);
            i += 1;
        } else if depth == 0 && b.is_ascii_alphabetic() && (i == 0 || !is_ident_byte(bytes[i - 1]))
        {
            if let Some((_, end)) = match_keyword_at(sql, i, keywords) {
                return Some((i, end));
            }
            i += 1;
        } else {
            i += 1;
        }
    }
    None
}

/// Given `open` pointing at '(', return the offset of its matching ')',
/// skipping literals and comments.
fn find_matching_paren(sql: &str, open: usize) -> Option<usize> {
    let bytes = sql.as_bytes();
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        let b = bytes[i];
        if matches!(b, b'\'' | b'"' | b'`' | b'[') {
            i = skip_quoted(sql, i);
        } else if b == b'$' {
            i = skip_dollar_quoted(sql, i).unwrap_or(i + 1);
        } else if let Some(next) = skip_comment(sql, i) {
            i = next;
        } else if b == b'(' {
            depth += 1;
            i += 1;
        } else if b == b')' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
            i += 1;
        } else {
            i += 1;
        }
    }
    None
}

/// Split a comma-separated list at parenthesis depth 0, ignoring commas
/// inside literals, quoted identifiers and comments.
fn split_top_level_commas(list: &str) -> Vec<&str> {
    let bytes = list.as_bytes();
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if matches!(b, b'\'' | b'"' | b'`' | b'[') {
            i = skip_quoted(list, i);
        } else if b == b'$' {
            i = skip_dollar_quoted(list, i).unwrap_or(i + 1);
        } else if let Some(next) = skip_comment(list, i) {
            i = next;
        } else if b == b'(' {
            depth += 1;
            i += 1;
        } else if b == b')' {
            depth = depth.saturating_sub(1);
            i += 1;
        } else if b == b',' && depth == 0 {
            parts.push(&list[start..i]);
            i += 1;
            start = i;
        } else {
            i += 1;
        }
    }
    parts.push(&list[start..]);
    parts
}

/// Parse a (possibly schema-qualified, possibly quoted) table name starting
/// at `pos`. Returns the offset just past the name.
fn parse_table_name(sql: &str, pos: usize) -> Option<usize> {
    let bytes = sql.as_bytes();
    let mut i = pos;
    loop {
        i = skip_ws(sql, i);
        match bytes.get(i) {
            Some(b'"') | Some(b'`') | Some(b'[') => i = skip_quoted(sql, i),
            Some(b) if b.is_ascii_alphabetic() || *b == b'_' => {
                while i < bytes.len() && is_ident_byte(bytes[i]) {
                    i += 1;
                }
            }
            _ => return None,
        }
        // Continue only for a dotted qualifier (schema.table).
        let next = skip_ws(sql, i);
        if bytes.get(next) == Some(&b'.') {
            i = next + 1;
        } else {
            return Some(i);
        }
    }
}

/// Strip one level of identifier quoting (`"x"`, `` `x` ``, `[x]`).
fn unquote_ident(ident: &str) -> &str {
    let bytes = ident.as_bytes();
    if ident.len() >= 2 {
        let (first, last) = (bytes[0], bytes[ident.len() - 1]);
        if (first == b'"' && last == b'"')
            || (first == b'`' && last == b'`')
            || (first == b'[' && last == b']')
        {
            return &ident[1..ident.len() - 1];
        }
    }
    ident
}

/// Check whether `ident` occurs in `sql` as an identifier — outside string
/// literals and comments — at any nesting depth. Quoted occurrences count.
fn references_identifier(sql: &str, ident: &str) -> bool {
    let bytes = sql.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\'' {
            i = skip_quoted(sql, i);
        } else if b == b'"' || b == b'`' || b == b'[' {
            let end = skip_quoted(sql, i);
            if end > i + 2 && bytes[i + 1..end - 1].eq_ignore_ascii_case(ident.as_bytes()) {
                return true;
            }
            i = end;
        } else if b == b'$' {
            // Dollar-quoted string content is literal text, not an identifier.
            i = skip_dollar_quoted(sql, i).unwrap_or(i + 1);
        } else if let Some(next) = skip_comment(sql, i) {
            i = next;
        } else if b.is_ascii_alphabetic() || b == b'_' {
            let start = i;
            while i < bytes.len() && is_ident_byte(bytes[i]) {
                i += 1;
            }
            if bytes[start..i].eq_ignore_ascii_case(ident.as_bytes()) {
                return true;
            }
        } else {
            i += 1;
        }
    }
    false
}

/// A scoped tenant context that clears on drop.
pub struct TenantScope {
    middleware: Arc<TenantMiddleware>,
}

impl Drop for TenantScope {
    fn drop(&mut self) {
        self.middleware.clear_tenant();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_level_filter_select() {
        let config = TenantConfig::row_level("tenant_id");
        let middleware = TenantMiddleware::new(config);

        let sql = middleware
            .apply_row_level_filter("SELECT * FROM users", "tenant-123")
            .unwrap();
        assert!(sql.contains("WHERE tenant_id = 'tenant-123'"));

        let sql = middleware
            .apply_row_level_filter("SELECT * FROM users WHERE active = true", "tenant-123")
            .unwrap();
        assert!(sql.contains("tenant_id = 'tenant-123' AND (active = true)"));
    }

    #[test]
    fn test_row_level_filter_update() {
        let config = TenantConfig::row_level("tenant_id");
        let middleware = TenantMiddleware::new(config);

        let sql = middleware
            .apply_row_level_filter("UPDATE users SET name = 'Bob'", "tenant-123")
            .unwrap();
        assert!(sql.contains("WHERE tenant_id = 'tenant-123'"));

        let sql = middleware
            .apply_row_level_filter("UPDATE users SET name = 'Bob' WHERE id = 1", "tenant-123")
            .unwrap();
        assert!(sql.contains("tenant_id = 'tenant-123' AND (id = 1)"));
    }

    #[test]
    fn test_row_level_filter_delete() {
        let config = TenantConfig::row_level("tenant_id");
        let middleware = TenantMiddleware::new(config);

        let sql = middleware
            .apply_row_level_filter("DELETE FROM users", "tenant-123")
            .unwrap();
        assert!(sql.contains("WHERE tenant_id = 'tenant-123'"));
    }

    #[test]
    fn test_tenant_scope() {
        let config = TenantConfig::row_level("tenant_id");
        let middleware = TenantMiddleware::new(config);

        {
            let _scope = middleware.scoped(TenantContext::new("tenant-123"));
            assert!(middleware.current_tenant().is_some());
            assert_eq!(
                middleware.current_tenant().unwrap().id.as_str(),
                "tenant-123"
            );
        }

        // Scope dropped, tenant cleared
        assert!(middleware.current_tenant().is_none());
    }

    #[test]
    fn test_integer_tenant_injection_rejected() {
        use super::super::strategy::{IsolationStrategy, RowLevelConfig};

        let mut config = TenantConfig::row_level("tenant_id");
        config.strategy = IsolationStrategy::RowLevel(
            RowLevelConfig::new("tenant_id").with_column_type(ColumnType::Integer),
        );
        let middleware = TenantMiddleware::new(config);

        // A malicious tenant id must be rejected, never interpolated.
        assert!(
            middleware
                .apply_row_level_filter("SELECT * FROM users", "1 OR true--")
                .is_err()
        );

        // A well-formed integer tenant id is still applied.
        let sql = middleware
            .apply_row_level_filter("SELECT * FROM users", "42")
            .unwrap();
        assert!(sql.contains("WHERE tenant_id = 42"));
    }

    #[test]
    fn test_or_predicate_is_parenthesized() {
        let config = TenantConfig::row_level("tenant_id");
        let middleware = TenantMiddleware::new(config);

        let sql = middleware
            .apply_row_level_filter(
                "SELECT * FROM users WHERE active = true OR admin = true",
                "tenant-123",
            )
            .unwrap();

        // The existing predicate is parenthesized so the OR branch cannot
        // bypass the tenant filter.
        assert!(sql.contains("tenant_id = 'tenant-123' AND (active = true OR admin = true)"));
    }

    #[test]
    fn test_group_by_where_placement() {
        let config = TenantConfig::row_level("tenant_id");
        let middleware = TenantMiddleware::new(config);

        // No WHERE: the injected clause must land before GROUP BY.
        let sql = middleware
            .apply_row_level_filter(
                "SELECT role, COUNT(*) FROM users GROUP BY role",
                "tenant-123",
            )
            .unwrap();
        let where_pos = sql.find("WHERE").expect("WHERE injected");
        let group_pos = sql.find("GROUP BY").expect("GROUP BY preserved");
        assert!(where_pos < group_pos, "unexpected SQL: {sql}");

        // Existing WHERE: the closing paren must land before GROUP BY.
        let sql = middleware
            .apply_row_level_filter(
                "SELECT role, COUNT(*) FROM users WHERE active = true GROUP BY role",
                "tenant-123",
            )
            .unwrap();
        assert!(sql.contains("tenant_id = 'tenant-123' AND (active = true) GROUP BY role"));
    }

    #[test]
    fn test_string_tenant_whitelist_enforced() {
        let config = TenantConfig::row_level("tenant_id");
        let middleware = TenantMiddleware::new(config);

        // Quote/backslash escape attempts (e.g. MySQL `\'`) and empty ids are
        // rejected outright instead of being escaped.
        for bad in [
            "' OR 1=1-- ",
            "\\' OR 1=1-- ",
            "tenant'; DROP TABLE users--",
            "a b",
            "",
        ] {
            assert!(
                middleware
                    .apply_row_level_filter("SELECT * FROM users", bad)
                    .is_err(),
                "tenant id {bad:?} must be rejected"
            );
        }

        // Letters, digits and `_`, `-`, `:`, `.`, `@` are accepted.
        for good in ["tenant-123", "a_b-c:d.e@f", "Tenant_01"] {
            let sql = middleware
                .apply_row_level_filter("SELECT * FROM users", good)
                .unwrap();
            assert!(
                sql.contains(&format!("tenant_id = '{good}'")),
                "unexpected SQL: {sql}"
            );
        }
    }

    #[test]
    fn test_cte_and_comment_prefixed_statements() {
        let config = TenantConfig::row_level("tenant_id");
        let middleware = TenantMiddleware::new(config);

        // WITH … SELECT is rejected loudly: scoping a CTE requires rewriting
        // inside subqueries.
        let err = middleware
            .apply_row_level_filter(
                "WITH active_users AS (SELECT * FROM users) SELECT * FROM active_users",
                "tenant-123",
            )
            .unwrap_err();
        assert!(
            err.to_string().contains("CTE-wrapped SELECT"),
            "unexpected error: {err}"
        );

        // Comment-prefixed SELECT is classified after skipping the comment.
        for sql in [
            "-- list users\nSELECT * FROM users",
            "/* audit */ SELECT * FROM users",
        ] {
            let rewritten = middleware
                .apply_row_level_filter(sql, "tenant-123")
                .unwrap();
            assert!(
                rewritten.contains("WHERE tenant_id = 'tenant-123'"),
                "unexpected SQL: {rewritten}"
            );
        }

        // WITH … UPDATE hits the write path: the filter is injected into the
        // top-level WHERE clause.
        let sql = middleware
            .apply_row_level_filter(
                "WITH t AS (SELECT 1) UPDATE users SET name = 'Bob'",
                "tenant-123",
            )
            .unwrap();
        assert!(
            sql.contains("WHERE tenant_id = 'tenant-123'"),
            "unexpected SQL: {sql}"
        );
    }

    #[test]
    fn test_unrecognized_statements_fail_closed() {
        let config = TenantConfig::row_level("tenant_id");
        let middleware = TenantMiddleware::new(config);

        // Unrecognized shapes are rejected instead of passing through
        // unfiltered.
        for sql in [
            "REPLACE INTO users (id) VALUES (1)",
            "MERGE INTO users USING src ON users.id = src.id WHEN MATCHED THEN UPDATE SET name = src.name",
            "INSERT INTO users SELECT * FROM staging",
            "VACUUM users",
        ] {
            assert!(
                middleware
                    .apply_row_level_filter(sql, "tenant-123")
                    .is_err(),
                "statement must be rejected: {sql}"
            );
        }
    }

    #[test]
    fn test_insert_existing_tenant_column_must_match() {
        let config = TenantConfig::row_level("tenant_id");
        let middleware = TenantMiddleware::new(config);

        // A pre-existing tenant column holding the current tenant's value
        // passes through (whitespace is normalized).
        for sql in [
            "INSERT INTO users (name, tenant_id) VALUES ('Bob', 'tenant-123')",
            "INSERT INTO users (name, tenant_id) VALUES ('Bob',   'tenant-123'  )",
        ] {
            let rewritten = middleware
                .apply_row_level_filter(sql, "tenant-123")
                .unwrap();
            assert_eq!(rewritten, sql);
        }

        // A different tenant's value is rejected: tenant A cannot write rows
        // owned by tenant B.
        assert!(
            middleware
                .apply_row_level_filter(
                    "INSERT INTO users (name, tenant_id) VALUES ('Bob', 'B')",
                    "tenant-123",
                )
                .is_err()
        );

        // Anything that isn't exactly the validated literal is rejected.
        assert!(
            middleware
                .apply_row_level_filter(
                    "INSERT INTO users (name, tenant_id) VALUES ('Bob', 'tenant-123' OR '1'='1')",
                    "tenant-123",
                )
                .is_err()
        );
    }

    #[test]
    fn test_insert_auto_injects_tenant_column() {
        let config = TenantConfig::row_level("tenant_id");
        let middleware = TenantMiddleware::new(config);

        let sql = middleware
            .apply_row_level_filter("INSERT INTO users (name) VALUES ('Bob')", "tenant-123")
            .unwrap();
        assert!(sql.contains("tenant_id"), "unexpected SQL: {sql}");
        assert!(sql.contains("'tenant-123'"), "unexpected SQL: {sql}");
    }

    #[test]
    fn test_bracket_doubled_escape_scans_as_one_identifier() {
        // `[a]]b]` is a single T-SQL identifier (`a]b`): the doubled `]]` is
        // T-SQL's escape, not the end of the quoted identifier.
        assert_eq!(skip_quoted("[a]]b]", 0), "[a]]b]".len());

        // A clause keyword inside a bracket-quoted identifier must not be
        // treated as a real clause boundary.
        let config = TenantConfig::row_level("tenant_id");
        let middleware = TenantMiddleware::new(config);
        let sql = middleware
            .apply_row_level_filter("SELECT * FROM [a]] WHERE b]", "tenant-123")
            .unwrap();
        assert!(
            sql.ends_with("WHERE tenant_id = 'tenant-123'"),
            "unexpected SQL: {sql}"
        );
    }

    #[test]
    fn test_dollar_quoted_strings_are_not_scanned() {
        let config = TenantConfig::row_level("tenant_id");
        let middleware = TenantMiddleware::new(config);

        // A clause keyword inside a tagged dollar-quoted literal (e.g. a
        // PostgreSQL function body) is not a clause boundary.
        let sql = middleware
            .apply_row_level_filter(
                "SELECT * FROM users WHERE note = $tag$x GROUP BY y$tag$",
                "tenant-123",
            )
            .unwrap();
        assert!(
            sql.contains("tenant_id = 'tenant-123' AND (note = $tag$x GROUP BY y$tag$)"),
            "unexpected SQL: {sql}"
        );

        // Empty-tag dollar quotes; the real LIMIT is still recognized.
        let sql = middleware
            .apply_row_level_filter("SELECT $$a LIMIT b$$ FROM users LIMIT 5", "tenant-123")
            .unwrap();
        assert_eq!(
            sql,
            "SELECT $$a LIMIT b$$ FROM users WHERE tenant_id = 'tenant-123' LIMIT 5"
        );

        // A WHERE inside a dollar-quoted literal doesn't count.
        let sql = middleware
            .apply_row_level_filter("SELECT $$x WHERE y$$ AS v FROM users", "tenant-123")
            .unwrap();
        assert_eq!(
            sql,
            "SELECT $$x WHERE y$$ AS v FROM users WHERE tenant_id = 'tenant-123'"
        );

        // `$1` is a parameter placeholder, not a dollar-quote opener.
        let sql = middleware
            .apply_row_level_filter("SELECT * FROM users WHERE id = $1", "tenant-123")
            .unwrap();
        assert!(
            sql.contains("tenant_id = 'tenant-123' AND (id = $1)"),
            "unexpected SQL: {sql}"
        );
    }

    #[test]
    fn test_nested_block_comments_are_skipped() {
        let config = TenantConfig::row_level("tenant_id");
        let middleware = TenantMiddleware::new(config);

        // PostgreSQL nests block comments: the `WHERE` after the inner
        // comment close is still comment text, not a clause.
        let sql = middleware
            .apply_row_level_filter(
                "SELECT * FROM users /* outer /* inner */ WHERE x */",
                "tenant-123",
            )
            .unwrap();
        assert_eq!(
            sql,
            "SELECT * FROM users /* outer /* inner */ WHERE x */ WHERE tenant_id = 'tenant-123'"
        );
    }

    #[test]
    fn test_window_and_fetch_terminate_where() {
        let config = TenantConfig::row_level("tenant_id");
        let middleware = TenantMiddleware::new(config);

        // The filter must land before the WINDOW clause, not after it.
        let sql = middleware
            .apply_row_level_filter(
                "SELECT row_number() OVER w FROM users WINDOW w AS (ORDER BY id)",
                "tenant-123",
            )
            .unwrap();
        assert_eq!(
            sql,
            "SELECT row_number() OVER w FROM users WHERE tenant_id = 'tenant-123' \
             WINDOW w AS (ORDER BY id)"
        );

        // … and before FETCH FIRST (standard-SQL row limiting).
        let sql = middleware
            .apply_row_level_filter("SELECT * FROM users FETCH FIRST 10 ROWS ONLY", "tenant-123")
            .unwrap();
        assert_eq!(
            sql,
            "SELECT * FROM users WHERE tenant_id = 'tenant-123' FETCH FIRST 10 ROWS ONLY"
        );
    }

    #[tokio::test]
    async fn test_task_local_tenant_resolution() {
        let config = TenantConfig::row_level("tenant_id");
        let middleware = TenantMiddleware::new(config);
        middleware.set_tenant(TenantContext::new("slot-tenant"));

        // Inside a task-local scope, the task-local tenant wins.
        task_local::with_tenant("task-tenant", async {
            let response = middleware
                .handle(
                    QueryContext::new("SELECT * FROM users", vec![]),
                    echo_next(),
                )
                .await
                .unwrap();
            let sql = response.data["sql"].as_str().unwrap();
            assert!(
                sql.contains("tenant_id = 'task-tenant'"),
                "unexpected SQL: {sql}"
            );
        })
        .await;

        // Outside a task-local scope the shared slot is used (backward compat).
        let response = middleware
            .handle(
                QueryContext::new("SELECT * FROM users", vec![]),
                echo_next(),
            )
            .await
            .unwrap();
        let sql = response.data["sql"].as_str().unwrap();
        assert!(
            sql.contains("tenant_id = 'slot-tenant'"),
            "unexpected SQL: {sql}"
        );
    }

    /// Terminal handler that echoes back the SQL it receives.
    fn echo_next<'a>() -> Next<'a> {
        Next {
            inner: Box::new(|ctx: QueryContext| {
                let sql = ctx.sql().to_string();
                Box::pin(async move {
                    Ok::<QueryResponse, QueryError>(QueryResponse::new(
                        serde_json::json!({ "sql": sql }),
                    ))
                })
            }),
        }
    }
}
