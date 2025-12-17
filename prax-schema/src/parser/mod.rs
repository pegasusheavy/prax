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
    let pairs = PraxParser::parse(Rule::schema, input).map_err(|e| {
        SchemaError::syntax(input.to_string(), 0, input.len(), e.to_string())
    })?;

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
                current_doc = Some(Documentation::new(text, Span::new(span.start(), span.end())));
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
fn parse_field_type(pair: pest::iterators::Pair<'_, Rule>) -> SchemaResult<(FieldType, TypeModifier)> {
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

    Ok(Attribute::new(name, args, Span::new(span.start(), span.end())))
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
        Ok(AttributeArg::named(name, value, Span::new(span.start(), span.end())))
    } else {
        // Positional argument
        let value = parse_attribute_value(first)?;
        Ok(AttributeArg::positional(value, Span::new(span.start(), span.end())))
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
        Rule::boolean_literal => {
            Ok(AttributeValue::Boolean(pair.as_str() == "true"))
        }
        Rule::identifier => {
            Ok(AttributeValue::Ident(SmolStr::new(pair.as_str())))
        }
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
            let values: Result<Vec<_>, _> = pair
                .into_inner()
                .map(parse_attribute_value)
                .collect();
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
}

