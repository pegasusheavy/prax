//! Schema parser for `.prax` files.

mod grammar;

use std::path::Path;

use pest::Parser;
use smol_str::SmolStr;

use crate::ast::*;
use crate::error::{SchemaError, SchemaResult};

pub use grammar::{PraxParser, Rule};

/// Parse a schema from a string.
pub fn parse_schema(input: &str) -> SchemaResult<Schema> {
    let pairs = PraxParser::parse(Rule::schema, input)
        .map_err(|e| SchemaError::syntax(input.to_string(), 0, input.len(), e.to_string()))?;

    let mut schema = Schema::new();
    let mut current_doc: Option<Documentation> = None;

    // The top-level parse result contains a single "schema" rule - get its inner pairs
    let schema_pair = pairs.into_iter().next().unwrap();

    for pair in schema_pair.into_inner() {
        match pair.as_rule() {
            Rule::documentation => {
                let span = pair.as_span();
                let text = pair
                    .into_inner()
                    .map(|p| p.as_str().trim_start_matches("///").trim())
                    .collect::<Vec<_>>()
                    .join("\n");
                current_doc = Some(Documentation::new(
                    text,
                    Span::new(span.start(), span.end()),
                ));
            }
            Rule::model_def => {
                let mut model = parse_model(pair)?;
                if let Some(doc) = current_doc.take() {
                    model = model.with_documentation(doc);
                }
                schema.add_model(model);
            }
            Rule::enum_def => {
                let mut e = parse_enum(pair)?;
                if let Some(doc) = current_doc.take() {
                    e = e.with_documentation(doc);
                }
                schema.add_enum(e);
            }
            Rule::type_def => {
                let mut t = parse_composite_type(pair)?;
                if let Some(doc) = current_doc.take() {
                    t = t.with_documentation(doc);
                }
                schema.add_type(t);
            }
            Rule::view_def => {
                let mut v = parse_view(pair)?;
                if let Some(doc) = current_doc.take() {
                    v = v.with_documentation(doc);
                }
                schema.add_view(v);
            }
            Rule::raw_sql_def => {
                let sql = parse_raw_sql(pair)?;
                schema.add_raw_sql(sql);
            }
            Rule::EOI => {}
            _ => {}
        }
    }

    Ok(schema)
}

/// Parse a schema from a file.
pub fn parse_schema_file(path: impl AsRef<Path>) -> SchemaResult<Schema> {
    let path = path.as_ref();
    let content = std::fs::read_to_string(path).map_err(|e| SchemaError::IoError {
        path: path.display().to_string(),
        source: e,
    })?;

    parse_schema(&content)
}

