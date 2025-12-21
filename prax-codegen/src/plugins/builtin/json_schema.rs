//! JSON Schema plugin - generates JSON Schema definitions for models.

use quote::quote;

use prax_schema::ast::{Enum, FieldType, Model, ScalarType, TypeModifier};

use crate::plugins::{Plugin, PluginContext, PluginOutput};

/// JSON Schema plugin that generates JSON Schema definitions.
///
/// When enabled, this plugin generates `json_schema()` methods for each model
/// that return a JSON Schema representation of the type.
///
/// Enable with: `PRAX_PLUGIN_JSON_SCHEMA=1`
pub struct JsonSchemaPlugin;

impl Plugin for JsonSchemaPlugin {
    fn name(&self) -> &'static str {
        "json_schema"
    }

    fn env_var(&self) -> &'static str {
        "PRAX_PLUGIN_JSON_SCHEMA"
    }

    fn description(&self) -> &'static str {
        "Generates JSON Schema definitions for models and types"
    }

    fn on_model(&self, _ctx: &PluginContext, model: &Model) -> PluginOutput {
        let model_name = model.name();
        let table_name = model.table_name();

        // Generate properties schema
        let properties: Vec<_> = model
            .fields
            .values()
            .map(|field| {
                let field_name = field.name();
                let json_type = scalar_to_json_type(&field.field_type, &field.modifier);
                let required = !field.modifier.is_optional();

                quote! {
                    properties.insert(
                        #field_name.to_string(),
                        serde_json::json!({
                            "type": #json_type
                        })
                    );
                    if #required {
                        required_fields.push(#field_name.to_string());
                    }
                }
            })
            .collect();

        PluginOutput::with_tokens(quote! {
            /// JSON Schema generation for this model.
            pub mod _json_schema {
                use serde_json::Value;
                use std::collections::HashMap;

                /// Get the JSON Schema for this model.
                pub fn schema() -> Value {
                    let mut properties: HashMap<String, Value> = HashMap::new();
                    let mut required_fields: Vec<String> = Vec::new();

                    #(#properties)*

                    serde_json::json!({
                        "$schema": "http://json-schema.org/draft-07/schema#",
                        "title": #model_name,
                        "description": concat!("Schema for ", #table_name, " table"),
                        "type": "object",
                        "properties": properties,
                        "required": required_fields
                    })
                }

                /// Get the JSON Schema as a string.
                pub fn schema_string() -> String {
                    serde_json::to_string_pretty(&schema()).unwrap_or_default()
                }
            }
        })
    }

    fn on_enum(&self, _ctx: &PluginContext, enum_def: &Enum) -> PluginOutput {
        let enum_name = enum_def.name();
        let variants: Vec<_> = enum_def
            .variants
            .iter()
            .map(|v| v.db_value().to_string())
            .collect();

        PluginOutput::with_tokens(quote! {
            /// JSON Schema generation for this enum.
            pub mod _json_schema {
                use serde_json::Value;

                /// Get the JSON Schema for this enum.
                pub fn schema() -> Value {
                    serde_json::json!({
                        "$schema": "http://json-schema.org/draft-07/schema#",
                        "title": #enum_name,
                        "type": "string",
                        "enum": [#(#variants),*]
                    })
                }
            }
        })
    }
}

/// Convert a scalar type to JSON Schema type string.
fn scalar_to_json_type(field_type: &FieldType, modifier: &TypeModifier) -> &'static str {
    let base_type = match field_type {
        FieldType::Scalar(scalar) => match scalar {
            ScalarType::Int | ScalarType::BigInt => "integer",
            ScalarType::Float | ScalarType::Decimal => "number",
            ScalarType::Boolean => "boolean",
            ScalarType::String
            | ScalarType::DateTime
            | ScalarType::Date
            | ScalarType::Time
            | ScalarType::Uuid => "string",
            // String-based ID types
            ScalarType::Cuid | ScalarType::Cuid2 | ScalarType::NanoId | ScalarType::Ulid => {
                "string"
            }
            ScalarType::Json => "object",
            ScalarType::Bytes => "string", // base64 encoded
        },
        FieldType::Enum(_) => "string",
        FieldType::Model(_) | FieldType::Composite(_) => "object",
        FieldType::Unsupported(_) => "string",
    };

    match modifier {
        TypeModifier::List | TypeModifier::OptionalList => "array",
        _ => base_type,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_schema::Schema;
    use prax_schema::ast::{Field, Ident, Span};

    fn make_span() -> Span {
        Span::new(0, 0)
    }

    fn make_ident(name: &str) -> Ident {
        Ident::new(name, make_span())
    }

    #[test]
    fn test_json_schema_plugin_model() {
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

        let plugin = JsonSchemaPlugin;
        let output = plugin.on_model(&ctx, &model);

        let code = output.tokens.to_string();
        assert!(code.contains("_json_schema"));
        assert!(code.contains("schema"));
        assert!(code.contains("draft-07"));
    }

    #[test]
    fn test_scalar_to_json_type() {
        assert_eq!(
            scalar_to_json_type(&FieldType::Scalar(ScalarType::Int), &TypeModifier::Required),
            "integer"
        );
        assert_eq!(
            scalar_to_json_type(
                &FieldType::Scalar(ScalarType::String),
                &TypeModifier::Required
            ),
            "string"
        );
        assert_eq!(
            scalar_to_json_type(
                &FieldType::Scalar(ScalarType::Boolean),
                &TypeModifier::Required
            ),
            "boolean"
        );
        assert_eq!(
            scalar_to_json_type(&FieldType::Scalar(ScalarType::Int), &TypeModifier::List),
            "array"
        );
    }
}
