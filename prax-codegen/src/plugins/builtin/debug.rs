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
        let variant_names: Vec<_> = enum_def
            .variants
            .iter()
            .map(|v| v.name().to_string())
            .collect();

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
    use prax_schema::Schema;
    use prax_schema::ast::{
        CompositeType, EnumVariant, Field, FieldType, Ident, ScalarType, Span, TypeModifier, View,
    };

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

    #[test]
    fn test_debug_plugin_name() {
        let plugin = DebugPlugin;
        assert_eq!(plugin.name(), "debug");
    }

    #[test]
    fn test_debug_plugin_env_var() {
        let plugin = DebugPlugin;
        assert_eq!(plugin.env_var(), "PRAX_PLUGIN_DEBUG");
    }

    #[test]
    fn test_debug_plugin_description() {
        let plugin = DebugPlugin;
        assert!(plugin.description().contains("debug"));
    }

    #[test]
    fn test_debug_plugin_start_with_populated_schema() {
        let mut schema = Schema::new();
        schema.add_model(Model::new(make_ident("User"), make_span()));
        schema.add_model(Model::new(make_ident("Post"), make_span()));
        schema.add_enum(Enum::new(make_ident("Role"), make_span()));
        schema.add_type(CompositeType::new(make_ident("Address"), make_span()));
        schema.add_view(View::new(make_ident("UserStats"), make_span()));

        let config = crate::plugins::PluginConfig::new();
        let ctx = PluginContext::new(&schema, &config);

        let plugin = DebugPlugin;
        let output = plugin.on_start(&ctx);

        let code = output.tokens.to_string();
        assert!(code.contains("MODEL_COUNT"));
        assert!(code.contains("ENUM_COUNT"));
        assert!(code.contains("TYPE_COUNT"));
        assert!(code.contains("VIEW_COUNT"));
    }

    #[test]
    fn test_debug_plugin_model_with_fields() {
        let schema = Schema::new();
        let config = crate::plugins::PluginConfig::new();
        let ctx = PluginContext::new(&schema, &config);

        let mut model = Model::new(make_ident("User"), make_span());
        model.add_field(Field::new(
            make_ident("id"),
            FieldType::Scalar(ScalarType::Int),
            TypeModifier::Required,
            vec![],
            make_span(),
        ));
        model.add_field(Field::new(
            make_ident("email"),
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
            vec![],
            make_span(),
        ));

        let plugin = DebugPlugin;
        let output = plugin.on_model(&ctx, &model);

        let code = output.tokens.to_string();
        assert!(code.contains("FIELD_COUNT"));
        assert!(code.contains("FIELD_NAMES"));
        assert!(code.contains("print_info"));
    }

    #[test]
    fn test_debug_plugin_enum() {
        let schema = Schema::new();
        let config = crate::plugins::PluginConfig::new();
        let ctx = PluginContext::new(&schema, &config);

        let mut enum_def = Enum::new(make_ident("Role"), make_span());
        enum_def.add_variant(EnumVariant::new(make_ident("Admin"), make_span()));
        enum_def.add_variant(EnumVariant::new(make_ident("User"), make_span()));
        enum_def.add_variant(EnumVariant::new(make_ident("Moderator"), make_span()));

        let plugin = DebugPlugin;
        let output = plugin.on_enum(&ctx, &enum_def);

        let code = output.tokens.to_string();
        assert!(code.contains("_enum_debug"));
        assert!(code.contains("ENUM_NAME"));
        assert!(code.contains("Role"));
        assert!(code.contains("VARIANT_COUNT"));
        assert!(code.contains("VARIANT_NAMES"));
    }

    #[test]
    fn test_debug_plugin_enum_with_no_variants() {
        let schema = Schema::new();
        let config = crate::plugins::PluginConfig::new();
        let ctx = PluginContext::new(&schema, &config);

        let enum_def = Enum::new(make_ident("EmptyEnum"), make_span());

        let plugin = DebugPlugin;
        let output = plugin.on_enum(&ctx, &enum_def);

        let code = output.tokens.to_string();
        assert!(code.contains("EmptyEnum"));
        assert!(code.contains("VARIANT_COUNT"));
    }

    #[test]
    fn test_debug_plugin_start_generates_print_stats() {
        let schema = Schema::new();
        let config = crate::plugins::PluginConfig::new();
        let ctx = PluginContext::new(&schema, &config);

        let plugin = DebugPlugin;
        let output = plugin.on_start(&ctx);

        let code = output.tokens.to_string();
        assert!(code.contains("print_stats"));
        assert!(code.contains("eprintln"));
        assert!(code.contains("Prax Debug"));
    }

    #[test]
    fn test_debug_plugin_model_generates_print_info() {
        let schema = Schema::new();
        let config = crate::plugins::PluginConfig::new();
        let ctx = PluginContext::new(&schema, &config);

        let model = Model::new(make_ident("TestModel"), make_span());

        let plugin = DebugPlugin;
        let output = plugin.on_model(&ctx, &model);

        let code = output.tokens.to_string();
        assert!(code.contains("print_info"));
        assert!(code.contains("eprintln"));
    }

    #[test]
    fn test_debug_plugin_model_field_names_are_correct() {
        let schema = Schema::new();
        let config = crate::plugins::PluginConfig::new();
        let ctx = PluginContext::new(&schema, &config);

        let mut model = Model::new(make_ident("User"), make_span());
        model.add_field(Field::new(
            make_ident("firstName"),
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
            vec![],
            make_span(),
        ));
        model.add_field(Field::new(
            make_ident("lastName"),
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
            vec![],
            make_span(),
        ));

        let plugin = DebugPlugin;
        let output = plugin.on_model(&ctx, &model);

        let code = output.tokens.to_string();
        // Field names should be included in the generated code
        assert!(code.contains("firstName") || code.contains("lastName"));
    }
}
