//! Structured fuzzing for the Prax schema parser.
//!
//! This target generates semi-valid schema inputs using the `arbitrary` crate
//! to explore more interesting code paths.
//!
//! Run with:
//! ```bash
//! cargo +nightly fuzz run fuzz_schema_structured
//! ```

#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use prax_schema::parser::parse_schema;

/// A generated field type.
#[derive(Debug, Arbitrary)]
enum FuzzFieldType {
    Int,
    String,
    Boolean,
    Float,
    DateTime,
    Json,
    Bytes,
    BigInt,
    Decimal,
}

impl FuzzFieldType {
    fn to_string(&self) -> &'static str {
        match self {
            Self::Int => "Int",
            Self::String => "String",
            Self::Boolean => "Boolean",
            Self::Float => "Float",
            Self::DateTime => "DateTime",
            Self::Json => "Json",
            Self::Bytes => "Bytes",
            Self::BigInt => "BigInt",
            Self::Decimal => "Decimal",
        }
    }
}

/// A generated field modifier.
#[derive(Debug, Arbitrary)]
enum FuzzFieldModifier {
    None,
    Optional,
    List,
    OptionalList,
}

impl FuzzFieldModifier {
    fn apply(&self, type_name: &str) -> String {
        match self {
            Self::None => type_name.to_string(),
            Self::Optional => format!("{}?", type_name),
            Self::List => format!("{}[]", type_name),
            Self::OptionalList => format!("{}[]?", type_name),
        }
    }
}

/// A generated field attribute.
#[derive(Debug, Arbitrary)]
enum FuzzFieldAttribute {
    None,
    Id,
    Auto,
    Unique,
    Default(FuzzDefaultValue),
    UpdatedAt,
    Map(String),
}

impl FuzzFieldAttribute {
    fn to_string(&self) -> String {
        match self {
            Self::None => String::new(),
            Self::Id => " @id".to_string(),
            Self::Auto => " @auto".to_string(),
            Self::Unique => " @unique".to_string(),
            Self::Default(val) => format!(" @default({})", val.to_string()),
            Self::UpdatedAt => " @updatedAt".to_string(),
            Self::Map(name) => format!(" @map(\"{}\")", sanitize_string(name)),
        }
    }
}

/// A generated default value.
#[derive(Debug, Arbitrary)]
enum FuzzDefaultValue {
    Now,
    AutoIncrement,
    Uuid,
    Cuid,
    IntLiteral(i32),
    BoolLiteral(bool),
    StringLiteral(String),
}

impl FuzzDefaultValue {
    fn to_string(&self) -> String {
        match self {
            Self::Now => "now()".to_string(),
            Self::AutoIncrement => "autoincrement()".to_string(),
            Self::Uuid => "uuid()".to_string(),
            Self::Cuid => "cuid()".to_string(),
            Self::IntLiteral(i) => i.to_string(),
            Self::BoolLiteral(b) => b.to_string(),
            Self::StringLiteral(s) => format!("\"{}\"", sanitize_string(s)),
        }
    }
}

/// A generated field.
#[derive(Debug, Arbitrary)]
struct FuzzField {
    name: String,
    field_type: FuzzFieldType,
    modifier: FuzzFieldModifier,
    attribute: FuzzFieldAttribute,
}

impl FuzzField {
    fn to_string(&self) -> String {
        let name = sanitize_identifier(&self.name);
        let type_str = self.modifier.apply(self.field_type.to_string());
        let attr = self.attribute.to_string();
        format!("    {} {}{}", name, type_str, attr)
    }
}

/// A generated model.
#[derive(Debug, Arbitrary)]
struct FuzzModel {
    name: String,
    fields: Vec<FuzzField>,
}

impl FuzzModel {
    fn to_string(&self) -> String {
        let name = sanitize_identifier(&self.name);
        let fields: Vec<String> = self.fields.iter().map(|f| f.to_string()).collect();
        format!("model {} {{\n{}\n}}", name, fields.join("\n"))
    }
}

/// A generated enum variant.
#[derive(Debug, Arbitrary)]
struct FuzzEnumVariant {
    name: String,
}

impl FuzzEnumVariant {
    fn to_string(&self) -> String {
        format!("    {}", sanitize_identifier(&self.name).to_uppercase())
    }
}

/// A generated enum.
#[derive(Debug, Arbitrary)]
struct FuzzEnum {
    name: String,
    variants: Vec<FuzzEnumVariant>,
}

impl FuzzEnum {
    fn to_string(&self) -> String {
        let name = sanitize_identifier(&self.name);
        let variants: Vec<String> = self.variants.iter().map(|v| v.to_string()).collect();
        format!("enum {} {{\n{}\n}}", name, variants.join("\n"))
    }
}

/// A generated schema.
#[derive(Debug, Arbitrary)]
struct FuzzSchema {
    models: Vec<FuzzModel>,
    enums: Vec<FuzzEnum>,
}

impl FuzzSchema {
    fn to_string(&self) -> String {
        let mut parts = Vec::new();

        for e in &self.enums {
            if !e.variants.is_empty() {
                parts.push(e.to_string());
            }
        }

        for m in &self.models {
            if !m.fields.is_empty() {
                parts.push(m.to_string());
            }
        }

        parts.join("\n\n")
    }
}

/// Sanitize a string to be a valid identifier.
fn sanitize_identifier(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if i == 0 {
            if c.is_ascii_alphabetic() {
                result.push(c);
            } else {
                result.push('x');
            }
        } else if c.is_ascii_alphanumeric() || c == '_' {
            result.push(c);
        }
    }
    if result.is_empty() {
        "field".to_string()
    } else {
        result
    }
}

/// Sanitize a string for use in a string literal.
fn sanitize_string(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '"' | '\\' | '\n' | '\r'))
        .take(50) // Limit length
        .collect()
}

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);

    if let Ok(schema) = FuzzSchema::arbitrary(&mut unstructured) {
        let schema_str = schema.to_string();

        // The parser should never panic
        let _ = parse_schema(&schema_str);
    }
});

