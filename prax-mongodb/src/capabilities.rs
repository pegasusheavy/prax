//! MongoDB engine capability declarations.
//!
//! MongoDB uses the document/aggregation model rather than SQL primitives, so
//! only a narrow subset of the SQL capability traits apply.
//!
//! `SupportsScalarSubqueryInSelect` is intentionally NOT impl'd here.
//! Relation-aggregate virtual fields require a `$lookup`-lowering pass
//! that is scheduled as a follow-up plan after phase 5.

use prax_query::capabilities::SupportsRelationFilter;

use crate::engine::MongoEngine;

impl SupportsRelationFilter for MongoEngine {}

// NOTE: `SupportsNestedWrites` is intentionally NOT impl'd here. Nested
// writes are unsupported: document-store engines inherit the panicking
// `NotSql` dialect default and are rejected by prax-query's nested-write
// executor, so the capability could never succeed at runtime. A future
// `$lookup`/embedded-document lowering could restore it.
