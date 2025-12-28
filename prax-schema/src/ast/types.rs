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
    /// UUID type (128-bit universally unique identifier).
    Uuid,
    /// CUID type (collision-resistant unique identifier).
    Cuid,
    /// CUID2 type (next generation collision-resistant unique identifier).
    Cuid2,
    /// NanoID type (URL-friendly unique identifier).
    NanoId,
    /// ULID type (Universally Unique Lexicographically Sortable Identifier).
    Ulid,

    // ==================== PostgreSQL Extension Types ====================
    // These types require the corresponding PostgreSQL extension to be enabled.
    /// Vector type for AI/ML embeddings (requires `vector` extension).
    /// Stores dense vectors of float4 values.
    /// Usage: `embedding Vector(1536)` for OpenAI embeddings.
    Vector(Option<u32>),
    /// Half-precision vector type (requires `vector` extension).
    /// Stores dense vectors of float2 values, using half the storage of Vector.
    /// Usage: `embedding HalfVector(1536)` for memory-efficient embeddings.
    HalfVector(Option<u32>),
    /// Sparse vector type (requires `vector` extension).
    /// Stores sparse vectors with explicit indices and values.
    /// Usage: `features SparseVector(10000)` for high-dimensional sparse data.
    SparseVector(Option<u32>),
    /// Bit vector type (requires `vector` extension).
    /// Stores binary vectors for binary quantization.
    /// Usage: `hash Bit(256)` for binary hash storage.
    Bit(Option<u32>),
}

