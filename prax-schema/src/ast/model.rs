//! Model definitions for the Prax schema AST.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use super::{Attribute, Documentation, Field, Ident, Span};

/// A model definition (maps to a database table).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Model {
    /// Model name.
    pub name: Ident,
    /// Model fields.
    pub fields: IndexMap<SmolStr, Field>,
    /// Model-level attributes (prefixed with `@@`).
    pub attributes: Vec<Attribute>,
    /// Documentation comment.
    pub documentation: Option<Documentation>,
    /// Source location.
    pub span: Span,
}

impl Model {
    /// Create a new model.
    pub fn new(name: Ident, span: Span) -> Self {
        Self {
            name,
            fields: IndexMap::new(),
            attributes: vec![],
            documentation: None,
            span,
        }
    }

    /// Get the model name as a string.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Add a field to the model.
    pub fn add_field(&mut self, field: Field) {
        self.fields.insert(field.name.name.clone(), field);
    }

    /// Get a field by name.
    pub fn get_field(&self, name: &str) -> Option<&Field> {
        self.fields.get(name)
    }

    /// Get the primary key field(s).
    pub fn id_fields(&self) -> Vec<&Field> {
        self.fields.values().filter(|f| f.is_id()).collect()
    }

    /// Get all relation fields.
    pub fn relation_fields(&self) -> Vec<&Field> {
        self.fields.values().filter(|f| f.is_relation()).collect()
    }

    /// Get all scalar (non-relation) fields.
    pub fn scalar_fields(&self) -> Vec<&Field> {
        self.fields.values().filter(|f| !f.is_relation()).collect()
    }

    /// Check if this model has a specific model-level attribute.
    pub fn has_attribute(&self, name: &str) -> bool {
        self.attributes.iter().any(|a| a.is(name))
    }

    /// Get a model-level attribute by name.
    pub fn get_attribute(&self, name: &str) -> Option<&Attribute> {
        self.attributes.iter().find(|a| a.is(name))
    }

    /// Get the database table name (from `@@map` or model name).
    pub fn table_name(&self) -> &str {
        self.get_attribute("map")
            .and_then(|a| a.first_arg())
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| self.name())
    }

    /// Set documentation.
    pub fn with_documentation(mut self, doc: Documentation) -> Self {
        self.documentation = Some(doc);
        self
    }
}

/// An enum definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Enum {
    /// Enum name.
    pub name: Ident,
    /// Enum variants.
    pub variants: Vec<EnumVariant>,
    /// Enum-level attributes.
    pub attributes: Vec<Attribute>,
    /// Documentation comment.
    pub documentation: Option<Documentation>,
    /// Source location.
    pub span: Span,
}

impl Enum {
    /// Create a new enum.
    pub fn new(name: Ident, span: Span) -> Self {
        Self {
            name,
            variants: vec![],
            attributes: vec![],
            documentation: None,
            span,
        }
    }

    /// Get the enum name as a string.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Add a variant to the enum.
    pub fn add_variant(&mut self, variant: EnumVariant) {
        self.variants.push(variant);
    }

    /// Get a variant by name.
    pub fn get_variant(&self, name: &str) -> Option<&EnumVariant> {
        self.variants.iter().find(|v| v.name.as_str() == name)
    }

    /// Get the database type name (from `@@map` or enum name).
    pub fn db_name(&self) -> &str {
        self.attributes
            .iter()
            .find(|a| a.is("map"))
            .and_then(|a| a.first_arg())
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| self.name())
    }

    /// Set documentation.
    pub fn with_documentation(mut self, doc: Documentation) -> Self {
        self.documentation = Some(doc);
        self
    }
}

/// An enum variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumVariant {
    /// Variant name.
    pub name: Ident,
    /// Variant-level attributes.
    pub attributes: Vec<Attribute>,
    /// Documentation comment.
    pub documentation: Option<Documentation>,
    /// Source location.
    pub span: Span,
}

impl EnumVariant {
    /// Create a new enum variant.
    pub fn new(name: Ident, span: Span) -> Self {
        Self {
            name,
            attributes: vec![],
            documentation: None,
            span,
        }
    }

    /// Get the variant name as a string.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Get the database value (from `@map` or variant name).
    pub fn db_value(&self) -> &str {
        self.attributes
            .iter()
            .find(|a| a.is("map"))
            .and_then(|a| a.first_arg())
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| self.name())
    }
}

/// A composite type definition (for embedded documents / JSON).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeType {
    /// Type name.
    pub name: Ident,
    /// Type fields.
    pub fields: IndexMap<SmolStr, Field>,
    /// Documentation comment.
    pub documentation: Option<Documentation>,
    /// Source location.
    pub span: Span,
}

impl CompositeType {
    /// Create a new composite type.
    pub fn new(name: Ident, span: Span) -> Self {
        Self {
            name,
            fields: IndexMap::new(),
            documentation: None,
            span,
        }
    }

    /// Get the type name as a string.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Add a field to the type.
    pub fn add_field(&mut self, field: Field) {
        self.fields.insert(field.name.name.clone(), field);
    }

    /// Get a field by name.
    pub fn get_field(&self, name: &str) -> Option<&Field> {
        self.fields.get(name)
    }

    /// Set documentation.
    pub fn with_documentation(mut self, doc: Documentation) -> Self {
        self.documentation = Some(doc);
        self
    }
}

/// A view definition (read-only model mapping to a database view).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct View {
    /// View name.
    pub name: Ident,
    /// View fields.
    pub fields: IndexMap<SmolStr, Field>,
    /// View-level attributes.
    pub attributes: Vec<Attribute>,
    /// Documentation comment.
    pub documentation: Option<Documentation>,
    /// Source location.
    pub span: Span,
}

impl View {
    /// Create a new view.
    pub fn new(name: Ident, span: Span) -> Self {
        Self {
            name,
            fields: IndexMap::new(),
            attributes: vec![],
            documentation: None,
            span,
        }
    }

    /// Get the view name as a string.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Add a field to the view.
    pub fn add_field(&mut self, field: Field) {
        self.fields.insert(field.name.name.clone(), field);
    }

    /// Get the database view name (from `@@map` or view name).
    pub fn view_name(&self) -> &str {
        self.attributes
            .iter()
            .find(|a| a.is("map"))
            .and_then(|a| a.first_arg())
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| self.name())
    }

    /// Set documentation.
    pub fn with_documentation(mut self, doc: Documentation) -> Self {
        self.documentation = Some(doc);
        self
    }
}

