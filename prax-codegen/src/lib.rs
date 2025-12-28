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
//! # Plugins
//!
//! Code generation can be extended with plugins enabled via environment variables:
//!
//! ```bash
//! # Enable debug information
//! PRAX_PLUGIN_DEBUG=1 cargo build
//!
//! # Enable JSON Schema generation
//! PRAX_PLUGIN_JSON_SCHEMA=1 cargo build
//!
//! # Enable GraphQL SDL generation
//! PRAX_PLUGIN_GRAPHQL=1 cargo build
//!
//! # Enable custom serialization helpers
//! PRAX_PLUGIN_SERDE=1 cargo build
//!
//! # Enable runtime validation
//! PRAX_PLUGIN_VALIDATOR=1 cargo build
//!
//! # Enable all plugins
//! PRAX_PLUGINS_ALL=1 cargo build
//! ```
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
use syn::{DeriveInput, LitStr, parse_macro_input};

mod generators;
mod plugins;
mod schema_reader;
mod types;

use generators::{
    generate_enum_module, generate_model_module_with_style, generate_type_module,
    generate_view_module,
};

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
    use plugins::{PluginConfig, PluginContext, PluginRegistry};
    use schema_reader::read_schema_with_config;

    // Read and parse the schema file along with prax.toml configuration
    let schema_with_config = read_schema_with_config(schema_path).map_err(|e| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("Failed to parse schema: {}", e),
        )
    })?;

    let schema = schema_with_config.schema;
    let model_style = schema_with_config.model_style;

    // Initialize plugin system with model_style from prax.toml
    // This auto-enables graphql plugins when model_style is GraphQL
    let plugin_config = PluginConfig::with_model_style(model_style);
    let plugin_registry = PluginRegistry::with_builtins();
    let plugin_ctx = PluginContext::new(&schema, &plugin_config);

    let mut output = proc_macro2::TokenStream::new();

    // Generate prelude with common imports
    output.extend(generate_prelude());

    // Run plugin start hooks
    let start_output = plugin_registry.run_start(&plugin_ctx);
    output.extend(start_output.tokens);
    output.extend(start_output.root_items);

    // Generate enums first (models may reference them)
    for (_, enum_def) in &schema.enums {
        output.extend(generate_enum_module(enum_def)?);

        // Run plugin enum hooks
        let plugin_output = plugin_registry.run_enum(&plugin_ctx, enum_def);
        if !plugin_output.is_empty() {
            // Add plugin output to the enum module
            output.extend(plugin_output.tokens);
        }
    }

    // Generate composite types
    for (_, type_def) in &schema.types {
        output.extend(generate_type_module(type_def)?);

        // Run plugin type hooks
        let plugin_output = plugin_registry.run_type(&plugin_ctx, type_def);
        if !plugin_output.is_empty() {
            output.extend(plugin_output.tokens);
        }
    }

    // Generate views
    for (_, view_def) in &schema.views {
        output.extend(generate_view_module(view_def)?);

        // Run plugin view hooks
        let plugin_output = plugin_registry.run_view(&plugin_ctx, view_def);
        if !plugin_output.is_empty() {
            output.extend(plugin_output.tokens);
        }
    }

    // Generate models with the configured model style
    for (_, model_def) in &schema.models {
        output.extend(generate_model_module_with_style(
            model_def,
            &schema,
            model_style,
        )?);

        // Run plugin model hooks
        let plugin_output = plugin_registry.run_model(&plugin_ctx, model_def);
        if !plugin_output.is_empty() {
            output.extend(plugin_output.tokens);
        }
    }

    // Run plugin finish hooks
    let finish_output = plugin_registry.run_finish(&plugin_ctx);
    output.extend(finish_output.tokens);
    output.extend(finish_output.root_items);

    // Generate plugin documentation
    output.extend(plugins::generate_plugin_docs(&plugin_registry));

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

    #[test]
    fn test_prelude_contains_table_name_const() {
        let prelude = generate_prelude();
        let code = prelude.to_string();

        assert!(code.contains("TABLE_NAME"));
        assert!(code.contains("PRIMARY_KEY"));
    }

    #[test]
    fn test_prelude_contains_to_sql_param_trait() {
        let prelude = generate_prelude();
        let code = prelude.to_string();

        assert!(code.contains("ToSqlParam"));
        assert!(code.contains("to_sql_param"));
    }

    #[test]
    fn test_prelude_contains_unset_type() {
        let prelude = generate_prelude();
        let code = prelude.to_string();

        assert!(code.contains("pub struct Unset"));
    }

    #[test]
    fn test_prelude_contains_set_param_methods() {
        let prelude = generate_prelude();
        let code = prelude.to_string();

        assert!(code.contains("fn is_set"));
        assert!(code.contains("fn get"));
        assert!(code.contains("fn take"));
    }

    #[test]
    fn test_prelude_contains_sort_order_variants() {
        let prelude = generate_prelude();
        let code = prelude.to_string();

        assert!(code.contains("Asc"));
        assert!(code.contains("Desc"));
    }

    #[test]
    fn test_prelude_contains_nulls_order() {
        let prelude = generate_prelude();
        let code = prelude.to_string();

        assert!(code.contains("pub enum NullsOrder"));
        assert!(code.contains("First"));
        assert!(code.contains("Last"));
    }

    #[test]
    fn test_prelude_contains_cursor_types() {
        let prelude = generate_prelude();
        let code = prelude.to_string();

        assert!(code.contains("pub struct Cursor"));
        assert!(code.contains("pub enum CursorDirection"));
        assert!(code.contains("After"));
        assert!(code.contains("Before"));
    }

    #[test]
    fn test_prelude_contains_std_imports() {
        let prelude = generate_prelude();
        let code = prelude.to_string();

        assert!(code.contains("std :: future :: Future"));
        assert!(code.contains("std :: pin :: Pin"));
        assert!(code.contains("std :: sync :: Arc"));
    }

    #[test]
    fn test_prelude_derive_macros() {
        let prelude = generate_prelude();
        let code = prelude.to_string();

        // SetParam should derive Clone and Debug
        assert!(code.contains("Clone"));
        assert!(code.contains("Debug"));
    }

    #[test]
    fn test_prelude_setparam_default_impl() {
        let prelude = generate_prelude();
        let code = prelude.to_string();

        // Should have Default implementation
        assert!(code.contains("impl < T > Default for SetParam"));
        assert!(code.contains("Self :: Unset"));
    }
}
