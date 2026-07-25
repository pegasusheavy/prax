//! Lower `order_by: { _sum: { views: desc }, team_id: asc }` in a
//! group_by! call to a Vec<OrderByField> token stream. Aggregate
//! orderings reference the SELECT-list alias emitted by
//! AggregateField::alias (`_sum_views`, `_count`, `_count_<col>`,
//! `_count_distinct_<col>`, …); bare-column orderings reference a
//! `by:` column.

use proc_macro2::{Span, TokenStream};
use quote::quote;

use super::LowerCtx;
use crate::macros::dsl::ast::{DslBlock, DslField, DslValue};
use crate::macros::lower::aggregate_select::AggKind;

/// An aggregate present in the group_by! call, for validating that an
/// order-by aggregate references something actually selected.
/// `column == None` means `_count`'s `_all`. `distinct` records a
/// `_count: { col: { distinct: true } }` selection, whose SELECT-list
/// alias is `_count_distinct_<col>` rather than `_count_<col>`.
pub struct AggPresence {
    pub kind: AggKind,
    pub column: Option<String>,
    pub distinct: bool,
}

pub fn lower_group_by_order_by(
    block: &DslBlock,
    _ctx: &LowerCtx<'_>,
    present_aggs: &[AggPresence],
    by_columns: &[String],
) -> syn::Result<TokenStream> {
    let mut items: Vec<TokenStream> = Vec::new();

    for entry in &block.fields {
        let DslField::Pair { key, value, .. } = entry else {
            return Err(syn::Error::new(
                Span::call_site(),
                "order_by block does not support spread or conditional fields",
            ));
        };
        let key_str = key.to_string();
        let agg_kind = match key_str.as_str() {
            "_count" => Some(AggKind::Count),
            "_sum" => Some(AggKind::Sum),
            "_avg" => Some(AggKind::Avg),
            "_min" => Some(AggKind::Min),
            "_max" => Some(AggKind::Max),
            _ => None,
        };

        if let Some(kind) = agg_kind {
            let DslValue::Block(inner) = value else {
                return Err(syn::Error::new(
                    key.span(),
                    format!(
                        "order_by `{}` must be a `{{ col: asc|desc }}` block",
                        key_str
                    ),
                ));
            };
            for ie in &inner.fields {
                let DslField::Pair {
                    key: ck, value: cv, ..
                } = ie
                else {
                    continue;
                };
                let col = ck.to_string();
                let dir_ts = parse_dir(cv, ck.span())?;
                let present = present_aggs.iter().find(|p| {
                    p.kind == kind
                        && match (&p.column, col.as_str()) {
                            (None, "_all") => true,
                            (Some(c), other) => c == other,
                            _ => false,
                        }
                });
                let Some(presence) = present else {
                    return Err(syn::Error::new(
                        ck.span(),
                        format!(
                            "order by `{}.{}` requires a matching `{}: {{ {} }}` block",
                            key_str, col, key_str, col
                        ),
                    ));
                };
                let alias = alias_for(kind, &col, presence.distinct);
                items.push(quote! {
                    ::prax_query::types::OrderByField::new(#alias, #dir_ts)
                });
            }
        } else {
            let col = key_str;
            if !by_columns.iter().any(|c| c == &col) {
                return Err(syn::Error::new(
                    key.span(),
                    format!("order by `{}` requires `{}` in `by:`", col, col),
                ));
            }
            let dir_ts = parse_dir(value, key.span())?;
            items.push(quote! {
                ::prax_query::types::OrderByField::new(#col, #dir_ts)
            });
        }
    }

    Ok(quote! {
        {
            let mut __ob: ::std::vec::Vec<::prax_query::types::OrderByField>
                = ::std::vec::Vec::new();
            #( __ob.push(#items); )*
            __ob
        }
    })
}

/// Compute the SELECT-list alias for an aggregate ordering, matching
/// AggregateField::alias. `distinct` marks a
/// `_count: { col: { distinct: true } }` selection, which the runtime
/// aliases as `_count_distinct_<col>` (AggregateField::CountDistinct).
fn alias_for(kind: AggKind, col: &str, distinct: bool) -> String {
    match kind {
        AggKind::Count if col == "_all" => "_count".to_string(),
        AggKind::Count if distinct => format!("_count_distinct_{}", col),
        AggKind::Count => format!("_count_{}", col),
        AggKind::Sum => format!("_sum_{}", col),
        AggKind::Avg => format!("_avg_{}", col),
        AggKind::Min => format!("_min_{}", col),
        AggKind::Max => format!("_max_{}", col),
    }
}

fn parse_dir(v: &DslValue, span: Span) -> syn::Result<TokenStream> {
    let name = match v {
        DslValue::BareIdent(i) => i.to_string(),
        _ => {
            return Err(syn::Error::new(
                span,
                "order direction must be `asc` or `desc`",
            ));
        }
    };
    match name.as_str() {
        "asc" => Ok(quote! { ::prax_query::types::SortOrder::Asc }),
        "desc" => Ok(quote! { ::prax_query::types::SortOrder::Desc }),
        other => Err(syn::Error::new(
            span,
            format!("unknown order direction `{}`; use asc or desc", other),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macros::lower::aggregate_select::AggKind;

    #[test]
    fn alias_for_matches_aggregate_field_alias() {
        assert_eq!(alias_for(AggKind::Count, "_all", false), "_count");
        assert_eq!(alias_for(AggKind::Count, "email", false), "_count_email");
        assert_eq!(alias_for(AggKind::Sum, "views", false), "_sum_views");
        assert_eq!(alias_for(AggKind::Avg, "score", false), "_avg_score");
        assert_eq!(
            alias_for(AggKind::Min, "created_at", false),
            "_min_created_at"
        );
        assert_eq!(
            alias_for(AggKind::Max, "created_at", false),
            "_max_created_at"
        );
    }

    #[test]
    fn alias_for_distinct_count() {
        // A `_count: { col: { distinct: true } }` selection aliases as
        // `_count_distinct_<col>` (AggregateField::CountDistinct::alias).
        assert_eq!(
            alias_for(AggKind::Count, "email", true),
            "_count_distinct_email"
        );
        // `_all` has no distinct form (rejected during select lowering),
        // so the flag is ignored there.
        assert_eq!(alias_for(AggKind::Count, "_all", true), "_count");
        // `distinct` is only valid inside `_count`; other kinds ignore it.
        assert_eq!(alias_for(AggKind::Sum, "views", true), "_sum_views");
    }
}
