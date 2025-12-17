//! GraphQL plugin - generates GraphQL type definitions.

use quote::quote;

use prax_schema::ast::{Enum, FieldType, Model, ScalarType, TypeModifier};

use crate::plugins::{Plugin, PluginContext, PluginOutput};

/// GraphQL plugin that generates GraphQL SDL type definitions.
///
/// When enabled, this plugin generates `graphql_sdl()` methods that return
/// GraphQL Schema Definition Language strings for each type.
///
/// Enable with: `PRAX_PLUGIN_GRAPHQL=1`
pub struct GraphQLPlugin;

impl Plugin for GraphQLPlugin {
    fn name(&self) -> &'static str {
        "graphql"
    }

    fn env_var(&self) -> &'static str {
        "PRAX_PLUGIN_GRAPHQL"
    }

    fn description(&self) -> &'static str {
        "Generates GraphQL SDL type definitions for models and enums"
    }

    fn on_model(&self, _ctx: &PluginContext, model: &Model) -> PluginOutput {
        let model_name = model.name();

        // Generate field definitions
        let fields: Vec<String> = model.fields.values().map(|field| {
            let field_name = field.name();
            let gql_type = field_type_to_graphql(&field.field_type, &field.modifier);
            format!("  {}: {}", field_name, gql_type)
        }).collect();

        let fields_str = fields.join("\n");
        let sdl = format!("type {} {{\n{}\n}}", model_name, fields_str);

        PluginOutput::with_tokens(quote! {
            /// GraphQL type definition for this model.
            pub mod _graphql {
                /// Get the GraphQL SDL for this type.
                pub const SDL: &str = #sdl;

                /// Get the GraphQL type name.
                pub const TYPE_NAME: &str = #model_name;

                /// Get field names for the GraphQL type.
                pub fn field_names() -> Vec<&'static str> {
                    vec![#(#fields),*]
                }
            }
        })
    }

    fn on_enum(&self, _ctx: &PluginContext, enum_def: &Enum) -> PluginOutput {
        let enum_name = enum_def.name();
        let variants: Vec<String> = enum_def.variants.iter()
            .map(|v| format!("  {}", v.name()))
            .collect();

        let variants_str = variants.join("\n");
        let sdl = format!("enum {} {{\n{}\n}}", enum_name, variants_str);

        PluginOutput::with_tokens(quote! {
            /// GraphQL enum definition.
            pub mod _graphql {
                /// Get the GraphQL SDL for this enum.
                pub const SDL: &str = #sdl;

                /// Get the GraphQL type name.
                pub const TYPE_NAME: &str = #enum_name;
            }
        })
    }

    fn on_finish(&self, ctx: &PluginContext) -> PluginOutput {
        let mut all_sdl_parts = Vec::new();

        // Collect all model SDLs
        for model in ctx.schema.models.values() {
            let model_name = model.name();
            let fields: Vec<String> = model.fields.values().map(|field| {
                let field_name = field.name();
                let gql_type = field_type_to_graphql(&field.field_type, &field.modifier);
                format!("  {}: {}", field_name, gql_type)
            }).collect();
            let fields_str = fields.join("\n");
            all_sdl_parts.push(format!("type {} {{\n{}\n}}", model_name, fields_str));
        }

        // Collect all enum SDLs
        for enum_def in ctx.schema.enums.values() {
            let enum_name = enum_def.name();
            let variants: Vec<String> = enum_def.variants.iter()
                .map(|v| format!("  {}", v.name()))
                .collect();
            let variants_str = variants.join("\n");
            all_sdl_parts.push(format!("enum {} {{\n{}\n}}", enum_name, variants_str));
        }

        let full_sdl = all_sdl_parts.join("\n\n");

        PluginOutput::with_tokens(quote! {
            /// Combined GraphQL schema for all types.
            pub mod _graphql_schema {
                /// The complete GraphQL SDL for the schema.
                pub const FULL_SDL: &str = #full_sdl;

                /// Print the full GraphQL schema.
                pub fn print_schema() {
                    println!("{}", FULL_SDL);
                }
            }
        })
    }
}

/// Convert a Prax field type to GraphQL type string.
fn field_type_to_graphql(field_type: &FieldType, modifier: &TypeModifier) -> String {
    let base_type = match field_type {
        FieldType::Scalar(scalar) => match scalar {
            ScalarType::Int => "Int",
            ScalarType::BigInt => "BigInt",
            ScalarType::Float | ScalarType::Decimal => "Float",
            ScalarType::Boolean => "Boolean",
            ScalarType::String => "String",
            ScalarType::DateTime | ScalarType::Date | ScalarType::Time => "DateTime",
            ScalarType::Json => "JSON",
            ScalarType::Bytes => "String",
            ScalarType::Uuid => "ID",
        },
        FieldType::Enum(name) | FieldType::Model(name) | FieldType::Composite(name) => {
            return format_graphql_type(name, modifier);
        }
        FieldType::Unsupported(_) => "String",
    };

    format_graphql_type(base_type, modifier)
}

/// Format a GraphQL type with modifiers.
fn format_graphql_type(base: &str, modifier: &TypeModifier) -> String {
    match modifier {
        TypeModifier::Required => format!("{}!", base),
        TypeModifier::Optional => base.to_string(),
        TypeModifier::List => format!("[{}!]!", base),
        TypeModifier::OptionalList => format!("[{}!]", base),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_schema::ast::{EnumVariant, Field, Ident, Span};
    use prax_schema::Schema;

    fn make_span() -> Span {
        Span::new(0, 0)
    }

    fn make_ident(name: &str) -> Ident {
        Ident::new(name, make_span())
    }

    #[test]
    fn test_graphql_plugin_model() {
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

        let plugin = GraphQLPlugin;
        let output = plugin.on_model(&ctx, &model);

        let code = output.tokens.to_string();
        assert!(code.contains("_graphql"));
        assert!(code.contains("SDL"));
        assert!(code.contains("User"));
    }

    #[test]
    fn test_graphql_plugin_enum() {
        let schema = Schema::new();
        let config = crate::plugins::PluginConfig::new();
        let ctx = PluginContext::new(&schema, &config);

        let mut enum_def = Enum::new(make_ident("Role"), make_span());
        enum_def.add_variant(EnumVariant::new(make_ident("USER"), make_span()));
        enum_def.add_variant(EnumVariant::new(make_ident("ADMIN"), make_span()));

        let plugin = GraphQLPlugin;
        let output = plugin.on_enum(&ctx, &enum_def);

        let code = output.tokens.to_string();
        assert!(code.contains("_graphql"));
        assert!(code.contains("enum Role"));
    }

    #[test]
    fn test_field_type_to_graphql() {
        assert_eq!(
            field_type_to_graphql(&FieldType::Scalar(ScalarType::Int), &TypeModifier::Required),
            "Int!"
        );
        assert_eq!(
            field_type_to_graphql(&FieldType::Scalar(ScalarType::String), &TypeModifier::Optional),
            "String"
        );
        assert_eq!(
            field_type_to_graphql(&FieldType::Scalar(ScalarType::Int), &TypeModifier::List),
            "[Int!]!"
        );
    }
}