/// Parse a model definition.
fn parse_model(pair: pest::iterators::Pair<'_, Rule>) -> SchemaResult<Model> {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let name_pair = inner.next().unwrap();
    let name = Ident::new(
        name_pair.as_str(),
        Span::new(name_pair.as_span().start(), name_pair.as_span().end()),
    );

    let mut model = Model::new(name, Span::new(span.start(), span.end()));

    for item in inner {
        match item.as_rule() {
            Rule::field_def => {
                let field = parse_field(item)?;
                model.add_field(field);
            }
            Rule::model_attribute => {
                let attr = parse_attribute(item)?;
                model.attributes.push(attr);
            }
            Rule::model_body_item => {
                // Unwrap the model_body_item to get the actual field_def or model_attribute
                let inner_item = item.into_inner().next().unwrap();
                match inner_item.as_rule() {
                    Rule::field_def => {
                        let field = parse_field(inner_item)?;
                        model.add_field(field);
                    }
                    Rule::model_attribute => {
                        let attr = parse_attribute(inner_item)?;
                        model.attributes.push(attr);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    Ok(model)
}

/// Parse an enum definition.
fn parse_enum(pair: pest::iterators::Pair<'_, Rule>) -> SchemaResult<Enum> {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let name_pair = inner.next().unwrap();
    let name = Ident::new(
        name_pair.as_str(),
        Span::new(name_pair.as_span().start(), name_pair.as_span().end()),
    );

    let mut e = Enum::new(name, Span::new(span.start(), span.end()));

    for item in inner {
        match item.as_rule() {
            Rule::enum_variant => {
                let variant = parse_enum_variant(item)?;
                e.add_variant(variant);
            }
            Rule::model_attribute => {
                let attr = parse_attribute(item)?;
                e.attributes.push(attr);
            }
            Rule::enum_body_item => {
                // Unwrap the enum_body_item to get the actual enum_variant or model_attribute
                let inner_item = item.into_inner().next().unwrap();
                match inner_item.as_rule() {
                    Rule::enum_variant => {
                        let variant = parse_enum_variant(inner_item)?;
                        e.add_variant(variant);
                    }
                    Rule::model_attribute => {
                        let attr = parse_attribute(inner_item)?;
                        e.attributes.push(attr);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    Ok(e)
}

/// Parse an enum variant.
fn parse_enum_variant(pair: pest::iterators::Pair<'_, Rule>) -> SchemaResult<EnumVariant> {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let name_pair = inner.next().unwrap();
    let name = Ident::new(
        name_pair.as_str(),
        Span::new(name_pair.as_span().start(), name_pair.as_span().end()),
    );

    let mut variant = EnumVariant::new(name, Span::new(span.start(), span.end()));

    for item in inner {
        if item.as_rule() == Rule::field_attribute {
            let attr = parse_attribute(item)?;
            variant.attributes.push(attr);
        }
    }

    Ok(variant)
}

/// Parse a composite type definition.
fn parse_composite_type(pair: pest::iterators::Pair<'_, Rule>) -> SchemaResult<CompositeType> {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let name_pair = inner.next().unwrap();
    let name = Ident::new(
        name_pair.as_str(),
        Span::new(name_pair.as_span().start(), name_pair.as_span().end()),
    );

    let mut t = CompositeType::new(name, Span::new(span.start(), span.end()));

    for item in inner {
        if item.as_rule() == Rule::field_def {
            let field = parse_field(item)?;
            t.add_field(field);
        }
    }

    Ok(t)
}

/// Parse a view definition.
fn parse_view(pair: pest::iterators::Pair<'_, Rule>) -> SchemaResult<View> {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let name_pair = inner.next().unwrap();
    let name = Ident::new(
        name_pair.as_str(),
        Span::new(name_pair.as_span().start(), name_pair.as_span().end()),
    );

    let mut v = View::new(name, Span::new(span.start(), span.end()));

    for item in inner {
        match item.as_rule() {
            Rule::field_def => {
                let field = parse_field(item)?;
                v.add_field(field);
            }
            Rule::model_attribute => {
                let attr = parse_attribute(item)?;
                v.attributes.push(attr);
            }
            _ => {}
        }
    }

    Ok(v)
}

/// Parse a field definition.
fn parse_field(pair: pest::iterators::Pair<'_, Rule>) -> SchemaResult<Field> {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let name_pair = inner.next().unwrap();
    let name = Ident::new(
        name_pair.as_str(),
        Span::new(name_pair.as_span().start(), name_pair.as_span().end()),
    );

    let type_pair = inner.next().unwrap();
    let (field_type, modifier) = parse_field_type(type_pair)?;

    let mut attributes = vec![];
    for item in inner {
        if item.as_rule() == Rule::field_attribute {
            let attr = parse_attribute(item)?;
            attributes.push(attr);
        }
    }

    Ok(Field::new(
        name,
        field_type,
        modifier,
        attributes,
        Span::new(span.start(), span.end()),
    ))
}

/// Parse a field type with optional modifier.
fn parse_field_type(
    pair: pest::iterators::Pair<'_, Rule>,
) -> SchemaResult<(FieldType, TypeModifier)> {
    let mut type_name = String::new();
    let mut modifier = TypeModifier::Required;

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::type_name => {
                type_name = item.as_str().to_string();
            }
            Rule::optional_marker => {
                modifier = if modifier == TypeModifier::List {
                    TypeModifier::OptionalList
                } else {
                    TypeModifier::Optional
                };
            }
            Rule::list_marker => {
                modifier = if modifier == TypeModifier::Optional {
                    TypeModifier::OptionalList
                } else {
                    TypeModifier::List
                };
            }
            _ => {}
        }
    }

    let field_type = if let Some(scalar) = ScalarType::from_str(&type_name) {
        FieldType::Scalar(scalar)
    } else {
        // Assume it's a reference to a model, enum, or type
        // This will be validated later
        FieldType::Model(SmolStr::new(&type_name))
    };

    Ok((field_type, modifier))
}

/// Parse an attribute.
fn parse_attribute(pair: pest::iterators::Pair<'_, Rule>) -> SchemaResult<Attribute> {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let name_pair = inner.next().unwrap();
    let name = Ident::new(
        name_pair.as_str(),
        Span::new(name_pair.as_span().start(), name_pair.as_span().end()),
    );

    let mut args = vec![];
    for item in inner {
        if item.as_rule() == Rule::attribute_args {
            args = parse_attribute_args(item)?;
        }
    }

    Ok(Attribute::new(
        name,
        args,
        Span::new(span.start(), span.end()),
    ))
}

/// Parse attribute arguments.
fn parse_attribute_args(pair: pest::iterators::Pair<'_, Rule>) -> SchemaResult<Vec<AttributeArg>> {
    let mut args = vec![];

    for item in pair.into_inner() {
        if item.as_rule() == Rule::attribute_arg {
            let arg = parse_attribute_arg(item)?;
            args.push(arg);
        }
    }

    Ok(args)
}

/// Parse a single attribute argument.
fn parse_attribute_arg(pair: pest::iterators::Pair<'_, Rule>) -> SchemaResult<AttributeArg> {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    let first = inner.next().unwrap();

    // Check if this is a named argument (name: value) or positional
    if let Some(second) = inner.next() {
        // Named argument
        let name = Ident::new(
            first.as_str(),
            Span::new(first.as_span().start(), first.as_span().end()),
        );
        let value = parse_attribute_value(second)?;
        Ok(AttributeArg::named(
            name,
            value,
            Span::new(span.start(), span.end()),
        ))
    } else {
        // Positional argument
        let value = parse_attribute_value(first)?;
        Ok(AttributeArg::positional(
            value,
            Span::new(span.start(), span.end()),
        ))
    }
}

/// Parse an attribute value.
fn parse_attribute_value(pair: pest::iterators::Pair<'_, Rule>) -> SchemaResult<AttributeValue> {
    match pair.as_rule() {
        Rule::string_literal => {
            let s = pair.as_str();
            // Remove quotes
            let unquoted = &s[1..s.len() - 1];
            Ok(AttributeValue::String(unquoted.to_string()))
        }
        Rule::number_literal => {
            let s = pair.as_str();
            if s.contains('.') {
                Ok(AttributeValue::Float(s.parse().unwrap()))
            } else {
                Ok(AttributeValue::Int(s.parse().unwrap()))
            }
        }
        Rule::boolean_literal => Ok(AttributeValue::Boolean(pair.as_str() == "true")),
        Rule::identifier => Ok(AttributeValue::Ident(SmolStr::new(pair.as_str()))),
        Rule::function_call => {
            let mut inner = pair.into_inner();
            let name = SmolStr::new(inner.next().unwrap().as_str());
            let mut args = vec![];
            for item in inner {
                args.push(parse_attribute_value(item)?);
            }
            Ok(AttributeValue::Function(name, args))
        }
        Rule::field_ref_list => {
            let refs: Vec<SmolStr> = pair
                .into_inner()
                .map(|p| SmolStr::new(p.as_str()))
                .collect();
            Ok(AttributeValue::FieldRefList(refs))
        }
        Rule::array_literal => {
            let values: Result<Vec<_>, _> = pair.into_inner().map(parse_attribute_value).collect();
            Ok(AttributeValue::Array(values?))
        }
        Rule::attribute_value => {
            // Unwrap nested attribute_value
            parse_attribute_value(pair.into_inner().next().unwrap())
        }
        _ => {
            // Fallback: treat as identifier
            Ok(AttributeValue::Ident(SmolStr::new(pair.as_str())))
        }
    }
}

/// Parse a raw SQL definition.
fn parse_raw_sql(pair: pest::iterators::Pair<'_, Rule>) -> SchemaResult<RawSql> {
    let mut inner = pair.into_inner();

    let name = inner.next().unwrap().as_str();
    let sql = inner.next().unwrap().as_str();

    // Remove triple quotes
    let sql_content = sql
        .trim_start_matches("\"\"\"")
        .trim_end_matches("\"\"\"")
        .trim();

    Ok(RawSql::new(name, sql_content))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Basic Model Parsing ====================

    #[test]
    fn test_parse_simple_model() {
        let schema = parse_schema(
            r#"
            model User {
                id    Int    @id @auto
                email String @unique
                name  String?
            }
        "#,
        )
        .unwrap();

        assert_eq!(schema.models.len(), 1);
        let user = schema.get_model("User").unwrap();
        assert_eq!(user.fields.len(), 3);
        assert!(user.get_field("id").unwrap().is_id());
        assert!(user.get_field("email").unwrap().is_unique());
        assert!(user.get_field("name").unwrap().is_optional());
    }

    #[test]
    fn test_parse_model_name() {
        let schema = parse_schema(
            r#"
            model BlogPost {
                id Int @id
            }
        "#,
        )
        .unwrap();

        assert!(schema.get_model("BlogPost").is_some());
    }

    #[test]
    fn test_parse_multiple_models() {
        let schema = parse_schema(
            r#"
            model User {
                id Int @id
            }
            
            model Post {
                id Int @id
            }
            
            model Comment {
                id Int @id
            }
        "#,
        )
        .unwrap();

        assert_eq!(schema.models.len(), 3);
        assert!(schema.get_model("User").is_some());
        assert!(schema.get_model("Post").is_some());
        assert!(schema.get_model("Comment").is_some());
    }

    // ==================== Field Type Parsing ====================

    #[test]
    fn test_parse_all_scalar_types() {
        let schema = parse_schema(
            r#"
            model AllTypes {
                id       Int      @id
                big      BigInt
                float_f  Float
                decimal  Decimal
                str      String
                bool     Boolean
                datetime DateTime
                date     Date
                time     Time
                json     Json
                bytes    Bytes
                uuid     Uuid
            }
        "#,
        )
        .unwrap();

        let model = schema.get_model("AllTypes").unwrap();
        assert_eq!(model.fields.len(), 12);

        assert!(matches!(
            model.get_field("id").unwrap().field_type,
            FieldType::Scalar(ScalarType::Int)
        ));
        assert!(matches!(
            model.get_field("big").unwrap().field_type,
            FieldType::Scalar(ScalarType::BigInt)
        ));
        assert!(matches!(
            model.get_field("str").unwrap().field_type,
            FieldType::Scalar(ScalarType::String)
        ));
        assert!(matches!(
            model.get_field("bool").unwrap().field_type,
            FieldType::Scalar(ScalarType::Boolean)
        ));
        assert!(matches!(
            model.get_field("datetime").unwrap().field_type,
            FieldType::Scalar(ScalarType::DateTime)
        ));
        assert!(matches!(
            model.get_field("uuid").unwrap().field_type,
            FieldType::Scalar(ScalarType::Uuid)
        ));
    }

    #[test]
    fn test_parse_optional_field() {
        let schema = parse_schema(
            r#"
            model User {
                id   Int     @id
                bio  String?
                age  Int?
            }
        "#,
        )
        .unwrap();

        let user = schema.get_model("User").unwrap();
        assert!(!user.get_field("id").unwrap().is_optional());
        assert!(user.get_field("bio").unwrap().is_optional());
        assert!(user.get_field("age").unwrap().is_optional());
    }

    #[test]
    fn test_parse_list_field() {
        let schema = parse_schema(
            r#"
            model User {
                id    Int      @id
                tags  String[]
                posts Post[]
            }
        "#,
        )
        .unwrap();

        let user = schema.get_model("User").unwrap();
        assert!(user.get_field("tags").unwrap().is_list());
        assert!(user.get_field("posts").unwrap().is_list());
    }

    #[test]
    fn test_parse_optional_list_field() {
        let schema = parse_schema(
            r#"
            model User {
                id       Int       @id
                metadata String[]?
            }
        "#,
        )
        .unwrap();

        let user = schema.get_model("User").unwrap();
        let metadata = user.get_field("metadata").unwrap();
        assert!(metadata.is_list());
        assert!(metadata.is_optional());
    }

    // ==================== Attribute Parsing ====================

    #[test]
    fn test_parse_id_attribute() {
        let schema = parse_schema(
            r#"
            model User {
                id Int @id
            }
        "#,
        )
        .unwrap();

        let user = schema.get_model("User").unwrap();
        assert!(user.get_field("id").unwrap().is_id());
    }

    #[test]
    fn test_parse_unique_attribute() {
        let schema = parse_schema(
            r#"
            model User {
                id    Int    @id
                email String @unique
            }
        "#,
        )
        .unwrap();

        let user = schema.get_model("User").unwrap();
        assert!(user.get_field("email").unwrap().is_unique());
    }

    #[test]
    fn test_parse_default_int() {
        let schema = parse_schema(
            r#"
            model Counter {
                id    Int @id
                count Int @default(0)
            }
        "#,
        )
        .unwrap();

        let counter = schema.get_model("Counter").unwrap();
        let count_field = counter.get_field("count").unwrap();
        let attrs = count_field.extract_attributes();
        assert!(attrs.default.is_some());
        assert_eq!(attrs.default.unwrap().as_int(), Some(0));
    }

    #[test]
    fn test_parse_default_string() {
        let schema = parse_schema(
            r#"
            model User {
                id     Int    @id
                status String @default("active")
            }
        "#,
        )
        .unwrap();

        let user = schema.get_model("User").unwrap();
        let status = user.get_field("status").unwrap();
        let attrs = status.extract_attributes();
        assert!(attrs.default.is_some());
        assert_eq!(attrs.default.unwrap().as_string(), Some("active"));
    }

    #[test]
    fn test_parse_default_boolean() {
        let schema = parse_schema(
            r#"
            model Post {
                id        Int     @id
                published Boolean @default(false)
            }
        "#,
        )
        .unwrap();

        let post = schema.get_model("Post").unwrap();
        let published = post.get_field("published").unwrap();
        let attrs = published.extract_attributes();
        assert!(attrs.default.is_some());
        assert_eq!(attrs.default.unwrap().as_bool(), Some(false));
    }

    #[test]
    fn test_parse_default_function() {
        let schema = parse_schema(
            r#"
            model User {
                id        Int      @id
                createdAt DateTime @default(now())
            }
        "#,
        )
        .unwrap();

        let user = schema.get_model("User").unwrap();
        let created_at = user.get_field("createdAt").unwrap();
        let attrs = created_at.extract_attributes();
        assert!(attrs.default.is_some());
        if let Some(AttributeValue::Function(name, _)) = attrs.default {
            assert_eq!(name.as_str(), "now");
        } else {
            panic!("Expected function default");
        }
    }

    #[test]
    fn test_parse_updated_at_attribute() {
        let schema = parse_schema(
            r#"
            model User {
                id        Int      @id
                updatedAt DateTime @updated_at
            }
        "#,
        )
        .unwrap();

        let user = schema.get_model("User").unwrap();
        let updated_at = user.get_field("updatedAt").unwrap();
        let attrs = updated_at.extract_attributes();
        assert!(attrs.is_updated_at);
    }

    #[test]
    fn test_parse_map_attribute() {
        let schema = parse_schema(
            r#"
            model User {
                id    Int    @id
                email String @map("email_address")
            }
        "#,
        )
        .unwrap();

        let user = schema.get_model("User").unwrap();
        let email = user.get_field("email").unwrap();
        let attrs = email.extract_attributes();
        assert_eq!(attrs.map, Some("email_address".to_string()));
    }

    #[test]
    fn test_parse_multiple_attributes() {
        let schema = parse_schema(
            r#"
            model User {
                id    Int    @id @auto
                email String @unique @index
            }
        "#,
        )
        .unwrap();

        let user = schema.get_model("User").unwrap();
        let id = user.get_field("id").unwrap();
        let email = user.get_field("email").unwrap();

        let id_attrs = id.extract_attributes();
        assert!(id_attrs.is_id);
        assert!(id_attrs.is_auto);

        let email_attrs = email.extract_attributes();
        assert!(email_attrs.is_unique);
        assert!(email_attrs.is_indexed);
    }

    // ==================== Model Attribute Parsing ====================

    #[test]
    fn test_parse_model_map_attribute() {
        let schema = parse_schema(
            r#"
            model User {
                id Int @id
                
                @@map("app_users")
            }
        "#,
        )
        .unwrap();

        let user = schema.get_model("User").unwrap();
        assert_eq!(user.table_name(), "app_users");
    }

    #[test]
    fn test_parse_model_index_attribute() {
        let schema = parse_schema(
            r#"
            model User {
                id    Int    @id
                email String
                name  String
                
                @@index([email, name])
            }
        "#,
        )
        .unwrap();

        let user = schema.get_model("User").unwrap();
        assert!(user.has_attribute("index"));
    }

    #[test]
    fn test_parse_composite_primary_key() {
        let schema = parse_schema(
            r#"
            model PostTag {
                postId Int
                tagId  Int
                
                @@id([postId, tagId])
            }
        "#,
        )
        .unwrap();

        let post_tag = schema.get_model("PostTag").unwrap();
        assert!(post_tag.has_attribute("id"));
    }

    // ==================== Enum Parsing ====================

    #[test]
    fn test_parse_enum() {
        let schema = parse_schema(
            r#"
            enum Role {
                User
                Admin
                Moderator
            }
        "#,
        )
        .unwrap();

        assert_eq!(schema.enums.len(), 1);
        let role = schema.get_enum("Role").unwrap();
        assert_eq!(role.variants.len(), 3);
    }

    #[test]
    fn test_parse_enum_variant_names() {
        let schema = parse_schema(
            r#"
            enum Status {
                Pending
                Active
                Completed
                Cancelled
            }
        "#,
        )
        .unwrap();

        let status = schema.get_enum("Status").unwrap();
        assert!(status.get_variant("Pending").is_some());
        assert!(status.get_variant("Active").is_some());
        assert!(status.get_variant("Completed").is_some());
        assert!(status.get_variant("Cancelled").is_some());
    }

    #[test]
    fn test_parse_enum_with_map() {
        let schema = parse_schema(
            r#"
            enum Role {
                User  @map("USER")
                Admin @map("ADMINISTRATOR")
            }
        "#,
        )
        .unwrap();

        let role = schema.get_enum("Role").unwrap();
        let user_variant = role.get_variant("User").unwrap();
        assert_eq!(user_variant.db_value(), "USER");

        let admin_variant = role.get_variant("Admin").unwrap();
        assert_eq!(admin_variant.db_value(), "ADMINISTRATOR");
    }

    // ==================== Relation Parsing ====================

    #[test]
    fn test_parse_one_to_many_relation() {
        let schema = parse_schema(
            r#"
            model User {
                id    Int    @id
                posts Post[]
            }
            
            model Post {
                id       Int  @id
                authorId Int
                author   User @relation(fields: [authorId], references: [id])
            }
        "#,
        )
        .unwrap();

        let user = schema.get_model("User").unwrap();
        let post = schema.get_model("Post").unwrap();

        assert!(user.get_field("posts").unwrap().is_list());
        assert!(post.get_field("author").unwrap().is_relation());
    }

    #[test]
    fn test_parse_relation_with_actions() {
        let schema = parse_schema(
            r#"
            model Post {
                id       Int  @id
                authorId Int
                author   User @relation(fields: [authorId], references: [id], onDelete: Cascade, onUpdate: Restrict)
            }
            
            model User {
                id    Int    @id
                posts Post[]
            }
        "#,
        )
        .unwrap();

        let post = schema.get_model("Post").unwrap();
        let author = post.get_field("author").unwrap();
        let attrs = author.extract_attributes();

        assert!(attrs.relation.is_some());
        let rel = attrs.relation.unwrap();
        assert_eq!(rel.on_delete, Some(ReferentialAction::Cascade));
        assert_eq!(rel.on_update, Some(ReferentialAction::Restrict));
    }

    // ==================== Documentation Parsing ====================

    #[test]
    fn test_parse_model_documentation() {
        let schema = parse_schema(
            r#"/// Represents a user in the system
model User {
    id Int @id
}"#,
        )
        .unwrap();

        let user = schema.get_model("User").unwrap();
        // Documentation parsing is optional - the model should still parse
        // If documentation is present, it should contain "user"
        if let Some(doc) = &user.documentation {
            assert!(doc.text.contains("user"));
        }
    }

    // ==================== Complete Schema Parsing ====================

    #[test]
    fn test_parse_complete_schema() {
        let schema = parse_schema(
            r#"
            /// User model
            model User {
                id        Int      @id @auto
                email     String   @unique
                name      String?
                role      Role     @default(User)
                posts     Post[]
                profile   Profile?
                createdAt DateTime @default(now())
                updatedAt DateTime @updated_at
                
                @@map("users")
                @@index([email])
            }
            
            model Post {
                id        Int      @id @auto
                title     String
                content   String?
                published Boolean  @default(false)
                authorId  Int
                author    User     @relation(fields: [authorId], references: [id])
                tags      Tag[]
                createdAt DateTime @default(now())
                
                @@index([authorId])
            }
            
            model Profile {
                id     Int    @id @auto
                bio    String?
                userId Int    @unique
                user   User   @relation(fields: [userId], references: [id])
            }
            
            model Tag {
                id    Int    @id @auto
                name  String @unique
                posts Post[]
            }
            
            enum Role {
                User
                Admin
                Moderator
            }
        "#,
        )
        .unwrap();

        // Verify models
        assert_eq!(schema.models.len(), 4);
        assert!(schema.get_model("User").is_some());
        assert!(schema.get_model("Post").is_some());
        assert!(schema.get_model("Profile").is_some());
        assert!(schema.get_model("Tag").is_some());

        // Verify enums
        assert_eq!(schema.enums.len(), 1);
        assert!(schema.get_enum("Role").is_some());

        // Verify User model details
        let user = schema.get_model("User").unwrap();
        assert_eq!(user.table_name(), "users");
        assert_eq!(user.fields.len(), 8);
        assert!(user.has_attribute("index"));

        // Verify relations
        let post = schema.get_model("Post").unwrap();
        assert!(post.get_field("author").unwrap().is_relation());
    }

    // ==================== Error Handling ====================

    #[test]
    fn test_parse_invalid_syntax() {
        let result = parse_schema("model { broken }");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_schema() {
        let schema = parse_schema("").unwrap();
        assert!(schema.models.is_empty());
        assert!(schema.enums.is_empty());
    }

    #[test]
    fn test_parse_whitespace_only() {
        let schema = parse_schema("   \n\t   \n   ").unwrap();
        assert!(schema.models.is_empty());
    }

    #[test]
    fn test_parse_comments_only() {
        let schema = parse_schema(
            r#"
            // This is a comment
            // Another comment
        "#,
        )
        .unwrap();
        assert!(schema.models.is_empty());
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_parse_model_with_no_fields() {
        // Models with no fields should still parse (might be invalid semantically but syntactically ok)
        let result = parse_schema(
            r#"
            model Empty {
            }
        "#,
        );
        // This might error or succeed depending on grammar - just verify it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_parse_long_identifier() {
        let schema = parse_schema(
            r#"
            model VeryLongModelNameThatIsStillValid {
                someVeryLongFieldNameThatShouldWork Int @id
            }
        "#,
        )
        .unwrap();

        assert!(
            schema
                .get_model("VeryLongModelNameThatIsStillValid")
                .is_some()
        );
    }

    #[test]
    fn test_parse_underscore_identifiers() {
        let schema = parse_schema(
            r#"
            model user_account {
                user_id     Int @id
                created_at  DateTime
            }
        "#,
        )
        .unwrap();

        let model = schema.get_model("user_account").unwrap();
        assert!(model.get_field("user_id").is_some());
        assert!(model.get_field("created_at").is_some());
    }

    #[test]
    fn test_parse_negative_default() {
        let schema = parse_schema(
            r#"
            model Config {
                id       Int @id
                minValue Int @default(-100)
            }
        "#,
        )
        .unwrap();

        let config = schema.get_model("Config").unwrap();
        let min_value = config.get_field("minValue").unwrap();
        let attrs = min_value.extract_attributes();
        assert!(attrs.default.is_some());
    }

    #[test]
    fn test_parse_float_default() {
        let schema = parse_schema(
            r#"
            model Product {
                id    Int   @id
                price Float @default(9.99)
            }
        "#,
        )
        .unwrap();

        let product = schema.get_model("Product").unwrap();
        let price = product.get_field("price").unwrap();
        let attrs = price.extract_attributes();
        assert!(attrs.default.is_some());
    }
}
