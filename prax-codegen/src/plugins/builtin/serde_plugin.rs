//! Serde plugin - adds custom serialization helpers.

use quote::quote;

use prax_schema::ast::{FieldType, Model, ScalarType};

use crate::plugins::{Plugin, PluginContext, PluginOutput};

/// Serde plugin that generates custom serialization helpers.
///
/// When enabled, this plugin adds:
/// - Custom date/time serialization formats
/// - Decimal serialization helpers
/// - UUID string serialization
/// - JSON field helpers
///
/// Enable with: `PRAX_PLUGIN_SERDE=1`
pub struct SerdePlugin;

impl Plugin for SerdePlugin {
    fn name(&self) -> &'static str {
        "serde"
    }

    fn env_var(&self) -> &'static str {
        "PRAX_PLUGIN_SERDE"
    }

    fn description(&self) -> &'static str {
        "Adds custom serialization helpers for dates, decimals, and UUIDs"
    }

    fn on_start(&self, _ctx: &PluginContext) -> PluginOutput {
        PluginOutput::with_tokens(quote! {
            /// Serde helper modules for custom serialization.
            pub mod _serde_helpers {
                /// ISO 8601 date format serialization.
                pub mod iso_date {
                    use serde::{self, Deserialize, Deserializer, Serializer};

                    const FORMAT: &str = "%Y-%m-%d";

                    pub fn serialize<S>(
                        date: &chrono::NaiveDate,
                        serializer: S,
                    ) -> Result<S::Ok, S::Error>
                    where
                        S: Serializer,
                    {
                        let s = date.format(FORMAT).to_string();
                        serializer.serialize_str(&s)
                    }

                    pub fn deserialize<'de, D>(deserializer: D) -> Result<chrono::NaiveDate, D::Error>
                    where
                        D: Deserializer<'de>,
                    {
                        let s = String::deserialize(deserializer)?;
                        chrono::NaiveDate::parse_from_str(&s, FORMAT)
                            .map_err(serde::de::Error::custom)
                    }
                }

                /// ISO 8601 datetime format serialization.
                pub mod iso_datetime {
                    use serde::{self, Deserialize, Deserializer, Serializer};

                    pub fn serialize<S>(
                        dt: &chrono::DateTime<chrono::Utc>,
                        serializer: S,
                    ) -> Result<S::Ok, S::Error>
                    where
                        S: Serializer,
                    {
                        serializer.serialize_str(&dt.to_rfc3339())
                    }

                    pub fn deserialize<'de, D>(
                        deserializer: D,
                    ) -> Result<chrono::DateTime<chrono::Utc>, D::Error>
                    where
                        D: Deserializer<'de>,
                    {
                        let s = String::deserialize(deserializer)?;
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .map_err(serde::de::Error::custom)
                    }
                }

                /// UUID as string serialization.
                pub mod uuid_string {
                    use serde::{self, Deserialize, Deserializer, Serializer};

                    pub fn serialize<S>(uuid: &uuid::Uuid, serializer: S) -> Result<S::Ok, S::Error>
                    where
                        S: Serializer,
                    {
                        serializer.serialize_str(&uuid.to_string())
                    }

                    pub fn deserialize<'de, D>(deserializer: D) -> Result<uuid::Uuid, D::Error>
                    where
                        D: Deserializer<'de>,
                    {
                        let s = String::deserialize(deserializer)?;
                        s.parse().map_err(serde::de::Error::custom)
                    }
                }

                /// Decimal as string serialization.
                pub mod decimal_string {
                    use serde::{self, Deserialize, Deserializer, Serializer};

                    pub fn serialize<S>(
                        decimal: &rust_decimal::Decimal,
                        serializer: S,
                    ) -> Result<S::Ok, S::Error>
                    where
                        S: Serializer,
                    {
                        serializer.serialize_str(&decimal.to_string())
                    }

                    pub fn deserialize<'de, D>(
                        deserializer: D,
                    ) -> Result<rust_decimal::Decimal, D::Error>
                    where
                        D: Deserializer<'de>,
                    {
                        let s = String::deserialize(deserializer)?;
                        s.parse().map_err(serde::de::Error::custom)
                    }
                }

                /// Bytes as base64 serialization.
                pub mod base64_bytes {
                    use serde::{self, Deserialize, Deserializer, Serializer};
                    use base64::{Engine as _, engine::general_purpose::STANDARD};

                    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
                    where
                        S: Serializer,
                    {
                        serializer.serialize_str(&STANDARD.encode(bytes))
                    }

                    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
                    where
                        D: Deserializer<'de>,
                    {
                        let s = String::deserialize(deserializer)?;
                        STANDARD.decode(&s).map_err(serde::de::Error::custom)
                    }
                }
            }
        })
    }

    fn on_model(&self, _ctx: &PluginContext, model: &Model) -> PluginOutput {
        // Check if model has any fields that need special serialization
        let has_datetime = model.fields.values().any(|f| {
            matches!(f.field_type, FieldType::Scalar(ScalarType::DateTime))
        });
        let has_date = model.fields.values().any(|f| {
            matches!(f.field_type, FieldType::Scalar(ScalarType::Date))
        });
        let has_uuid = model.fields.values().any(|f| {
            matches!(f.field_type, FieldType::Scalar(ScalarType::Uuid))
        });
        let has_decimal = model.fields.values().any(|f| {
            matches!(f.field_type, FieldType::Scalar(ScalarType::Decimal))
        });
        let has_bytes = model.fields.values().any(|f| {
            matches!(f.field_type, FieldType::Scalar(ScalarType::Bytes))
        });

        let mut hints = Vec::new();
        if has_datetime {
            hints.push("DateTime fields use RFC 3339 format");
        }
        if has_date {
            hints.push("Date fields use ISO 8601 format (YYYY-MM-DD)");
        }
        if has_uuid {
            hints.push("UUID fields are serialized as strings");
        }
        if has_decimal {
            hints.push("Decimal fields are serialized as strings");
        }
        if has_bytes {
            hints.push("Bytes fields are serialized as base64");
        }

        if hints.is_empty() {
            return PluginOutput::new();
        }

        let hints_str = hints.join(", ");

        PluginOutput::with_tokens(quote! {
            /// Serialization hints for this model.
            pub mod _serde_info {
                /// Serialization format hints.
                pub const HINTS: &str = #hints_str;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_schema::ast::{Field, Ident, Span, TypeModifier};
    use prax_schema::Schema;

    fn make_span() -> Span {
        Span::new(0, 0)
    }

    fn make_ident(name: &str) -> Ident {
        Ident::new(name, make_span())
    }

    #[test]
    fn test_serde_plugin_start() {
        let schema = Schema::new();
        let config = crate::plugins::PluginConfig::new();
        let ctx = PluginContext::new(&schema, &config);

        let plugin = SerdePlugin;
        let output = plugin.on_start(&ctx);

        let code = output.tokens.to_string();
        assert!(code.contains("_serde_helpers"));
        assert!(code.contains("iso_date"));
        assert!(code.contains("iso_datetime"));
        assert!(code.contains("uuid_string"));
    }

    #[test]
    fn test_serde_plugin_model_with_datetime() {
        let schema = Schema::new();
        let config = crate::plugins::PluginConfig::new();
        let ctx = PluginContext::new(&schema, &config);

        let mut model = Model::new(make_ident("Event"), make_span());
        model.add_field(Field::new(
            make_ident("created_at"),
            FieldType::Scalar(ScalarType::DateTime),
            TypeModifier::Required,
            vec![],
            make_span(),
        ));

        let plugin = SerdePlugin;
        let output = plugin.on_model(&ctx, &model);

        let code = output.tokens.to_string();
        assert!(code.contains("_serde_info"));
        assert!(code.contains("HINTS"));
    }
}

