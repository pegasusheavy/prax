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
        // String-based ID types (stored as String in Rust)
        ScalarType::Cuid | ScalarType::Cuid2 | ScalarType::NanoId | ScalarType::Ulid => {
            quote! { String }
        }
        // PostgreSQL vector types (require pgvector crate)
        ScalarType::Vector(_) | ScalarType::HalfVector(_) => quote! { Vec<f32> },
        ScalarType::SparseVector(_) => quote! { Vec<(u32, f32)> },
        ScalarType::Bit(_) => quote! { Vec<u8> },
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
            // String-based ID types (stored as TEXT/VARCHAR in database)
            ScalarType::Cuid | ScalarType::Cuid2 | ScalarType::NanoId | ScalarType::Ulid => "TEXT",
            // PostgreSQL vector extension types (dimension is handled separately)
            ScalarType::Vector(_) => "vector",
            ScalarType::HalfVector(_) => "halfvec",
            ScalarType::SparseVector(_) => "sparsevec",
            ScalarType::Bit(_) => "bit",
        },
        FieldType::Enum(_) => "TEXT",       // Enums are stored as text
        FieldType::Model(_) => "INT",       // Foreign key reference
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
        // String-based ID types default to empty string
        ScalarType::Cuid | ScalarType::Cuid2 | ScalarType::NanoId | ScalarType::Ulid => {
            quote! { String::new() }
        }
        // Vector types default to empty vector
        ScalarType::Vector(_) | ScalarType::HalfVector(_) => quote! { Vec::new() },
        ScalarType::SparseVector(_) => quote! { Vec::new() },
        ScalarType::Bit(_) => quote! { Vec::new() },
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
    use smol_str::SmolStr;

    #[test]
    fn test_scalar_to_rust_type() {
        assert_eq!(scalar_to_rust_type(&ScalarType::Int).to_string(), "i32");
        assert_eq!(scalar_to_rust_type(&ScalarType::BigInt).to_string(), "i64");
        assert_eq!(scalar_to_rust_type(&ScalarType::Float).to_string(), "f64");
        assert_eq!(
            scalar_to_rust_type(&ScalarType::Boolean).to_string(),
            "bool"
        );
        assert_eq!(
            scalar_to_rust_type(&ScalarType::String).to_string(),
            "String"
        );
    }

    #[test]
    fn test_scalar_to_rust_type_all_scalars() {
        // Test all scalar types for comprehensive coverage
        assert_eq!(scalar_to_rust_type(&ScalarType::Int).to_string(), "i32");
        assert_eq!(scalar_to_rust_type(&ScalarType::BigInt).to_string(), "i64");
        assert_eq!(scalar_to_rust_type(&ScalarType::Float).to_string(), "f64");
        assert!(
            scalar_to_rust_type(&ScalarType::Decimal)
                .to_string()
                .contains("Decimal")
        );
        assert_eq!(
            scalar_to_rust_type(&ScalarType::Boolean).to_string(),
            "bool"
        );
        assert_eq!(
            scalar_to_rust_type(&ScalarType::String).to_string(),
            "String"
        );
        assert!(
            scalar_to_rust_type(&ScalarType::DateTime)
                .to_string()
                .contains("DateTime")
        );
        assert!(
            scalar_to_rust_type(&ScalarType::Date)
                .to_string()
                .contains("NaiveDate")
        );
        assert!(
            scalar_to_rust_type(&ScalarType::Time)
                .to_string()
                .contains("NaiveTime")
        );
        assert!(
            scalar_to_rust_type(&ScalarType::Json)
                .to_string()
                .contains("Value")
        );
        assert!(
            scalar_to_rust_type(&ScalarType::Bytes)
                .to_string()
                .contains("Vec")
        );
        assert!(
            scalar_to_rust_type(&ScalarType::Uuid)
                .to_string()
                .contains("Uuid")
        );

        // String-based ID types
        assert_eq!(scalar_to_rust_type(&ScalarType::Cuid).to_string(), "String");
        assert_eq!(
            scalar_to_rust_type(&ScalarType::Cuid2).to_string(),
            "String"
        );
        assert_eq!(
            scalar_to_rust_type(&ScalarType::NanoId).to_string(),
            "String"
        );
        assert_eq!(scalar_to_rust_type(&ScalarType::Ulid).to_string(), "String");
    }

    #[test]
    fn test_field_type_to_rust_scalar() {
        let field_type = FieldType::Scalar(ScalarType::Int);
        let result = field_type_to_rust(&field_type, &TypeModifier::Required);
        assert_eq!(result.to_string(), "i32");
    }

    #[test]
    fn test_field_type_to_rust_enum() {
        let field_type = FieldType::Enum(SmolStr::new("UserRole"));
        let result = field_type_to_rust(&field_type, &TypeModifier::Required);
        assert!(result.to_string().contains("UserRole"));
    }

    #[test]
    fn test_field_type_to_rust_model() {
        let field_type = FieldType::Model(SmolStr::new("User"));
        let result = field_type_to_rust(&field_type, &TypeModifier::Required);
        assert!(result.to_string().contains("User"));
    }

    #[test]
    fn test_field_type_to_rust_composite() {
        let field_type = FieldType::Composite(SmolStr::new("Address"));
        let result = field_type_to_rust(&field_type, &TypeModifier::Required);
        assert!(result.to_string().contains("Address"));
    }

    #[test]
    fn test_field_type_to_rust_unsupported() {
        let field_type = FieldType::Unsupported(SmolStr::new("GEOGRAPHY"));
        let result = field_type_to_rust(&field_type, &TypeModifier::Required);
        assert!(result.to_string().contains("UnsupportedType"));
        assert!(result.to_string().contains("GEOGRAPHY"));
    }

    #[test]
    fn test_field_type_to_rust_with_modifiers() {
        let field_type = FieldType::Scalar(ScalarType::String);

        // Required
        let required = field_type_to_rust(&field_type, &TypeModifier::Required);
        assert_eq!(required.to_string(), "String");

        // Optional
        let optional = field_type_to_rust(&field_type, &TypeModifier::Optional);
        assert!(optional.to_string().contains("Option"));

        // List
        let list = field_type_to_rust(&field_type, &TypeModifier::List);
        assert!(list.to_string().contains("Vec"));

        // Optional List
        let opt_list = field_type_to_rust(&field_type, &TypeModifier::OptionalList);
        assert!(opt_list.to_string().contains("Option"));
        assert!(opt_list.to_string().contains("Vec"));
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
    fn test_field_type_to_sql_type_scalars() {
        assert_eq!(
            field_type_to_sql_type(&FieldType::Scalar(ScalarType::Int)),
            "INT"
        );
        assert_eq!(
            field_type_to_sql_type(&FieldType::Scalar(ScalarType::BigInt)),
            "BIGINT"
        );
        assert_eq!(
            field_type_to_sql_type(&FieldType::Scalar(ScalarType::Float)),
            "DOUBLE PRECISION"
        );
        assert_eq!(
            field_type_to_sql_type(&FieldType::Scalar(ScalarType::Decimal)),
            "DECIMAL"
        );
        assert_eq!(
            field_type_to_sql_type(&FieldType::Scalar(ScalarType::Boolean)),
            "BOOLEAN"
        );
        assert_eq!(
            field_type_to_sql_type(&FieldType::Scalar(ScalarType::String)),
            "TEXT"
        );
        assert_eq!(
            field_type_to_sql_type(&FieldType::Scalar(ScalarType::DateTime)),
            "TIMESTAMPTZ"
        );
        assert_eq!(
            field_type_to_sql_type(&FieldType::Scalar(ScalarType::Date)),
            "DATE"
        );
        assert_eq!(
            field_type_to_sql_type(&FieldType::Scalar(ScalarType::Time)),
            "TIME"
        );
        assert_eq!(
            field_type_to_sql_type(&FieldType::Scalar(ScalarType::Json)),
            "JSONB"
        );
        assert_eq!(
            field_type_to_sql_type(&FieldType::Scalar(ScalarType::Bytes)),
            "BYTEA"
        );
        assert_eq!(
            field_type_to_sql_type(&FieldType::Scalar(ScalarType::Uuid)),
            "UUID"
        );
        // String-based ID types
        assert_eq!(
            field_type_to_sql_type(&FieldType::Scalar(ScalarType::Cuid)),
            "TEXT"
        );
        assert_eq!(
            field_type_to_sql_type(&FieldType::Scalar(ScalarType::Cuid2)),
            "TEXT"
        );
        assert_eq!(
            field_type_to_sql_type(&FieldType::Scalar(ScalarType::NanoId)),
            "TEXT"
        );
        assert_eq!(
            field_type_to_sql_type(&FieldType::Scalar(ScalarType::Ulid)),
            "TEXT"
        );
    }

    #[test]
    fn test_field_type_to_sql_type_complex() {
        assert_eq!(
            field_type_to_sql_type(&FieldType::Enum(SmolStr::new("Role"))),
            "TEXT"
        );
        assert_eq!(
            field_type_to_sql_type(&FieldType::Model(SmolStr::new("User"))),
            "INT"
        );
        assert_eq!(
            field_type_to_sql_type(&FieldType::Composite(SmolStr::new("Address"))),
            "JSONB"
        );
        assert_eq!(
            field_type_to_sql_type(&FieldType::Unsupported(SmolStr::new("CUSTOM_TYPE"))),
            "UNKNOWN"
        );
    }

    #[test]
    fn test_case_conversion() {
        assert_eq!(to_snake_case("UserProfile"), "user_profile");
        assert_eq!(to_pascal_case("user_profile"), "UserProfile");
        assert_eq!(to_screaming_snake("UserProfile"), "USER_PROFILE");
    }

    #[test]
    fn test_case_conversion_edge_cases() {
        // Single word
        assert_eq!(to_snake_case("User"), "user");
        assert_eq!(to_pascal_case("user"), "User");
        assert_eq!(to_screaming_snake("user"), "USER");

        // Multiple words
        assert_eq!(to_snake_case("HTTPRequest"), "http_request");
        assert_eq!(to_pascal_case("http_request"), "HttpRequest");
        assert_eq!(to_screaming_snake("HttpRequest"), "HTTP_REQUEST");

        // Already correct case
        assert_eq!(to_snake_case("already_snake"), "already_snake");
        assert_eq!(to_pascal_case("AlreadyPascal"), "AlreadyPascal");
    }

    #[test]
    fn test_default_value_for_type() {
        // Numeric types
        let int_default = default_value_for_type(&ScalarType::Int);
        assert_eq!(int_default.to_string(), "0");

        let bigint_default = default_value_for_type(&ScalarType::BigInt);
        assert_eq!(bigint_default.to_string(), "0");

        let float_default = default_value_for_type(&ScalarType::Float);
        assert_eq!(float_default.to_string(), "0.0");

        let decimal_default = default_value_for_type(&ScalarType::Decimal);
        assert_eq!(decimal_default.to_string(), "0.0");

        // Boolean
        let bool_default = default_value_for_type(&ScalarType::Boolean);
        assert_eq!(bool_default.to_string(), "false");

        // String
        let string_default = default_value_for_type(&ScalarType::String);
        assert!(string_default.to_string().contains("String :: new"));

        // DateTime types
        let datetime_default = default_value_for_type(&ScalarType::DateTime);
        assert!(datetime_default.to_string().contains("Utc :: now"));

        let date_default = default_value_for_type(&ScalarType::Date);
        assert!(date_default.to_string().contains("date_naive"));

        let time_default = default_value_for_type(&ScalarType::Time);
        assert!(time_default.to_string().contains("time"));

        // Json
        let json_default = default_value_for_type(&ScalarType::Json);
        assert!(json_default.to_string().contains("Null"));

        // Bytes
        let bytes_default = default_value_for_type(&ScalarType::Bytes);
        assert!(bytes_default.to_string().contains("Vec :: new"));

        // Uuid
        let uuid_default = default_value_for_type(&ScalarType::Uuid);
        assert!(uuid_default.to_string().contains("nil"));

        // String-based ID types
        let cuid_default = default_value_for_type(&ScalarType::Cuid);
        assert!(cuid_default.to_string().contains("String :: new"));

        let cuid2_default = default_value_for_type(&ScalarType::Cuid2);
        assert!(cuid2_default.to_string().contains("String :: new"));

        let nanoid_default = default_value_for_type(&ScalarType::NanoId);
        assert!(nanoid_default.to_string().contains("String :: new"));

        let ulid_default = default_value_for_type(&ScalarType::Ulid);
        assert!(ulid_default.to_string().contains("String :: new"));
    }

    #[test]
    fn test_supports_comparison() {
        assert!(supports_comparison(&ScalarType::Int));
        assert!(supports_comparison(&ScalarType::DateTime));
        assert!(!supports_comparison(&ScalarType::String));
        assert!(!supports_comparison(&ScalarType::Boolean));
    }

    #[test]
    fn test_supports_comparison_all_types() {
        // Comparison supported
        assert!(supports_comparison(&ScalarType::Int));
        assert!(supports_comparison(&ScalarType::BigInt));
        assert!(supports_comparison(&ScalarType::Float));
        assert!(supports_comparison(&ScalarType::Decimal));
        assert!(supports_comparison(&ScalarType::DateTime));
        assert!(supports_comparison(&ScalarType::Date));
        assert!(supports_comparison(&ScalarType::Time));

        // Comparison not supported
        assert!(!supports_comparison(&ScalarType::String));
        assert!(!supports_comparison(&ScalarType::Boolean));
        assert!(!supports_comparison(&ScalarType::Json));
        assert!(!supports_comparison(&ScalarType::Bytes));
        assert!(!supports_comparison(&ScalarType::Uuid));
        assert!(!supports_comparison(&ScalarType::Cuid));
        assert!(!supports_comparison(&ScalarType::Cuid2));
        assert!(!supports_comparison(&ScalarType::NanoId));
        assert!(!supports_comparison(&ScalarType::Ulid));
    }

    #[test]
    fn test_supports_string_ops() {
        assert!(supports_string_ops(&ScalarType::String));
        assert!(!supports_string_ops(&ScalarType::Int));
    }

    #[test]
    fn test_supports_string_ops_all_types() {
        // String ops supported
        assert!(supports_string_ops(&ScalarType::String));

        // String ops not supported
        assert!(!supports_string_ops(&ScalarType::Int));
        assert!(!supports_string_ops(&ScalarType::BigInt));
        assert!(!supports_string_ops(&ScalarType::Float));
        assert!(!supports_string_ops(&ScalarType::Decimal));
        assert!(!supports_string_ops(&ScalarType::Boolean));
        assert!(!supports_string_ops(&ScalarType::DateTime));
        assert!(!supports_string_ops(&ScalarType::Date));
        assert!(!supports_string_ops(&ScalarType::Time));
        assert!(!supports_string_ops(&ScalarType::Json));
        assert!(!supports_string_ops(&ScalarType::Bytes));
        assert!(!supports_string_ops(&ScalarType::Uuid));
        assert!(!supports_string_ops(&ScalarType::Cuid));
    }

    #[test]
    fn test_supports_in_op() {
        // Scalars that support IN
        assert!(supports_in_op(&FieldType::Scalar(ScalarType::Int)));
        assert!(supports_in_op(&FieldType::Scalar(ScalarType::BigInt)));
        assert!(supports_in_op(&FieldType::Scalar(ScalarType::Float)));
        assert!(supports_in_op(&FieldType::Scalar(ScalarType::Decimal)));
        assert!(supports_in_op(&FieldType::Scalar(ScalarType::Boolean)));
        assert!(supports_in_op(&FieldType::Scalar(ScalarType::String)));
        assert!(supports_in_op(&FieldType::Scalar(ScalarType::DateTime)));
        assert!(supports_in_op(&FieldType::Scalar(ScalarType::Date)));
        assert!(supports_in_op(&FieldType::Scalar(ScalarType::Time)));
        assert!(supports_in_op(&FieldType::Scalar(ScalarType::Uuid)));
        assert!(supports_in_op(&FieldType::Scalar(ScalarType::Cuid)));

        // Scalars that don't support IN
        assert!(!supports_in_op(&FieldType::Scalar(ScalarType::Json)));
        assert!(!supports_in_op(&FieldType::Scalar(ScalarType::Bytes)));

        // Enum supports IN
        assert!(supports_in_op(&FieldType::Enum(SmolStr::new("Role"))));

        // Model and Composite don't support IN
        assert!(!supports_in_op(&FieldType::Model(SmolStr::new("User"))));
        assert!(!supports_in_op(&FieldType::Composite(SmolStr::new(
            "Address"
        ))));
        assert!(!supports_in_op(&FieldType::Unsupported(SmolStr::new(
            "CUSTOM"
        ))));
    }
}
