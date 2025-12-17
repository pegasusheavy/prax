//! Procedural macros for the Prax ORM.
//!
//! This crate provides compile-time code generation for Prax, transforming
//! schema definitions into type-safe Rust code.
//!
//! # Macros
//!
//! - [`prax_schema!`] - Generate models from a `.prax` schema file
//! - [`Model`] - Derive macro for manual model definition
//!
//! # Example
//!
//! ```rust,ignore
//! // Generate models from schema file
//! prax::prax_schema!("schema.prax");
//!
//! // Or manually define with derive macro
//! #[derive(prax::Model)]
//! #[prax(table = "users")]
//! struct User {
//!     #[prax(id, auto)]
//!     id: i32,
//!     #[prax(unique)]
//!     email: String,
//!     name: Option<String>,
//! }
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, LitStr};

mod generators;
mod schema_reader;
mod types;

use generators::{
    generate_enum_module, generate_model_module, generate_type_module, generate_view_module,
};
use schema_reader::read_and_parse_schema;

/// Generate models from a Prax schema file.
///
/// This macro reads a `.prax` schema file at compile time and generates
/// type-safe Rust code for all models, enums, and types defined in the schema.
///
/// # Example
///
/// ```rust,ignore
/// prax::prax_schema!("schema.prax");
///
/// // Now you can use the generated types:
/// let user = client.user().find_unique(user::id::equals(1)).exec().await?;
/// ```
///
/// # Generated Code
///
/// For each model in the schema, this macro generates:
/// - A module with the model name (snake_case)
/// - A `Data` struct representing a row from the database
/// - A `CreateInput` struct for creating new records
/// - A `UpdateInput` struct for updating records
/// - Field modules with filter operations (`equals`, `contains`, `in_`, etc.)
/// - A `WhereParam` enum for type-safe filtering
/// - An `OrderByParam` enum for sorting
/// - Select and Include builders for partial queries
#[proc_macro]
pub fn prax_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as LitStr);
    let schema_path = input.value();

    match generate_from_schema(&schema_path) {
        Ok(tokens) => tokens.into(),
        Err(err) => {
            let err_msg = err.to_string();
            quote! {
                compile_error!(#err_msg);
            }
            .into()
        }
    }
}

/// Derive macro for defining Prax models manually.
///
/// This derive macro allows you to define models in Rust code instead of
/// using a `.prax` schema file. It generates the same query builder methods
/// and type-safe operations.
///
/// # Attributes
///
/// ## Struct-level
/// - `#[prax(table = "table_name")]` - Map to a different table name
/// - `#[prax(schema = "schema_name")]` - Specify database schema
///
/// ## Field-level
/// - `#[prax(id)]` - Mark as primary key
/// - `#[prax(auto)]` - Auto-increment field
/// - `#[prax(unique)]` - Unique constraint
/// - `#[prax(default = value)]` - Default value
/// - `#[prax(column = "col_name")]` - Map to different column
/// - `#[prax(relation(...))]` - Define relation
///
/// # Example
///
/// ```rust,ignore
/// #[derive(prax::Model)]
/// #[prax(table = "users")]
/// struct User {
///     #[prax(id, auto)]
///     id: i32,
///     
///     #[prax(unique)]
///     email: String,
///     
///     #[prax(column = "display_name")]
///     name: Option<String>,
///     
///     #[prax(default = "now()")]
///     created_at: chrono::DateTime<chrono::Utc>,
/// }
/// ```
#[proc_macro_derive(Model, attributes(prax))]
pub fn derive_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match generators::derive_model_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Internal function to generate code from a schema file.
fn generate_from_schema(schema_path: &str) -> Result<proc_macro2::TokenStream, syn::Error> {
    // Read and parse the schema file
    let schema = read_and_parse_schema(schema_path).map_err(|e| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("Failed to parse schema: {}", e),
        )
    })?;

    let mut output = proc_macro2::TokenStream::new();

    // Generate prelude with common imports
    output.extend(generate_prelude());

    // Generate enums first (models may reference them)
    for (_, enum_def) in &schema.enums {
        output.extend(generate_enum_module(enum_def)?);
    }

    // Generate composite types
    for (_, type_def) in &schema.types {
        output.extend(generate_type_module(type_def)?);
    }

    // Generate views
    for (_, view_def) in &schema.views {
        output.extend(generate_view_module(view_def)?);
    }

    // Generate models
    for (_, model_def) in &schema.models {
        output.extend(generate_model_module(model_def, &schema)?);
    }

    Ok(output)
}

/// Generate the prelude module with common types and imports.
fn generate_prelude() -> proc_macro2::TokenStream {
    quote! {
        /// Common types used by generated Prax models.
        pub mod _prax_prelude {
            pub use std::future::Future;
            pub use std::pin::Pin;
            pub use std::sync::Arc;

            /// Marker trait for Prax models.
            pub trait PraxModel {
                /// The table name in the database.
                const TABLE_NAME: &'static str;

                /// The primary key column(s).
                const PRIMARY_KEY: &'static [&'static str];
            }

            /// Trait for types that can be converted to SQL parameters.
            pub trait ToSqlParam {
                /// Convert to a boxed SQL parameter.
                fn to_sql_param(&self) -> Box<dyn std::any::Any + Send + Sync>;
            }

            /// Marker for optional fields in queries.
            #[derive(Debug, Clone, Default)]
            pub struct Unset;

            /// Set or unset field wrapper for updates.
            #[derive(Debug, Clone)]
            pub enum SetParam<T> {
                /// Set the field to a value.
                Set(T),
                /// Leave the field unchanged.
                Unset,
            }

            impl<T> Default for SetParam<T> {
                fn default() -> Self {
                    Self::Unset
                }
            }

            impl<T> SetParam<T> {
                /// Check if the value is set.
                pub fn is_set(&self) -> bool {
                    matches!(self, Self::Set(_))
                }

                /// Get the inner value if set.
                pub fn get(&self) -> Option<&T> {
                    match self {
                        Self::Set(v) => Some(v),
                        Self::Unset => None,
                    }
                }

                /// Take the inner value if set.
                pub fn take(self) -> Option<T> {
                    match self {
                        Self::Set(v) => Some(v),
                        Self::Unset => None,
                    }
                }
            }

            /// Sort direction for order by clauses.
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum SortOrder {
                /// Ascending order (A-Z, 0-9).
                Asc,
                /// Descending order (Z-A, 9-0).
                Desc,
            }

            /// Null handling in sorting.
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum NullsOrder {
                /// Nulls first in the result.
                First,
                /// Nulls last in the result.
                Last,
            }

            /// Pagination cursor for cursor-based pagination.
            #[derive(Debug, Clone)]
            pub struct Cursor<T> {
                /// The field value to start from.
                pub value: T,
                /// The direction of pagination.
                pub direction: CursorDirection,
            }

            /// Direction for cursor-based pagination.
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum CursorDirection {
                /// Get records after the cursor.
                After,
                /// Get records before the cursor.
                Before,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prelude_generation() {
        let prelude = generate_prelude();
        let code = prelude.to_string();

        assert!(code.contains("pub mod _prax_prelude"));
        assert!(code.contains("pub trait PraxModel"));
        assert!(code.contains("pub enum SortOrder"));
        assert!(code.contains("pub enum SetParam"));
    }
}

