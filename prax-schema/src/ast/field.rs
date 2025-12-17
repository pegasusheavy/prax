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
                            attrs.native_type =
                                Some(super::NativeType::new(name.clone(), args.clone()));
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
                    if let Some(super::AttributeValue::FieldRefList(fields)) =
                        attr.get_arg("fields")
                    {
                        rel.fields = fields.clone();
                    }
                    if let Some(super::AttributeValue::FieldRefList(refs)) =
                        attr.get_arg("references")
                    {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{AttributeArg, AttributeValue, ReferentialAction, ScalarType};

    fn make_span() -> Span {
        Span::new(0, 10)
    }

    fn make_field(name: &str, field_type: FieldType, modifier: TypeModifier) -> Field {
        Field::new(
            Ident::new(name, make_span()),
            field_type,
            modifier,
            vec![],
            make_span(),
        )
    }

    fn make_attribute(name: &str) -> Attribute {
        Attribute::simple(Ident::new(name, make_span()), make_span())
    }

    fn make_attribute_with_arg(name: &str, value: AttributeValue) -> Attribute {
        Attribute::new(
            Ident::new(name, make_span()),
            vec![AttributeArg::positional(value, make_span())],
            make_span(),
        )
    }

    // ==================== Field Construction Tests ====================

    #[test]
    fn test_field_new() {
        let field = Field::new(
            Ident::new("id", make_span()),
            FieldType::Scalar(ScalarType::Int),
            TypeModifier::Required,
            vec![],
            make_span(),
        );

        assert_eq!(field.name(), "id");
        assert!(field.field_type.is_scalar());
        assert_eq!(field.modifier, TypeModifier::Required);
        assert!(field.attributes.is_empty());
        assert!(field.documentation.is_none());
    }

    #[test]
    fn test_field_with_attributes() {
        let field = Field::new(
            Ident::new("email", make_span()),
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
            vec![make_attribute("unique")],
            make_span(),
        );

        assert_eq!(field.attributes.len(), 1);
    }

    #[test]
    fn test_field_with_documentation() {
        let field = make_field(
            "name",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Optional,
        )
        .with_documentation(Documentation::new("User's display name", make_span()));

        assert!(field.documentation.is_some());
        assert_eq!(field.documentation.unwrap().text, "User's display name");
    }

    // ==================== Field Name Tests ====================

    #[test]
    fn test_field_name() {
        let field = make_field(
            "created_at",
            FieldType::Scalar(ScalarType::DateTime),
            TypeModifier::Required,
        );
        assert_eq!(field.name(), "created_at");
    }

    // ==================== Field Modifier Tests ====================

    #[test]
    fn test_field_is_optional_required() {
        let field = make_field(
            "id",
            FieldType::Scalar(ScalarType::Int),
            TypeModifier::Required,
        );
        assert!(!field.is_optional());
    }

    #[test]
    fn test_field_is_optional_true() {
        let field = make_field(
            "bio",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Optional,
        );
        assert!(field.is_optional());
    }

    #[test]
    fn test_field_is_list_false() {
        let field = make_field(
            "name",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
        );
        assert!(!field.is_list());
    }

    #[test]
    fn test_field_is_list_true() {
        let field = make_field(
            "tags",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::List,
        );
        assert!(field.is_list());
    }

    #[test]
    fn test_field_optional_list() {
        let field = make_field(
            "metadata",
            FieldType::Scalar(ScalarType::Json),
            TypeModifier::OptionalList,
        );
        assert!(field.is_optional());
        assert!(field.is_list());
    }

    // ==================== Field Attribute Tests ====================

    #[test]
    fn test_field_has_attribute_true() {
        let mut field = make_field(
            "id",
            FieldType::Scalar(ScalarType::Int),
            TypeModifier::Required,
        );
        field.attributes.push(make_attribute("id"));
        field.attributes.push(make_attribute("auto"));

        assert!(field.has_attribute("id"));
        assert!(field.has_attribute("auto"));
    }

    #[test]
    fn test_field_has_attribute_false() {
        let field = make_field(
            "name",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
        );
        assert!(!field.has_attribute("unique"));
    }

    #[test]
    fn test_field_get_attribute() {
        let mut field = make_field(
            "email",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
        );
        field.attributes.push(make_attribute("unique"));

        let attr = field.get_attribute("unique");
        assert!(attr.is_some());
        assert!(attr.unwrap().is("unique"));

        assert!(field.get_attribute("id").is_none());
    }

    #[test]
    fn test_field_is_id_true() {
        let mut field = make_field(
            "id",
            FieldType::Scalar(ScalarType::Int),
            TypeModifier::Required,
        );
        field.attributes.push(make_attribute("id"));
        assert!(field.is_id());
    }

    #[test]
    fn test_field_is_id_false() {
        let field = make_field(
            "email",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
        );
        assert!(!field.is_id());
    }

    #[test]
    fn test_field_is_unique_true() {
        let mut field = make_field(
            "email",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
        );
        field.attributes.push(make_attribute("unique"));
        assert!(field.is_unique());
    }

    #[test]
    fn test_field_is_unique_false() {
        let field = make_field(
            "name",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
        );
        assert!(!field.is_unique());
    }

    // ==================== Field Relation Tests ====================

    #[test]
    fn test_field_is_relation_by_type() {
        let field = make_field(
            "author",
            FieldType::Model("User".into()),
            TypeModifier::Required,
        );
        assert!(field.is_relation());
    }

    #[test]
    fn test_field_is_relation_by_attribute() {
        let mut field = make_field(
            "author_id",
            FieldType::Scalar(ScalarType::Int),
            TypeModifier::Required,
        );
        field.attributes.push(make_attribute("relation"));
        assert!(field.is_relation());
    }

    #[test]
    fn test_field_is_relation_list() {
        let field = make_field("posts", FieldType::Model("Post".into()), TypeModifier::List);
        assert!(field.is_relation());
        assert!(field.is_list());
    }

    // ==================== Extract Attributes Tests ====================

    #[test]
    fn test_extract_attributes_empty() {
        let field = make_field(
            "name",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
        );
        let attrs = field.extract_attributes();

        assert!(!attrs.is_id);
        assert!(!attrs.is_auto);
        assert!(!attrs.is_unique);
        assert!(!attrs.is_indexed);
        assert!(!attrs.is_updated_at);
        assert!(!attrs.is_omit);
        assert!(attrs.default.is_none());
        assert!(attrs.map.is_none());
        assert!(attrs.native_type.is_none());
        assert!(attrs.relation.is_none());
    }

    #[test]
    fn test_extract_attributes_id_and_auto() {
        let mut field = make_field(
            "id",
            FieldType::Scalar(ScalarType::Int),
            TypeModifier::Required,
        );
        field.attributes.push(make_attribute("id"));
        field.attributes.push(make_attribute("auto"));

        let attrs = field.extract_attributes();
        assert!(attrs.is_id);
        assert!(attrs.is_auto);
    }

    #[test]
    fn test_extract_attributes_unique() {
        let mut field = make_field(
            "email",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
        );
        field.attributes.push(make_attribute("unique"));

        let attrs = field.extract_attributes();
        assert!(attrs.is_unique);
    }

    #[test]
    fn test_extract_attributes_index() {
        let mut field = make_field(
            "name",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
        );
        field.attributes.push(make_attribute("index"));

        let attrs = field.extract_attributes();
        assert!(attrs.is_indexed);
    }

    #[test]
    fn test_extract_attributes_updated_at() {
        let mut field = make_field(
            "updated_at",
            FieldType::Scalar(ScalarType::DateTime),
            TypeModifier::Required,
        );
        field.attributes.push(make_attribute("updated_at"));

        let attrs = field.extract_attributes();
        assert!(attrs.is_updated_at);
    }

    #[test]
    fn test_extract_attributes_omit() {
        let mut field = make_field(
            "password_hash",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
        );
        field.attributes.push(make_attribute("omit"));

        let attrs = field.extract_attributes();
        assert!(attrs.is_omit);
    }

    #[test]
    fn test_extract_attributes_default_int() {
        let mut field = make_field(
            "count",
            FieldType::Scalar(ScalarType::Int),
            TypeModifier::Required,
        );
        field
            .attributes
            .push(make_attribute_with_arg("default", AttributeValue::Int(0)));

        let attrs = field.extract_attributes();
        assert!(attrs.default.is_some());
        assert_eq!(attrs.default.as_ref().unwrap().as_int(), Some(0));
    }

    #[test]
    fn test_extract_attributes_default_function() {
        let mut field = make_field(
            "created_at",
            FieldType::Scalar(ScalarType::DateTime),
            TypeModifier::Required,
        );
        field.attributes.push(make_attribute_with_arg(
            "default",
            AttributeValue::Function("now".into(), vec![]),
        ));

        let attrs = field.extract_attributes();
        assert!(attrs.default.is_some());
        if let AttributeValue::Function(name, _) = attrs.default.as_ref().unwrap() {
            assert_eq!(name.as_str(), "now");
        } else {
            panic!("Expected Function");
        }
    }

    #[test]
    fn test_extract_attributes_map() {
        let mut field = make_field(
            "email",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
        );
        field.attributes.push(make_attribute_with_arg(
            "map",
            AttributeValue::String("email_address".into()),
        ));

        let attrs = field.extract_attributes();
        assert_eq!(attrs.map, Some("email_address".to_string()));
    }

    #[test]
    fn test_extract_attributes_native_type_ident() {
        let mut field = make_field(
            "data",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
        );
        field.attributes.push(make_attribute_with_arg(
            "db",
            AttributeValue::Ident("Text".into()),
        ));

        let attrs = field.extract_attributes();
        assert!(attrs.native_type.is_some());
        let nt = attrs.native_type.unwrap();
        assert_eq!(nt.name.as_str(), "Text");
        assert!(nt.args.is_empty());
    }

    #[test]
    fn test_extract_attributes_native_type_function() {
        let mut field = make_field(
            "name",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
        );
        field.attributes.push(make_attribute_with_arg(
            "db",
            AttributeValue::Function("VarChar".into(), vec![AttributeValue::Int(255)]),
        ));

        let attrs = field.extract_attributes();
        assert!(attrs.native_type.is_some());
        let nt = attrs.native_type.unwrap();
        assert_eq!(nt.name.as_str(), "VarChar");
        assert_eq!(nt.args.len(), 1);
    }

    #[test]
    fn test_extract_attributes_relation() {
        let mut field = make_field(
            "author",
            FieldType::Model("User".into()),
            TypeModifier::Required,
        );
        field.attributes.push(Attribute::new(
            Ident::new("relation", make_span()),
            vec![
                AttributeArg::named(
                    Ident::new("fields", make_span()),
                    AttributeValue::FieldRefList(vec!["author_id".into()]),
                    make_span(),
                ),
                AttributeArg::named(
                    Ident::new("references", make_span()),
                    AttributeValue::FieldRefList(vec!["id".into()]),
                    make_span(),
                ),
                AttributeArg::named(
                    Ident::new("onDelete", make_span()),
                    AttributeValue::Ident("Cascade".into()),
                    make_span(),
                ),
                AttributeArg::named(
                    Ident::new("onUpdate", make_span()),
                    AttributeValue::Ident("Restrict".into()),
                    make_span(),
                ),
            ],
            make_span(),
        ));

        let attrs = field.extract_attributes();
        assert!(attrs.relation.is_some());

        let rel = attrs.relation.unwrap();
        assert_eq!(rel.fields, vec!["author_id".to_string()]);
        assert_eq!(rel.references, vec!["id".to_string()]);
        assert_eq!(rel.on_delete, Some(ReferentialAction::Cascade));
        assert_eq!(rel.on_update, Some(ReferentialAction::Restrict));
    }

    #[test]
    fn test_extract_attributes_relation_with_name() {
        let mut field = make_field(
            "author",
            FieldType::Model("User".into()),
            TypeModifier::Required,
        );
        field.attributes.push(Attribute::new(
            Ident::new("relation", make_span()),
            vec![AttributeArg::positional(
                AttributeValue::String("PostAuthor".into()),
                make_span(),
            )],
            make_span(),
        ));

        let attrs = field.extract_attributes();
        assert!(attrs.relation.is_some());
        assert_eq!(attrs.relation.unwrap().name, Some("PostAuthor".to_string()));
    }

    // ==================== Field Display Tests ====================

    #[test]
    fn test_field_display_required() {
        let field = make_field(
            "id",
            FieldType::Scalar(ScalarType::Int),
            TypeModifier::Required,
        );
        assert_eq!(format!("{}", field), "id Int");
    }

    #[test]
    fn test_field_display_optional() {
        let field = make_field(
            "bio",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Optional,
        );
        assert_eq!(format!("{}", field), "bio String?");
    }

    #[test]
    fn test_field_display_list() {
        let field = make_field(
            "tags",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::List,
        );
        assert_eq!(format!("{}", field), "tags String[]");
    }

    #[test]
    fn test_field_display_optional_list() {
        let field = make_field(
            "data",
            FieldType::Scalar(ScalarType::Json),
            TypeModifier::OptionalList,
        );
        assert_eq!(format!("{}", field), "data Json[]?");
    }

    #[test]
    fn test_field_display_with_simple_attribute() {
        let mut field = make_field(
            "id",
            FieldType::Scalar(ScalarType::Int),
            TypeModifier::Required,
        );
        field.attributes.push(make_attribute("id"));
        assert!(format!("{}", field).contains("@id"));
    }

    #[test]
    fn test_field_display_with_attribute_args() {
        let mut field = make_field(
            "count",
            FieldType::Scalar(ScalarType::Int),
            TypeModifier::Required,
        );
        field
            .attributes
            .push(make_attribute_with_arg("default", AttributeValue::Int(0)));
        assert!(format!("{}", field).contains("@default(...)"));
    }

    #[test]
    fn test_field_display_relation() {
        let field = make_field(
            "author",
            FieldType::Model("User".into()),
            TypeModifier::Required,
        );
        assert_eq!(format!("{}", field), "author User");
    }

    #[test]
    fn test_field_display_enum() {
        let field = make_field(
            "role",
            FieldType::Enum("Role".into()),
            TypeModifier::Required,
        );
        assert_eq!(format!("{}", field), "role Role");
    }

    // ==================== Field Equality Tests ====================

    #[test]
    fn test_field_equality() {
        let field1 = make_field(
            "id",
            FieldType::Scalar(ScalarType::Int),
            TypeModifier::Required,
        );
        let field2 = make_field(
            "id",
            FieldType::Scalar(ScalarType::Int),
            TypeModifier::Required,
        );
        assert_eq!(field1, field2);
    }

    #[test]
    fn test_field_inequality_name() {
        let field1 = make_field(
            "id",
            FieldType::Scalar(ScalarType::Int),
            TypeModifier::Required,
        );
        let field2 = make_field(
            "user_id",
            FieldType::Scalar(ScalarType::Int),
            TypeModifier::Required,
        );
        assert_ne!(field1, field2);
    }

    #[test]
    fn test_field_inequality_type() {
        let field1 = make_field(
            "id",
            FieldType::Scalar(ScalarType::Int),
            TypeModifier::Required,
        );
        let field2 = make_field(
            "id",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
        );
        assert_ne!(field1, field2);
    }

    #[test]
    fn test_field_inequality_modifier() {
        let field1 = make_field(
            "name",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
        );
        let field2 = make_field(
            "name",
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Optional,
        );
        assert_ne!(field1, field2);
    }
}
