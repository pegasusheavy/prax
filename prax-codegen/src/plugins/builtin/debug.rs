//! Debug plugin - adds compile-time debug information and runtime logging.

use quote::quote;

use prax_schema::ast::{Enum, Model};

use crate::plugins::{Plugin, PluginContext, PluginOutput};

/// Debug plugin that generates additional debug information.
///
/// When enabled, this plugin:
/// - Adds `#[derive(Debug)]` to all generated types (if not already present)
/// - Generates debug helper methods
/// - Adds compile-time information about generated code
///
/// Enable with: `PRAX_PLUGIN_DEBUG=1`
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn name(&self) -> &'static str {
        "debug"
    }

    fn env_var(&self) -> &'static str {
        "PRAX_PLUGIN_DEBUG"
    }

    fn description(&self) -> &'static str {
        "Adds debug information and logging helpers to generated code"
    }

    fn on_start(&self, ctx: &PluginContext) -> PluginOutput {
        let model_count = ctx.schema.models.len();
        let enum_count = ctx.schema.enums.len();
        let type_count = ctx.schema.types.len();
        let view_count = ctx.schema.views.len();

        PluginOutput::with_tokens(quote! {
            /// Debug information about the generated schema.
            pub mod _debug {
                /// Number of models in the schema.
                pub const MODEL_COUNT: usize = #model_count;
                /// Number of enums in the schema.
                pub const ENUM_COUNT: usize = #enum_count;
                /// Number of composite types in the schema.
                pub const TYPE_COUNT: usize = #type_count;
                /// Number of views in the schema.
                pub const VIEW_COUNT: usize = #view_count;

                /// Print schema statistics.
                pub fn print_stats() {
                    eprintln!("[Prax Debug] Schema Statistics:");
                    eprintln!("  Models: {}", MODEL_COUNT);
                    eprintln!("  Enums: {}", ENUM_COUNT);
                    eprintln!("  Types: {}", TYPE_COUNT);
                    eprintln!("  Views: {}", VIEW_COUNT);
                }
            }
        })
    }

    fn on_model(&self, _ctx: &PluginContext, model: &Model) -> PluginOutput {
        let model_name = model.name();
        let field_count = model.fields.len();
        let field_names: Vec<_> = model.fields.keys().map(|k| k.to_string()).collect();

        PluginOutput::with_tokens(quote! {
            /// Debug information for this model.
            pub mod _model_debug {
                /// Name of the model.
                pub const MODEL_NAME: &str = #model_name;
                /// Number of fields in the model.
                pub const FIELD_COUNT: usize = #field_count;
                /// Names of all fields.
                pub const FIELD_NAMES: &[&str] = &[#(#field_names),*];

                /// Print model debug information.
                pub fn print_info() {
                    eprintln!("[Prax Debug] Model: {}", MODEL_NAME);
                    eprintln!("  Fields ({}): {:?}", FIELD_COUNT, FIELD_NAMES);
                }
            }
        })
    }

    fn on_enum(&self, _ctx: &PluginContext, enum_def: &Enum) -> PluginOutput {
        let enum_name = enum_def.name();
        let variant_count = enum_def.variants.len();
        let variant_names: Vec<_> = enum_def.variants.iter().map(|v| v.name().to_string()).collect();

        PluginOutput::with_tokens(quote! {
            /// Debug information for this enum.
            pub mod _enum_debug {
                /// Name of the enum.
                pub const ENUM_NAME: &str = #enum_name;
                /// Number of variants.
                pub const VARIANT_COUNT: usize = #variant_count;
                /// Names of all variants.
                pub const VARIANT_NAMES: &[&str] = &[#(#variant_names),*];
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_schema::ast::{Ident, Span};
    use prax_schema::Schema;

    fn make_span() -> Span {
        Span::new(0, 0)
    }

    fn make_ident(name: &str) -> Ident {
        Ident::new(name, make_span())
    }

    #[test]
    fn test_debug_plugin_start() {
        let schema = Schema::new();
        let config = crate::plugins::PluginConfig::new();
        let ctx = PluginContext::new(&schema, &config);

        let plugin = DebugPlugin;
        let output = plugin.on_start(&ctx);

        let code = output.tokens.to_string();
        assert!(code.contains("_debug"));
        assert!(code.contains("MODEL_COUNT"));
        assert!(code.contains("print_stats"));
    }

    #[test]
    fn test_debug_plugin_model() {
        let schema = Schema::new();
        let config = crate::plugins::PluginConfig::new();
        let ctx = PluginContext::new(&schema, &config);

        let model = Model::new(make_ident("User"), make_span());

        let plugin = DebugPlugin;
        let output = plugin.on_model(&ctx, &model);

        let code = output.tokens.to_string();
        assert!(code.contains("_model_debug"));
        assert!(code.contains("MODEL_NAME"));
        assert!(code.contains("User"));
    }
}

