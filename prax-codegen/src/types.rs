//! Type mapping from Prax schema types to Rust types.

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use prax_schema::ast::{FieldType, ScalarType, TypeModifier};

/// Convert a schema scalar type to its Rust type token stream.
pub fn scalar_to_rust_type(scalar: &ScalarType) -> TokenStream {
    match scalar {
        ScalarType::Int => quote! { i32 },
        ScalarType::BigInt => quote! { i64 },
        ScalarType::Float => quote! { f64 },
        ScalarType::Decimal => quote! { rust_decimal::Decimal },
        ScalarType::Boolean => quote! { bool },
        ScalarType::String => quote! { String },
        ScalarType::DateTime => quote! { chrono::DateTime<chrono::Utc> },
        ScalarType::Date => quote! { chrono::NaiveDate },
        ScalarType::Time => quote! { chrono::NaiveTime },
        ScalarType::Json => quote! { serde_json::Value },
        ScalarType::Bytes => quote! { Vec<u8> },
        ScalarType::Uuid => quote! { uuid::Uuid },
    }
}

/// Convert a field type to its Rust type token stream.
pub fn field_type_to_rust(field_type: &FieldType, modifier: &TypeModifier) -> TokenStream {
    let base_type = match field_type {
        FieldType::Scalar(scalar) => scalar_to_rust_type(scalar),
        FieldType::Enum(name) | FieldType::Model(name) | FieldType::Composite(name) => {
            let ident = format_ident!("{}", name.to_string());
            quote! { #ident }
        }
        FieldType::Unsupported(raw) => {
            // For unsupported types, we use a raw SQL type wrapper
            let raw_str = raw.as_str();
            quote! { prax::types::UnsupportedType<#raw_str> }
        }
    };

    apply_modifier(base_type, modifier)
}

/// Apply a type modifier to a base type.
pub fn apply_modifier(base_type: TokenStream, modifier: &TypeModifier) -> TokenStream {
    match modifier {
        TypeModifier::Required => base_type,
        TypeModifier::Optional => quote! { Option<#base_type> },
        TypeModifier::List => quote! { Vec<#base_type> },
        TypeModifier::OptionalList => quote! { Option<Vec<#base_type>> },
    }
}

/// Convert a field type to the SQL type string for the query builder.
#[allow(dead_code)]
pub fn field_type_to_sql_type(field_type: &FieldType) -> &'static str {
    match field_type {
        FieldType::Scalar(scalar) => match scalar {
            ScalarType::Int => "INT",
            ScalarType::BigInt => "BIGINT",
            ScalarType::Float => "DOUBLE PRECISION",
            ScalarType::Decimal => "DECIMAL",
            ScalarType::Boolean => "BOOLEAN",
            ScalarType::String => "TEXT",
            ScalarType::DateTime => "TIMESTAMPTZ",
            ScalarType::Date => "DATE",
            ScalarType::Time => "TIME",
            ScalarType::Json => "JSONB",
            ScalarType::Bytes => "BYTEA",
            ScalarType::Uuid => "UUID",
        },
        FieldType::Enum(_) => "TEXT", // Enums are stored as text
        FieldType::Model(_) => "INT",  // Foreign key reference
        FieldType::Composite(_) => "JSONB", // Composite types as JSON
        FieldType::Unsupported(_raw) => {
            // Return the raw type as-is (static lifetime requires leaking)
            // In practice, we should handle this differently
            "UNKNOWN"
        }
    }
}

/// Convert a name to snake_case for module/function names.
pub fn to_snake_case(name: &str) -> String {
    name.to_case(Case::Snake)
}

/// Convert a name to PascalCase for type names.
pub fn to_pascal_case(name: &str) -> String {
    name.to_case(Case::Pascal)
}

/// Convert a name to SCREAMING_SNAKE_CASE for constants.
#[allow(dead_code)]
pub fn to_screaming_snake(name: &str) -> String {
    name.to_case(Case::ScreamingSnake)
}

/// Get the default value expression for a scalar type.
#[allow(dead_code)]
pub fn default_value_for_type(scalar: &ScalarType) -> TokenStream {
    match scalar {
        ScalarType::Int | ScalarType::BigInt => quote! { 0 },
        ScalarType::Float | ScalarType::Decimal => quote! { 0.0 },
        ScalarType::Boolean => quote! { false },
        ScalarType::String => quote! { String::new() },
        ScalarType::DateTime => quote! { chrono::Utc::now() },
        ScalarType::Date => quote! { chrono::Utc::now().date_naive() },
        ScalarType::Time => quote! { chrono::Utc::now().time() },
        ScalarType::Json => quote! { serde_json::Value::Null },
        ScalarType::Bytes => quote! { Vec::new() },
        ScalarType::Uuid => quote! { uuid::Uuid::nil() },
    }
}

/// Check if a scalar type supports comparison operations (>, <, >=, <=).
pub fn supports_comparison(scalar: &ScalarType) -> bool {
    matches!(
        scalar,
        ScalarType::Int
            | ScalarType::BigInt
            | ScalarType::Float
            | ScalarType::Decimal
            | ScalarType::DateTime
            | ScalarType::Date
            | ScalarType::Time
    )
}

/// Check if a scalar type supports string operations (contains, startsWith, etc.).
pub fn supports_string_ops(scalar: &ScalarType) -> bool {
    matches!(scalar, ScalarType::String)
}

/// Check if a type supports the `in` operation.
pub fn supports_in_op(field_type: &FieldType) -> bool {
    match field_type {
        FieldType::Scalar(scalar) => !matches!(scalar, ScalarType::Json | ScalarType::Bytes),
        FieldType::Enum(_) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_to_rust_type() {
        assert_eq!(scalar_to_rust_type(&ScalarType::Int).to_string(), "i32");
        assert_eq!(scalar_to_rust_type(&ScalarType::BigInt).to_string(), "i64");
        assert_eq!(scalar_to_rust_type(&ScalarType::Float).to_string(), "f64");
        assert_eq!(scalar_to_rust_type(&ScalarType::Boolean).to_string(), "bool");
        assert_eq!(scalar_to_rust_type(&ScalarType::String).to_string(), "String");
    }

    #[test]
    fn test_apply_modifier() {
        let base = quote! { i32 };

        let required = apply_modifier(base.clone(), &TypeModifier::Required);
        assert_eq!(required.to_string(), "i32");

        let optional = apply_modifier(base.clone(), &TypeModifier::Optional);
        assert_eq!(optional.to_string(), "Option < i32 >");

        let list = apply_modifier(base.clone(), &TypeModifier::List);
        assert_eq!(list.to_string(), "Vec < i32 >");

        let opt_list = apply_modifier(base, &TypeModifier::OptionalList);
        assert_eq!(opt_list.to_string(), "Option < Vec < i32 >>");
    }

    #[test]
    fn test_case_conversion() {
        assert_eq!(to_snake_case("UserProfile"), "user_profile");
        assert_eq!(to_pascal_case("user_profile"), "UserProfile");
        assert_eq!(to_screaming_snake("UserProfile"), "USER_PROFILE");
    }

    #[test]
    fn test_supports_comparison() {
        assert!(supports_comparison(&ScalarType::Int));
        assert!(supports_comparison(&ScalarType::DateTime));
        assert!(!supports_comparison(&ScalarType::String));
        assert!(!supports_comparison(&ScalarType::Boolean));
    }

    #[test]
    fn test_supports_string_ops() {
        assert!(supports_string_ops(&ScalarType::String));
        assert!(!supports_string_ops(&ScalarType::Int));
    }
}