impl ScalarType {
    /// Parse a scalar type from a string.
    /// For parameterized types like Vector(1536), use `parse_with_param`.
    #[allow(clippy::should_implement_trait)]
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
            "Cuid" | "CUID" => Some(Self::Cuid),
            "Cuid2" | "CUID2" => Some(Self::Cuid2),
            "NanoId" | "NanoID" | "Nanoid" => Some(Self::NanoId),
            "Ulid" | "ULID" => Some(Self::Ulid),
            // Vector types without dimension
            "Vector" => Some(Self::Vector(None)),
            "HalfVector" | "Halfvec" => Some(Self::HalfVector(None)),
            "SparseVector" | "Sparsevec" => Some(Self::SparseVector(None)),
            "Bit" => Some(Self::Bit(None)),
            _ => None,
        }
    }

    /// Parse a scalar type with an optional dimension parameter.
    /// Handles types like `Vector(1536)`, `HalfVector(768)`, etc.
    pub fn parse_with_param(name: &str, param: Option<u32>) -> Option<Self> {
        match name {
            "Vector" => Some(Self::Vector(param)),
            "HalfVector" | "Halfvec" => Some(Self::HalfVector(param)),
            "SparseVector" | "Sparsevec" => Some(Self::SparseVector(param)),
            "Bit" => Some(Self::Bit(param)),
            _ => Self::from_str(name),
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
            Self::Cuid => "Cuid",
            Self::Cuid2 => "Cuid2",
            Self::NanoId => "NanoId",
            Self::Ulid => "Ulid",
            Self::Vector(_) => "Vector",
            Self::HalfVector(_) => "HalfVector",
            Self::SparseVector(_) => "SparseVector",
            Self::Bit(_) => "Bit",
        }
    }

    /// Check if this type is an identifier type (UUID, CUID, etc.).
    pub fn is_id_type(&self) -> bool {
        matches!(
            self,
            Self::Uuid | Self::Cuid | Self::Cuid2 | Self::NanoId | Self::Ulid
        )
    }

    /// Check if this type requires the `vector` PostgreSQL extension.
    pub fn requires_vector_extension(&self) -> bool {
        matches!(
            self,
            Self::Vector(_) | Self::HalfVector(_) | Self::SparseVector(_) | Self::Bit(_)
        )
    }

    /// Get the dimension for vector types, if specified.
    pub fn dimension(&self) -> Option<u32> {
        match self {
            Self::Vector(d) | Self::HalfVector(d) | Self::SparseVector(d) | Self::Bit(d) => *d,
            _ => None,
        }
    }

    /// Get the PostgreSQL type name for this scalar type.
    pub fn postgres_type(&self) -> String {
        match self {
            Self::Int => "INTEGER".to_string(),
            Self::BigInt => "BIGINT".to_string(),
            Self::Float => "DOUBLE PRECISION".to_string(),
            Self::Decimal => "DECIMAL".to_string(),
            Self::String => "TEXT".to_string(),
            Self::Boolean => "BOOLEAN".to_string(),
            Self::DateTime => "TIMESTAMP WITH TIME ZONE".to_string(),
            Self::Date => "DATE".to_string(),
            Self::Time => "TIME".to_string(),
            Self::Json => "JSONB".to_string(),
            Self::Bytes => "BYTEA".to_string(),
            Self::Uuid => "UUID".to_string(),
            Self::Cuid | Self::Cuid2 | Self::NanoId | Self::Ulid => "TEXT".to_string(),
            Self::Vector(Some(dim)) => format!("vector({})", dim),
            Self::Vector(None) => "vector".to_string(),
            Self::HalfVector(Some(dim)) => format!("halfvec({})", dim),
            Self::HalfVector(None) => "halfvec".to_string(),
            Self::SparseVector(Some(dim)) => format!("sparsevec({})", dim),
            Self::SparseVector(None) => "sparsevec".to_string(),
            Self::Bit(Some(dim)) => format!("bit({})", dim),
            Self::Bit(None) => "bit".to_string(),
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
            Self::Enum(name)
            | Self::Model(name)
            | Self::Composite(name)
            | Self::Unsupported(name) => name.as_str(),
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

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Span Tests ====================

    #[test]
    fn test_span_new() {
        let span = Span::new(10, 20);
        assert_eq!(span.start, 10);
        assert_eq!(span.end, 20);
    }

    #[test]
    fn test_span_len() {
        let span = Span::new(5, 15);
        assert_eq!(span.len(), 10);
    }

    #[test]
    fn test_span_len_zero() {
        let span = Span::new(10, 10);
        assert_eq!(span.len(), 0);
    }

    #[test]
    fn test_span_is_empty_true() {
        let span = Span::new(5, 5);
        assert!(span.is_empty());
    }

    #[test]
    fn test_span_is_empty_false() {
        let span = Span::new(5, 10);
        assert!(!span.is_empty());
    }

    #[test]
    fn test_span_merge_adjacent() {
        let span1 = Span::new(0, 10);
        let span2 = Span::new(10, 20);
        let merged = span1.merge(span2);
        assert_eq!(merged.start, 0);
        assert_eq!(merged.end, 20);
    }

    #[test]
    fn test_span_merge_overlapping() {
        let span1 = Span::new(5, 15);
        let span2 = Span::new(10, 25);
        let merged = span1.merge(span2);
        assert_eq!(merged.start, 5);
        assert_eq!(merged.end, 25);
    }

    #[test]
    fn test_span_merge_disjoint() {
        let span1 = Span::new(0, 5);
        let span2 = Span::new(20, 30);
        let merged = span1.merge(span2);
        assert_eq!(merged.start, 0);
        assert_eq!(merged.end, 30);
    }

    #[test]
    fn test_span_from_tuple() {
        let span: Span = (10, 20).into();
        assert_eq!(span.start, 10);
        assert_eq!(span.end, 20);
    }

    #[test]
    fn test_span_equality() {
        let span1 = Span::new(10, 20);
        let span2 = Span::new(10, 20);
        let span3 = Span::new(10, 25);
        assert_eq!(span1, span2);
        assert_ne!(span1, span3);
    }

    #[test]
    fn test_span_clone() {
        let span1 = Span::new(10, 20);
        let span2 = span1;
        assert_eq!(span1, span2);
    }

    // ==================== Ident Tests ====================

    #[test]
    fn test_ident_new() {
        let ident = Ident::new("user_id", Span::new(0, 7));
        assert_eq!(ident.name.as_str(), "user_id");
        assert_eq!(ident.span.start, 0);
        assert_eq!(ident.span.end, 7);
    }

    #[test]
    fn test_ident_as_str() {
        let ident = Ident::new("field_name", Span::new(0, 10));
        assert_eq!(ident.as_str(), "field_name");
    }

    #[test]
    fn test_ident_display() {
        let ident = Ident::new("MyModel", Span::new(0, 7));
        assert_eq!(format!("{}", ident), "MyModel");
    }

    #[test]
    fn test_ident_equality() {
        let ident1 = Ident::new("name", Span::new(0, 4));
        let ident2 = Ident::new("name", Span::new(0, 4));
        let ident3 = Ident::new("name", Span::new(5, 9));
        let ident4 = Ident::new("other", Span::new(0, 5));

        assert_eq!(ident1, ident2);
        assert_ne!(ident1, ident3); // different span
        assert_ne!(ident1, ident4); // different name
    }

    #[test]
    fn test_ident_from_string() {
        let name = String::from("dynamic_name");
        let ident = Ident::new(name, Span::new(0, 12));
        assert_eq!(ident.as_str(), "dynamic_name");
    }

    // ==================== ScalarType Tests ====================

    #[test]
    fn test_scalar_type_from_str_int() {
        assert_eq!(ScalarType::from_str("Int"), Some(ScalarType::Int));
    }

    #[test]
    fn test_scalar_type_from_str_bigint() {
        assert_eq!(ScalarType::from_str("BigInt"), Some(ScalarType::BigInt));
    }

    #[test]
    fn test_scalar_type_from_str_float() {
        assert_eq!(ScalarType::from_str("Float"), Some(ScalarType::Float));
    }

    #[test]
    fn test_scalar_type_from_str_decimal() {
        assert_eq!(ScalarType::from_str("Decimal"), Some(ScalarType::Decimal));
    }

    #[test]
    fn test_scalar_type_from_str_string() {
        assert_eq!(ScalarType::from_str("String"), Some(ScalarType::String));
    }

    #[test]
    fn test_scalar_type_from_str_boolean() {
        assert_eq!(ScalarType::from_str("Boolean"), Some(ScalarType::Boolean));
    }

    #[test]
    fn test_scalar_type_from_str_bool_alias() {
        assert_eq!(ScalarType::from_str("Bool"), Some(ScalarType::Boolean));
    }

    #[test]
    fn test_scalar_type_from_str_datetime() {
        assert_eq!(ScalarType::from_str("DateTime"), Some(ScalarType::DateTime));
    }

    #[test]
    fn test_scalar_type_from_str_date() {
        assert_eq!(ScalarType::from_str("Date"), Some(ScalarType::Date));
    }

    #[test]
    fn test_scalar_type_from_str_time() {
        assert_eq!(ScalarType::from_str("Time"), Some(ScalarType::Time));
    }

    #[test]
    fn test_scalar_type_from_str_json() {
        assert_eq!(ScalarType::from_str("Json"), Some(ScalarType::Json));
    }

    #[test]
    fn test_scalar_type_from_str_bytes() {
        assert_eq!(ScalarType::from_str("Bytes"), Some(ScalarType::Bytes));
    }

    #[test]
    fn test_scalar_type_from_str_uuid() {
        assert_eq!(ScalarType::from_str("Uuid"), Some(ScalarType::Uuid));
    }

    #[test]
    fn test_scalar_type_from_str_uuid_uppercase() {
        assert_eq!(ScalarType::from_str("UUID"), Some(ScalarType::Uuid));
    }

    #[test]
    fn test_scalar_type_from_str_cuid() {
        assert_eq!(ScalarType::from_str("Cuid"), Some(ScalarType::Cuid));
        assert_eq!(ScalarType::from_str("CUID"), Some(ScalarType::Cuid));
    }

    #[test]
    fn test_scalar_type_from_str_cuid2() {
        assert_eq!(ScalarType::from_str("Cuid2"), Some(ScalarType::Cuid2));
        assert_eq!(ScalarType::from_str("CUID2"), Some(ScalarType::Cuid2));
    }

    #[test]
    fn test_scalar_type_from_str_nanoid() {
        assert_eq!(ScalarType::from_str("NanoId"), Some(ScalarType::NanoId));
        assert_eq!(ScalarType::from_str("NanoID"), Some(ScalarType::NanoId));
        assert_eq!(ScalarType::from_str("Nanoid"), Some(ScalarType::NanoId));
    }

    #[test]
    fn test_scalar_type_from_str_ulid() {
        assert_eq!(ScalarType::from_str("Ulid"), Some(ScalarType::Ulid));
        assert_eq!(ScalarType::from_str("ULID"), Some(ScalarType::Ulid));
    }

    #[test]
    fn test_scalar_type_from_str_unknown() {
        assert_eq!(ScalarType::from_str("Unknown"), None);
        assert_eq!(ScalarType::from_str("int"), None); // case sensitive
        assert_eq!(ScalarType::from_str(""), None);
    }

    #[test]
    fn test_scalar_type_as_str() {
        assert_eq!(ScalarType::Int.as_str(), "Int");
        assert_eq!(ScalarType::BigInt.as_str(), "BigInt");
        assert_eq!(ScalarType::Float.as_str(), "Float");
        assert_eq!(ScalarType::Decimal.as_str(), "Decimal");
        assert_eq!(ScalarType::String.as_str(), "String");
        assert_eq!(ScalarType::Boolean.as_str(), "Boolean");
        assert_eq!(ScalarType::DateTime.as_str(), "DateTime");
        assert_eq!(ScalarType::Date.as_str(), "Date");
        assert_eq!(ScalarType::Time.as_str(), "Time");
        assert_eq!(ScalarType::Json.as_str(), "Json");
        assert_eq!(ScalarType::Bytes.as_str(), "Bytes");
        assert_eq!(ScalarType::Uuid.as_str(), "Uuid");
        assert_eq!(ScalarType::Cuid.as_str(), "Cuid");
        assert_eq!(ScalarType::Cuid2.as_str(), "Cuid2");
        assert_eq!(ScalarType::NanoId.as_str(), "NanoId");
        assert_eq!(ScalarType::Ulid.as_str(), "Ulid");
    }

    #[test]
    fn test_scalar_type_is_id_type() {
        assert!(ScalarType::Uuid.is_id_type());
        assert!(ScalarType::Cuid.is_id_type());
        assert!(ScalarType::Cuid2.is_id_type());
        assert!(ScalarType::NanoId.is_id_type());
        assert!(ScalarType::Ulid.is_id_type());
        assert!(!ScalarType::Int.is_id_type());
        assert!(!ScalarType::String.is_id_type());
    }

    #[test]
    fn test_scalar_type_display() {
        assert_eq!(format!("{}", ScalarType::Int), "Int");
        assert_eq!(format!("{}", ScalarType::String), "String");
        assert_eq!(format!("{}", ScalarType::DateTime), "DateTime");
    }

    #[test]
    fn test_scalar_type_equality() {
        assert_eq!(ScalarType::Int, ScalarType::Int);
        assert_ne!(ScalarType::Int, ScalarType::String);
    }

    // ==================== FieldType Tests ====================

    #[test]
    fn test_field_type_scalar() {
        let ft = FieldType::Scalar(ScalarType::Int);
        assert!(ft.is_scalar());
        assert!(!ft.is_relation());
        assert!(!ft.is_enum());
        assert_eq!(ft.type_name(), "Int");
    }

    #[test]
    fn test_field_type_enum() {
        let ft = FieldType::Enum("Role".into());
        assert!(!ft.is_scalar());
        assert!(!ft.is_relation());
        assert!(ft.is_enum());
        assert_eq!(ft.type_name(), "Role");
    }

    #[test]
    fn test_field_type_model() {
        let ft = FieldType::Model("User".into());
        assert!(!ft.is_scalar());
        assert!(ft.is_relation());
        assert!(!ft.is_enum());
        assert_eq!(ft.type_name(), "User");
    }

    #[test]
    fn test_field_type_composite() {
        let ft = FieldType::Composite("Address".into());
        assert!(!ft.is_scalar());
        assert!(!ft.is_relation());
        assert!(!ft.is_enum());
        assert_eq!(ft.type_name(), "Address");
    }

    #[test]
    fn test_field_type_unsupported() {
        let ft = FieldType::Unsupported("CustomType".into());
        assert!(!ft.is_scalar());
        assert!(!ft.is_relation());
        assert!(!ft.is_enum());
        assert_eq!(ft.type_name(), "CustomType");
    }

    #[test]
    fn test_field_type_display() {
        assert_eq!(
            format!("{}", FieldType::Scalar(ScalarType::String)),
            "String"
        );
        assert_eq!(format!("{}", FieldType::Enum("Status".into())), "Status");
        assert_eq!(format!("{}", FieldType::Model("Post".into())), "Post");
    }

    #[test]
    fn test_field_type_equality() {
        let ft1 = FieldType::Scalar(ScalarType::Int);
        let ft2 = FieldType::Scalar(ScalarType::Int);
        let ft3 = FieldType::Scalar(ScalarType::String);

        assert_eq!(ft1, ft2);
        assert_ne!(ft1, ft3);
    }

    // ==================== TypeModifier Tests ====================

    #[test]
    fn test_type_modifier_required() {
        let tm = TypeModifier::Required;
        assert!(!tm.is_optional());
        assert!(!tm.is_list());
    }

    #[test]
    fn test_type_modifier_optional() {
        let tm = TypeModifier::Optional;
        assert!(tm.is_optional());
        assert!(!tm.is_list());
    }

    #[test]
    fn test_type_modifier_list() {
        let tm = TypeModifier::List;
        assert!(!tm.is_optional());
        assert!(tm.is_list());
    }

    #[test]
    fn test_type_modifier_optional_list() {
        let tm = TypeModifier::OptionalList;
        assert!(tm.is_optional());
        assert!(tm.is_list());
    }

    #[test]
    fn test_type_modifier_equality() {
        assert_eq!(TypeModifier::Required, TypeModifier::Required);
        assert_eq!(TypeModifier::Optional, TypeModifier::Optional);
        assert_ne!(TypeModifier::Required, TypeModifier::Optional);
    }

    // ==================== Documentation Tests ====================

    #[test]
    fn test_documentation_new() {
        let doc = Documentation::new("This is a doc comment", Span::new(0, 21));
        assert_eq!(doc.text, "This is a doc comment");
        assert_eq!(doc.span.start, 0);
        assert_eq!(doc.span.end, 21);
    }

    #[test]
    fn test_documentation_from_string() {
        let text = String::from("Dynamic documentation");
        let doc = Documentation::new(text, Span::new(0, 21));
        assert_eq!(doc.text, "Dynamic documentation");
    }

    #[test]
    fn test_documentation_equality() {
        let doc1 = Documentation::new("Same text", Span::new(0, 9));
        let doc2 = Documentation::new("Same text", Span::new(0, 9));
        let doc3 = Documentation::new("Different", Span::new(0, 9));

        assert_eq!(doc1, doc2);
        assert_ne!(doc1, doc3);
    }

    #[test]
    fn test_documentation_multiline() {
        let doc = Documentation::new("Line 1\nLine 2\nLine 3", Span::new(0, 20));
        assert!(doc.text.contains('\n'));
        assert!(doc.text.starts_with("Line 1"));
        assert!(doc.text.ends_with("Line 3"));
    }
}
