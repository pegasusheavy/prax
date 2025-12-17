//! Code generation for Prax models.

use proc_macro2::TokenStream;
use quote::quote;

use prax_schema::ast::{FieldType, Model, Schema, TypeModifier};

use super::fields::{generate_field_module, generate_order_by_param, generate_select_param, generate_set_param};
use super::{generate_doc_comment, pascal_ident, snake_ident};
use crate::types::field_type_to_rust;

/// Generate the complete module for a model.
pub fn generate_model_module(model: &Model, schema: &Schema) -> Result<TokenStream, syn::Error> {
    let model_name = pascal_ident(model.name());
    let module_name = snake_ident(model.name());

    let doc = generate_doc_comment(model.documentation.as_ref().map(|d| d.text.as_str()));

    // Get database table name
    let table_name = model.table_name().to_string();
    let table_name_str = table_name.as_str();

    // Get primary key field(s)
    let pk_fields = get_primary_key_fields(model);
    let pk_field_names: Vec<_> = pk_fields.iter().map(|f| f.as_str()).collect();

    // Generate Data struct fields
    let data_fields: Vec<_> = model
        .fields
        .values()
        .map(|field| {
            let field_name = snake_ident(field.name());
            let field_type = field_type_to_rust(&field.field_type, &field.modifier);
            let field_doc =
                generate_doc_comment(field.documentation.as_ref().map(|d| d.text.as_str()));

            let serde_rename = field
                .attributes
                .iter()
                .find(|a| a.name() == "map")
                .and_then(|a| a.first_arg())
                .and_then(|v| v.as_string())
                .map(|name| quote! { #[serde(rename = #name)] })
                .unwrap_or_default();

            quote! {
                #field_doc
                #serde_rename
                pub #field_name: #field_type
            }
        })
        .collect();

    // Generate CreateInput fields (excluding auto-generated fields)
    let create_fields: Vec<_> = model
        .fields
        .values()
        .filter(|f| {
            let attrs = f.extract_attributes();
            !attrs.is_auto && !attrs.is_updated_at && !matches!(f.field_type, FieldType::Model(_))
        })
        .map(|field| {
            let field_name = snake_ident(field.name());
            let is_optional = field.modifier.is_optional() || field.extract_attributes().default.is_some();
            let base_type = field_type_to_rust(&field.field_type, &TypeModifier::Required);
            let field_type = if is_optional {
                quote! { Option<#base_type> }
            } else {
                base_type
            };

            quote! {
                pub #field_name: #field_type
            }
        })
        .collect();

    // Generate UpdateInput fields (all optional)
    let update_fields: Vec<_> = model
        .fields
        .values()
        .filter(|f| {
            let attrs = f.extract_attributes();
            !attrs.is_auto && !attrs.is_updated_at && !matches!(f.field_type, FieldType::Model(_))
        })
        .map(|field| {
            let field_name = snake_ident(field.name());
            let base_type = field_type_to_rust(&field.field_type, &TypeModifier::Required);

            quote! {
                pub #field_name: Option<#base_type>
            }
        })
        .collect();

    // Generate field modules
    let field_modules: Vec<_> = model
        .fields
        .values()
        .map(|field| generate_field_module(field, model))
        .collect();

    // Generate where param enum
    let where_param = generate_where_param(model);

    // Generate select, order by, and set params
    let select_param = generate_select_param(model);
    let order_by_param = generate_order_by_param(model);
    let set_param = generate_set_param(model);

    // Generate query builder
    let query_builder = generate_query_builder(model, &table_name);

    // Generate relation helpers
    let relation_helpers = generate_relation_helpers(model, schema);

    Ok(quote! {
        #doc
        pub mod #module_name {
            use serde::{Deserialize, Serialize};

            /// Database table name.
            pub const TABLE_NAME: &str = #table_name_str;

            /// Primary key column(s).
            pub const PRIMARY_KEY: &[&str] = &[#(#pk_field_names),*];

            #doc
            /// Represents a row from the `#table_name_str` table.
            #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
            pub struct #model_name {
                #(#data_fields,)*
            }

            impl super::_prax_prelude::PraxModel for #model_name {
                const TABLE_NAME: &'static str = TABLE_NAME;
                const PRIMARY_KEY: &'static [&'static str] = PRIMARY_KEY;
            }

            /// Input type for creating a new record.
            #[derive(Debug, Clone, Default, Serialize, Deserialize)]
            pub struct CreateInput {
                #(#create_fields,)*
            }

            /// Input type for updating a record.
            #[derive(Debug, Clone, Default, Serialize, Deserialize)]
            pub struct UpdateInput {
                #(#update_fields,)*
            }

            // Field modules
            #(#field_modules)*

            // Where param enum
            #where_param

            // Select, OrderBy, and Set params
            #select_param
            #order_by_param
            #set_param

            // Query builder
            #query_builder

            // Relation helpers
            #relation_helpers
        }

        // Re-export the model type at the parent level
        pub use #module_name::#model_name;
    })
}

