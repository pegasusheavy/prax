//! Type definitions for the Prax schema AST.

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

/// A span in the source code for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    /// Start offset in bytes.
    pub start: usize,
    /// End offset in bytes.
    pub end: usize,
}

impl Span {
    /// Create a new span.
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Get the length of the span.
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Check if the span is empty.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Merge two spans into one that covers both.
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

impl From<(usize, usize)> for Span {
    fn from((start, end): (usize, usize)) -> Self {
        Self { start, end }
    }
}

/// An identifier with source location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ident {
    /// The identifier name.
    pub name: SmolStr,
    /// Source location.
    pub span: Span,
}

impl Ident {
    /// Create a new identifier.
    pub fn new(name: impl Into<SmolStr>, span: Span) -> Self {
        Self {
            name: name.into(),
            span,
        }
    }

    /// Get the name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.name
    }
}

impl std::fmt::Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Scalar types supported by Prax.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScalarType {
    /// Integer type (maps to INT/INTEGER).
    Int,
    /// Big integer type (maps to BIGINT).
    BigInt,
    /// Floating point type (maps to FLOAT/REAL).
    Float,
    /// Decimal type for precise calculations (maps to DECIMAL/NUMERIC).
    Decimal,
    /// String type (maps to VARCHAR/TEXT).
    String,
    /// Boolean type.
    Boolean,
    /// Date and time type.
    DateTime,
    /// Date only type.
    Date,
    /// Time only type.
    Time,
    /// JSON type.
    Json,
    /// Binary/Bytes type.
    Bytes,
    /// UUID type.
    Uuid,
}

impl ScalarType {
    /// Parse a scalar type from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Int" => Some(Self::Int),
            "BigInt" => Some(Self::BigInt),
            "Float" => Some(Self::Float),
            "Decimal" => Some(Self::Decimal),
            "String" => Some(Self::String),
            "Boolean" | "Bool" => Some(Self::Boolean),
            "DateTime" => Some(Self::DateTime),
            "Date" => Some(Self::Date),
            "Time" => Some(Self::Time),
            "Json" => Some(Self::Json),
            "Bytes" => Some(Self::Bytes),
            "Uuid" | "UUID" => Some(Self::Uuid),
            _ => None,
        }
    }

    /// Get the type name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Int => "Int",
            Self::BigInt => "BigInt",
            Self::Float => "Float",
            Self::Decimal => "Decimal",
            Self::String => "String",
            Self::Boolean => "Boolean",
            Self::DateTime => "DateTime",
            Self::Date => "Date",
            Self::Time => "Time",
            Self::Json => "Json",
            Self::Bytes => "Bytes",
            Self::Uuid => "Uuid",
        }
    }
}

impl std::fmt::Display for ScalarType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A field type in the schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    /// A scalar type (Int, String, etc.).
    Scalar(ScalarType),
    /// A reference to an enum defined in the schema.
    Enum(SmolStr),
    /// A reference to a model (relation).
    Model(SmolStr),
    /// A reference to a composite type.
    Composite(SmolStr),
    /// An unsupported/unknown type (for forward compatibility).
    Unsupported(SmolStr),
}

impl FieldType {
    /// Check if this is a scalar type.
    pub fn is_scalar(&self) -> bool {
        matches!(self, Self::Scalar(_))
    }

    /// Check if this is a relation to another model.
    pub fn is_relation(&self) -> bool {
        matches!(self, Self::Model(_))
    }

    /// Check if this is an enum type.
    pub fn is_enum(&self) -> bool {
        matches!(self, Self::Enum(_))
    }

    /// Get the type name as a string.
    pub fn type_name(&self) -> &str {
        match self {
            Self::Scalar(s) => s.as_str(),
            Self::Enum(name) | Self::Model(name) | Self::Composite(name) | Self::Unsupported(name) => name.as_str(),
        }
    }
}

impl std::fmt::Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.type_name())
    }
}

/// Modifier for field types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeModifier {
    /// Required field (no modifier).
    Required,
    /// Optional field (`?` suffix).
    Optional,
    /// List/Array field (`[]` suffix).
    List,
    /// Optional list field (`[]?` suffix - rare but supported).
    OptionalList,
}

impl TypeModifier {
    /// Check if the field is optional.
    pub fn is_optional(&self) -> bool {
        matches!(self, Self::Optional | Self::OptionalList)
    }

    /// Check if the field is a list.
    pub fn is_list(&self) -> bool {
        matches!(self, Self::List | Self::OptionalList)
    }
}

/// A documentation comment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Documentation {
    /// The documentation text (without `///` prefix).
    pub text: String,
    /// Source location.
    pub span: Span,
}

impl Documentation {
    /// Create new documentation.
    pub fn new(text: impl Into<String>, span: Span) -> Self {
        Self {
            text: text.into(),
            span,
        }
    }
}

