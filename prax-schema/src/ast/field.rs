//! Field definitions for the Prax schema AST.

use serde::{Deserialize, Serialize};

use super::{Attribute, Documentation, FieldAttributes, FieldType, Ident, Span, TypeModifier};

/// A field in a model or composite type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    /// Field name.
    pub name: Ident,
    /// Field type.
    pub field_type: FieldType,
    /// Type modifier (optional, list, etc.).
    pub modifier: TypeModifier,
    /// Raw attributes as parsed.
    pub attributes: Vec<Attribute>,
    /// Documentation comment.
    pub documentation: Option<Documentation>,
    /// Source location.
    pub span: Span,
}

impl Field {
    /// Create a new field.
    pub fn new(
        name: Ident,
        field_type: FieldType,
        modifier: TypeModifier,
        attributes: Vec<Attribute>,
        span: Span,
    ) -> Self {
        Self {
            name,
            field_type,
            modifier,
            attributes,
            documentation: None,
            span,
        }
    }

    /// Get the field name as a string.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Check if the field is optional.
    pub fn is_optional(&self) -> bool {
        self.modifier.is_optional()
    }

    /// Check if the field is a list.
    pub fn is_list(&self) -> bool {
        self.modifier.is_list()
    }

    /// Check if this field has a specific attribute.
    pub fn has_attribute(&self, name: &str) -> bool {
        self.attributes.iter().any(|a| a.is(name))
    }

    /// Get an attribute by name.
    pub fn get_attribute(&self, name: &str) -> Option<&Attribute> {
        self.attributes.iter().find(|a| a.is(name))
    }

    /// Check if this is a primary key field.
    pub fn is_id(&self) -> bool {
        self.has_attribute("id")
    }

    /// Check if this field has a unique constraint.
    pub fn is_unique(&self) -> bool {
        self.has_attribute("unique")
    }

    /// Check if this is a relation field.
    pub fn is_relation(&self) -> bool {
        self.field_type.is_relation() || self.has_attribute("relation")
    }

    /// Extract structured field attributes.
    pub fn extract_attributes(&self) -> FieldAttributes {
        let mut attrs = FieldAttributes::default();

        for attr in &self.attributes {
            match attr.name() {
                "id" => attrs.is_id = true,
                "auto" => attrs.is_auto = true,
                "unique" => attrs.is_unique = true,
                "index" => attrs.is_indexed = true,
                "updated_at" => attrs.is_updated_at = true,
                "omit" => attrs.is_omit = true,
                "default" => {
                    attrs.default = attr.first_arg().cloned();
                }
                "map" => {
                    if let Some(val) = attr.first_arg() {
                        attrs.map = val.as_string().map(String::from);
                    }
                }
                "db" => {
                    // Parse native type like @db.VarChar(255)
                    if let Some(val) = attr.first_arg() {
                        if let super::AttributeValue::Function(name, args) = val {
                            attrs.native_type = Some(super::NativeType::new(name.clone(), args.clone()));
                        } else if let Some(name) = val.as_ident() {
                            attrs.native_type = Some(super::NativeType::new(name, vec![]));
                        }
                    }
                }
                "relation" => {
                    // Parse relation attributes
                    let mut rel = super::RelationAttribute {
                        name: None,
                        fields: vec![],
                        references: vec![],
                        on_delete: None,
                        on_update: None,
                    };

                    // First positional arg is the relation name
                    if let Some(val) = attr.first_arg() {
                        rel.name = val.as_string().map(String::from);
                    }

                    // Named arguments
                    if let Some(super::AttributeValue::FieldRefList(fields)) = attr.get_arg("fields") {
                        rel.fields = fields.clone();
                    }
                    if let Some(super::AttributeValue::FieldRefList(refs)) = attr.get_arg("references") {
                        rel.references = refs.clone();
                    }
                    if let Some(val) = attr.get_arg("onDelete") {
                        if let Some(action) = val.as_ident() {
                            rel.on_delete = super::ReferentialAction::from_str(action);
                        }
                    }
                    if let Some(val) = attr.get_arg("onUpdate") {
                        if let Some(action) = val.as_ident() {
                            rel.on_update = super::ReferentialAction::from_str(action);
                        }
                    }

                    attrs.relation = Some(rel);
                }
                _ => {}
            }
        }

        attrs
    }

    /// Set documentation.
    pub fn with_documentation(mut self, doc: Documentation) -> Self {
        self.documentation = Some(doc);
        self
    }
}

impl std::fmt::Display for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;

        // Type with modifier
        match self.modifier {
            TypeModifier::Required => write!(f, " {}", self.field_type)?,
            TypeModifier::Optional => write!(f, " {}?", self.field_type)?,
            TypeModifier::List => write!(f, " {}[]", self.field_type)?,
            TypeModifier::OptionalList => write!(f, " {}[]?", self.field_type)?,
        }

        // Attributes
        for attr in &self.attributes {
            write!(f, " @{}", attr.name)?;
            if !attr.args.is_empty() {
                write!(f, "(...)")?;
            }
        }

        Ok(())
    }
}