/// Get the primary key field names for a model.
fn get_primary_key_fields(model: &Model) -> Vec<String> {
    // Check for composite @@id
    if let Some(attr) = model.attributes.iter().find(|a| a.name() == "id") {
        if let Some(prax_schema::ast::AttributeValue::FieldRefList(fields)) = attr.first_arg() {
            return fields.iter().map(|s| s.to_string()).collect();
        }
    }

    // Otherwise, find @id field
    model
        .fields
        .values()
        .filter(|f| f.is_id())
        .map(|f| f.name().to_string())
        .collect()
}

/// Generate the WhereParam enum for a model.
fn generate_where_param(model: &Model) -> TokenStream {
    let variants: Vec<_> = model
        .fields
        .values()
        .map(|field| {
            let name = pascal_ident(field.name());
            let field_mod = snake_ident(field.name());
            quote! { #name(#field_mod::WhereOp) }
        })
        .collect();

    let to_sql_matches: Vec<_> = model
        .fields
        .values()
        .map(|field| {
            let name = pascal_ident(field.name());
            let field_mod = snake_ident(field.name());
            quote! { Self::#name(op) => #field_mod::COLUMN }
        })
        .collect();

    quote! {
        /// Where clause parameters for filtering queries.
        #[derive(Debug, Clone)]
        pub enum WhereParam {
            #(#variants,)*
            /// Combine with AND.
            And(Vec<WhereParam>),
            /// Combine with OR.
            Or(Vec<WhereParam>),
            /// Negate the condition.
            Not(Box<WhereParam>),
        }

        impl WhereParam {
            /// Get the column name for simple conditions.
            pub fn column(&self) -> Option<&'static str> {
                match self {
                    #(#to_sql_matches,)*
                    Self::And(_) | Self::Or(_) | Self::Not(_) => None,
                }
            }

            /// Combine multiple conditions with AND.
            pub fn and(conditions: Vec<WhereParam>) -> Self {
                Self::And(conditions)
            }

            /// Combine multiple conditions with OR.
            pub fn or(conditions: Vec<WhereParam>) -> Self {
                Self::Or(conditions)
            }

            /// Negate a condition.
            pub fn not(condition: WhereParam) -> Self {
                Self::Not(Box::new(condition))
            }
        }
    }
}

/// Generate the query builder for a model.
fn generate_query_builder(_model: &Model, _table_name: &str) -> TokenStream {

    quote! {
        /// Query builder for the model.
        #[derive(Debug, Default)]
        pub struct Query {
            /// Select specific fields.
            pub select: Vec<SelectParam>,
            /// Where conditions.
            pub where_: Vec<WhereParam>,
            /// Order by clauses.
            pub order_by: Vec<OrderByParam>,
            /// Skip N records.
            pub skip: Option<usize>,
            /// Take N records.
            pub take: Option<usize>,
            /// Distinct fields.
            pub distinct: Vec<SelectParam>,
        }

        impl Query {
            /// Create a new query builder.
            pub fn new() -> Self {
                Self::default()
            }

            /// Add a where condition.
            pub fn where_(mut self, param: WhereParam) -> Self {
                self.where_.push(param);
                self
            }

            /// Add multiple where conditions with AND.
            pub fn where_and(mut self, params: Vec<WhereParam>) -> Self {
                self.where_.push(WhereParam::And(params));
                self
            }

            /// Add multiple where conditions with OR.
            pub fn where_or(mut self, params: Vec<WhereParam>) -> Self {
                self.where_.push(WhereParam::Or(params));
                self
            }

            /// Order by a field.
            pub fn order_by(mut self, param: OrderByParam) -> Self {
                self.order_by.push(param);
                self
            }

            /// Skip N records.
            pub fn skip(mut self, n: usize) -> Self {
                self.skip = Some(n);
                self
            }

            /// Take N records.
            pub fn take(mut self, n: usize) -> Self {
                self.take = Some(n);
                self
            }

            /// Select specific fields.
            pub fn select(mut self, fields: Vec<SelectParam>) -> Self {
                self.select = fields;
                self
            }

            /// Get distinct values.
            pub fn distinct(mut self, fields: Vec<SelectParam>) -> Self {
                self.distinct = fields;
                self
            }

            /// Generate the SELECT SQL query.
            pub fn to_select_sql(&self) -> String {
                let columns = if self.select.is_empty() {
                    "*".to_string()
                } else {
                    self.select.iter().map(|s| s.column()).collect::<Vec<_>>().join(", ")
                };

                let distinct = if self.distinct.is_empty() {
                    String::new()
                } else {
                    format!(
                        "DISTINCT ON ({}) ",
                        self.distinct.iter().map(|d| d.column()).collect::<Vec<_>>().join(", ")
                    )
                };

                let mut sql = format!("SELECT {}{} FROM {}", distinct, columns, TABLE_NAME);

                // WHERE clause would be added here with parameter binding

                if !self.order_by.is_empty() {
                    sql.push_str(" ORDER BY ");
                    sql.push_str(
                        &self.order_by.iter().map(|o| o.to_sql()).collect::<Vec<_>>().join(", ")
                    );
                }

                if let Some(take) = self.take {
                    sql.push_str(&format!(" LIMIT {}", take));
                }

                if let Some(skip) = self.skip {
                    sql.push_str(&format!(" OFFSET {}", skip));
                }

                sql
            }
        }

        /// Actions available on the model.
        pub struct Actions;

        impl Actions {
            /// Find multiple records.
            pub fn find_many() -> Query {
                Query::new()
            }

            /// Find a unique record (by primary key or unique constraint).
            pub fn find_unique() -> Query {
                Query::new().take(1)
            }

            /// Find the first matching record.
            pub fn find_first() -> Query {
                Query::new().take(1)
            }

            /// Create input for a new record.
            pub fn create() -> CreateInput {
                CreateInput::default()
            }

            /// Update input for a record.
            pub fn update() -> UpdateInput {
                UpdateInput::default()
            }
        }
    }
}

/// Generate relation helper types.
fn generate_relation_helpers(model: &Model, _schema: &Schema) -> TokenStream {
    let relation_fields: Vec<_> = model
        .fields
        .values()
        .filter(|f| matches!(f.field_type, FieldType::Model(_)))
        .collect();

    if relation_fields.is_empty() {
        return TokenStream::new();
    }

    let include_variants: Vec<_> = relation_fields
        .iter()
        .map(|f| {
            let name = pascal_ident(f.name());
            let is_list = f.modifier.is_list();
            if is_list {
                quote! { #name(Option<Box<super::super::#name::Query>>) }
            } else {
                quote! { #name }
            }
        })
        .collect();

    quote! {
        /// Include related records in the query.
        #[derive(Debug, Clone, Default)]
        pub enum IncludeParam {
            #[default]
            None,
            #(#include_variants,)*
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_schema::ast::{Attribute, Field, Ident, ScalarType, Span};

    fn make_span() -> Span {
        Span::new(0, 0)
    }

    fn make_ident(name: &str) -> Ident {
        Ident::new(name, make_span())
    }

    fn make_simple_schema() -> Schema {
        let mut schema = Schema::new();
        let mut user = Model::new(make_ident("User"), make_span());
        user.add_field(
            Field::new(
                make_ident("id"),
                FieldType::Scalar(ScalarType::Int),
                TypeModifier::Required,
                vec![
                    Attribute::simple(make_ident("id"), make_span()),
                    Attribute::simple(make_ident("auto"), make_span()),
                ],
                make_span(),
            ),
        );
        user.add_field(
            Field::new(
                make_ident("email"),
                FieldType::Scalar(ScalarType::String),
                TypeModifier::Required,
                vec![Attribute::simple(make_ident("unique"), make_span())],
                make_span(),
            ),
        );
        user.add_field(
            Field::new(
                make_ident("name"),
                FieldType::Scalar(ScalarType::String),
                TypeModifier::Optional,
                vec![],
                make_span(),
            ),
        );
        schema.add_model(user);
        schema
    }

    #[test]
    fn test_generate_model_module() {
        let schema = make_simple_schema();
        let model = schema.get_model("User").unwrap();

        let result = generate_model_module(model, &schema);
        assert!(result.is_ok());

        let code = result.unwrap().to_string();
        assert!(code.contains("pub mod user"));
        assert!(code.contains("pub struct User"));
        assert!(code.contains("pub struct CreateInput"));
        assert!(code.contains("pub struct UpdateInput"));
        assert!(code.contains("pub enum WhereParam"));
        assert!(code.contains("pub struct Query"));
    }

    #[test]
    fn test_get_primary_key_fields() {
        let schema = make_simple_schema();
        let model = schema.get_model("User").unwrap();

        let pk = get_primary_key_fields(model);
        assert_eq!(pk, vec!["id"]);
    }
}

