//! Attribute definitions for the Prax schema AST.

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use super::{Ident, Span};

/// An attribute argument value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AttributeValue {
    /// A string literal.
    String(String),
    /// An integer literal.
    Int(i64),
    /// A float literal.
    Float(f64),
    /// A boolean literal.
    Boolean(bool),
    /// An identifier/constant reference (e.g., enum value).
    Ident(SmolStr),
    /// A function call (e.g., `now()`, `uuid()`).
    Function(SmolStr, Vec<AttributeValue>),
    /// An array of values.
    Array(Vec<AttributeValue>),
    /// A field reference (e.g., `[field_name]`).
    FieldRef(SmolStr),
    /// A list of field references (e.g., `[field1, field2]`).
    FieldRefList(Vec<SmolStr>),
}

impl AttributeValue {
    /// Try to get the value as a string.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get the value as an integer.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Try to get the value as a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to get the value as an identifier.
    pub fn as_ident(&self) -> Option<&str> {
        match self {
            Self::Ident(s) => Some(s),
            _ => None,
        }
    }
}

/// An attribute argument (named or positional).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttributeArg {
    /// Argument name (None for positional arguments).
    pub name: Option<Ident>,
    /// Argument value.
    pub value: AttributeValue,
    /// Source location.
    pub span: Span,
}

impl AttributeArg {
    /// Create a positional argument.
    pub fn positional(value: AttributeValue, span: Span) -> Self {
        Self {
            name: None,
            value,
            span,
        }
    }

    /// Create a named argument.
    pub fn named(name: Ident, value: AttributeValue, span: Span) -> Self {
        Self {
            name: Some(name),
            value,
            span,
        }
    }

    /// Check if this is a positional argument.
    pub fn is_positional(&self) -> bool {
        self.name.is_none()
    }
}

/// An attribute applied to a field, model, or enum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    /// Attribute name (without `@` prefix).
    pub name: Ident,
    /// Attribute arguments.
    pub args: Vec<AttributeArg>,
    /// Source location (including `@`).
    pub span: Span,
}

impl Attribute {
    /// Create a new attribute.
    pub fn new(name: Ident, args: Vec<AttributeArg>, span: Span) -> Self {
        Self { name, args, span }
    }

    /// Create an attribute with no arguments.
    pub fn simple(name: Ident, span: Span) -> Self {
        Self {
            name,
            args: vec![],
            span,
        }
    }

    /// Get the attribute name as a string.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Check if this attribute has the given name.
    pub fn is(&self, name: &str) -> bool {
        self.name.as_str() == name
    }

    /// Get the first positional argument.
    pub fn first_arg(&self) -> Option<&AttributeValue> {
        self.args.first().map(|a| &a.value)
    }

    /// Get a named argument by name.
    pub fn get_arg(&self, name: &str) -> Option<&AttributeValue> {
        self.args
            .iter()
            .find(|a| a.name.as_ref().map(|n| n.as_str()) == Some(name))
            .map(|a| &a.value)
    }

    /// Check if this is a field-level attribute.
    pub fn is_field_attribute(&self) -> bool {
        matches!(
            self.name(),
            "id" | "auto"
                | "unique"
                | "index"
                | "default"
                | "updated_at"
                | "omit"
                | "map"
                | "db"
                | "relation"
        )
    }

    /// Check if this is a model-level attribute (prefixed with `@@`).
    pub fn is_model_attribute(&self) -> bool {
        matches!(
            self.name(),
            "map" | "index" | "unique" | "id" | "search" | "sql"
        )
    }
}

/// Common field attributes.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FieldAttributes {
    /// This field is the primary key.
    pub is_id: bool,
    /// This field auto-increments (for integer IDs).
    pub is_auto: bool,
    /// This field has a unique constraint.
    pub is_unique: bool,
    /// This field is indexed.
    pub is_indexed: bool,
    /// This field is updated automatically on record update.
    pub is_updated_at: bool,
    /// This field should be omitted from default selections.
    pub is_omit: bool,
    /// Default value expression.
    pub default: Option<AttributeValue>,
    /// Database column name mapping.
    pub map: Option<String>,
    /// Native database type (e.g., `@db.VarChar(255)`).
    pub native_type: Option<NativeType>,
    /// Relation attributes.
    pub relation: Option<RelationAttribute>,
}

/// Native database type specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeType {
    /// Type name (e.g., "VarChar", "Text", "Decimal").
    pub name: SmolStr,
    /// Type arguments (e.g., length, precision, scale).
    pub args: Vec<AttributeValue>,
}

impl NativeType {
    /// Create a new native type.
    pub fn new(name: impl Into<SmolStr>, args: Vec<AttributeValue>) -> Self {
        Self {
            name: name.into(),
            args,
        }
    }
}

/// Relation attribute details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationAttribute {
    /// Relation name (for disambiguation).
    pub name: Option<String>,
    /// Fields on this model that reference the other model.
    pub fields: Vec<SmolStr>,
    /// Fields on the other model being referenced.
    pub references: Vec<SmolStr>,
    /// On delete action.
    pub on_delete: Option<ReferentialAction>,
    /// On update action.
    pub on_update: Option<ReferentialAction>,
}

/// Referential actions for relations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferentialAction {
    /// Cascade the operation.
    Cascade,
    /// Restrict the operation (error if references exist).
    Restrict,
    /// No action (deferred check).
    NoAction,
    /// Set to null.
    SetNull,
    /// Set to default value.
    SetDefault,
}

impl ReferentialAction {
    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Cascade" => Some(Self::Cascade),
            "Restrict" => Some(Self::Restrict),
            "NoAction" => Some(Self::NoAction),
            "SetNull" => Some(Self::SetNull),
            "SetDefault" => Some(Self::SetDefault),
            _ => None,
        }
    }

    /// Get the action name.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cascade => "CASCADE",
            Self::Restrict => "RESTRICT",
            Self::NoAction => "NO ACTION",
            Self::SetNull => "SET NULL",
            Self::SetDefault => "SET DEFAULT",
        }
    }
}

