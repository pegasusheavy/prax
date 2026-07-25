//! SeaORM entity parser and converter.

use crate::converter::{FieldBuilder, ModelBuilder, SchemaBuilder, table_name_to_model_name};
use crate::error::ImportResult;
use crate::seaorm::types::*;
use convert_case::{Case, Casing};
use prax_schema::ast::*;
use smol_str::SmolStr;
use std::fs;
use std::path::Path;
use syn::punctuated::Punctuated;
use syn::{Attribute, Fields, Item, Meta, Token, Type};

/// Parse a SeaORM entity file from a string.
pub fn parse_seaorm_entity(input: &str) -> ImportResult<SeaOrmEntity> {
    let syntax = syn::parse_file(input).map_err(|e| {
        crate::error::ImportError::SeaOrmParseError(format!("Failed to parse Rust file: {}", e))
    })?;

    let mut entity = None;
    let mut relations = vec![];

    for item in syntax.items {
        match item {
            Item::Struct(item_struct) => {
                // Check if this is an entity model (has DeriveEntityModel)
                if has_derive(&item_struct.attrs, "DeriveEntityModel") {
                    entity = Some(parse_entity_struct(item_struct)?);
                }
            }
            Item::Enum(item_enum) if has_derive(&item_enum.attrs, "DeriveRelation") => {
                // This is a Relation enum
                relations = parse_relation_enum(item_enum)?;
            }
            _ => {}
        }
    }

    let mut entity = entity.ok_or_else(|| {
        crate::error::ImportError::SeaOrmParseError(
            "No entity struct found with #[derive(DeriveEntityModel)]".to_string(),
        )
    })?;

    entity.relations = relations;

    Ok(entity)
}

/// Parse a SeaORM entity from a file.
pub fn parse_seaorm_entity_file<P: AsRef<Path>>(path: P) -> ImportResult<SeaOrmEntity> {
    let content = fs::read_to_string(path)?;
    parse_seaorm_entity(&content)
}

/// Convert SeaORM entity to Prax schema.
pub fn import_seaorm_entity(input: &str) -> ImportResult<Schema> {
    let entity = parse_seaorm_entity(input)?;
    convert_seaorm_to_prax(vec![entity])
}

/// Convert a SeaORM entity file to Prax schema.
pub fn import_seaorm_entity_file<P: AsRef<Path>>(path: P) -> ImportResult<Schema> {
    let entity = parse_seaorm_entity_file(path)?;
    convert_seaorm_to_prax(vec![entity])
}

/// Check if attributes contain a specific derive.
fn has_derive(attrs: &[Attribute], derive_name: &str) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("derive") {
            // Parse as Meta::List to access nested items
            if let Meta::List(list) = &attr.meta {
                let tokens = list.tokens.to_string();
                return tokens.contains(derive_name);
            }
        }
        false
    })
}

/// Parse entity struct into SeaOrmEntity.
fn parse_entity_struct(item_struct: syn::ItemStruct) -> ImportResult<SeaOrmEntity> {
    let name = item_struct.ident.to_string();

    // Extract table name from sea_orm attribute
    let table_name = extract_table_name(&item_struct.attrs).unwrap_or_else(|| {
        // Convert struct name to snake_case and pluralize
        let snake = name.to_lowercase();
        if snake.ends_with('y') {
            format!("{}ies", &snake[..snake.len() - 1])
        } else {
            format!("{}s", snake)
        }
    });

    let mut fields = vec![];

    if let Fields::Named(named_fields) = item_struct.fields {
        for field in named_fields.named {
            let field_name = field.ident.unwrap().to_string();
            let (field_type, is_optional) = parse_field_type(&field.ty)?;
            let attributes = parse_field_attributes(&field.attrs)?;

            fields.push(SeaOrmField {
                name: field_name,
                field_type,
                is_optional,
                attributes,
                documentation: None,
            });
        }
    }

    Ok(SeaOrmEntity {
        name,
        table_name,
        fields,
        relations: vec![],
        documentation: None,
    })
}

/// Extract table name from sea_orm attribute.
fn extract_table_name(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("sea_orm")
            && let Ok(meta) = attr.parse_args::<syn::Meta>()
            && let syn::Meta::NameValue(nv) = meta
            && nv.path.is_ident("table_name")
            && let syn::Expr::Lit(lit) = &nv.value
            && let syn::Lit::Str(s) = &lit.lit
        {
            return Some(s.value());
        }
    }
    None
}

/// Parse field type and check if optional.
fn parse_field_type(ty: &Type) -> ImportResult<(SeaOrmFieldType, bool)> {
    // Check if it's Option<T>
    if let Type::Path(type_path) = ty {
        let segments = &type_path.path.segments;

        if let Some(last_segment) = segments.last() {
            let type_name = last_segment.ident.to_string();

            // Handle Option<T>
            if type_name == "Option"
                && let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments
                && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
            {
                let (inner_type, _) = parse_field_type(inner_ty)?;
                return Ok((inner_type, true));
            }

            // Map Rust types to SeaORM types
            let field_type = match type_name.as_str() {
                "i32" => SeaOrmFieldType::I32,
                "i64" => SeaOrmFieldType::I64,
                "f32" => SeaOrmFieldType::F32,
                "f64" => SeaOrmFieldType::F64,
                "String" => SeaOrmFieldType::String,
                "bool" => SeaOrmFieldType::Bool,
                "DateTime" => SeaOrmFieldType::DateTime,
                "Date" => SeaOrmFieldType::Date,
                "Time" => SeaOrmFieldType::Time,
                "Decimal" => SeaOrmFieldType::Decimal,
                "Value" => SeaOrmFieldType::Json, // serde_json::Value
                "Uuid" => SeaOrmFieldType::Uuid,
                "Vec" => {
                    // Check if Vec<u8>
                    if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments
                        && let Some(syn::GenericArgument::Type(Type::Path(inner_path))) =
                            args.args.first()
                        && let Some(seg) = inner_path.path.segments.last()
                        && seg.ident == "u8"
                    {
                        return Ok((SeaOrmFieldType::Bytes, false));
                    }
                    SeaOrmFieldType::Custom("Vec".to_string())
                }
                other => SeaOrmFieldType::Custom(other.to_string()),
            };

            return Ok((field_type, false));
        }
    }

    Ok((SeaOrmFieldType::Custom("Unknown".to_string()), false))
}

/// Parse field attributes from sea_orm.
fn parse_field_attributes(attrs: &[Attribute]) -> ImportResult<Vec<SeaOrmFieldAttribute>> {
    let mut attributes = vec![];

    for attr in attrs {
        if attr.path().is_ident("sea_orm") {
            // Parse sea_orm attributes
            if let Ok(meta) = attr.parse_args::<syn::Meta>() {
                match meta {
                    syn::Meta::Path(path) => {
                        // Single identifiers like primary_key, auto_increment
                        if path.is_ident("primary_key") {
                            attributes.push(SeaOrmFieldAttribute::PrimaryKey);
                        } else if path.is_ident("auto_increment") {
                            attributes.push(SeaOrmFieldAttribute::AutoIncrement);
                        } else if path.is_ident("unique") {
                            attributes.push(SeaOrmFieldAttribute::Unique);
                        } else if path.is_ident("indexed") {
                            attributes.push(SeaOrmFieldAttribute::Indexed);
                        } else if path.is_ident("nullable") {
                            attributes.push(SeaOrmFieldAttribute::Nullable);
                        }
                    }
                    syn::Meta::NameValue(nv) => {
                        // Key-value pairs like column_name = "..."
                        if nv.path.is_ident("column_name") {
                            if let syn::Expr::Lit(lit) = &nv.value
                                && let syn::Lit::Str(s) = &lit.lit
                            {
                                attributes.push(SeaOrmFieldAttribute::ColumnName(s.value()));
                            }
                        } else if nv.path.is_ident("column_type") {
                            if let syn::Expr::Lit(lit) = &nv.value
                                && let syn::Lit::Str(s) = &lit.lit
                            {
                                attributes.push(SeaOrmFieldAttribute::ColumnType(s.value()));
                            }
                        } else if nv.path.is_ident("default_value")
                            && let syn::Expr::Lit(lit) = &nv.value
                            && let syn::Lit::Str(s) = &lit.lit
                        {
                            attributes.push(SeaOrmFieldAttribute::DefaultValue(s.value()));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(attributes)
}

/// Parse Relation enum.
fn parse_relation_enum(item_enum: syn::ItemEnum) -> ImportResult<Vec<SeaOrmRelation>> {
    let mut relations = vec![];

    for variant in item_enum.variants {
        let name = variant.ident.to_string();

        // Parse sea_orm relation attributes
        for attr in &variant.attrs {
            if attr.path().is_ident("sea_orm")
                && let Some(relation) = parse_relation_attribute(name.clone(), attr)?
            {
                relations.push(relation);
            }
        }
    }

    Ok(relations)
}

/// Parse a single relation attribute.
///
/// Real-world SeaORM relation attributes use the list form, e.g.
/// `#[sea_orm(belongs_to = "super::user::Entity", from = "Column::UserId", to = "Column::Id")]`,
/// so the tokens are parsed as a comma-separated list of `Meta` items. A bare
/// `#[sea_orm(has_many = "...")]` is a one-element list, so both forms are covered.
fn parse_relation_attribute(
    name: String,
    attr: &Attribute,
) -> ImportResult<Option<SeaOrmRelation>> {
    let Ok(metas) = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) else {
        return Ok(None);
    };

    let mut relation_type = None;
    let mut entity = None;
    let mut from = None;
    let mut to = None;

    for meta in metas {
        let Meta::NameValue(nv) = meta else {
            continue;
        };

        if nv.path.is_ident("belongs_to") {
            relation_type = Some(SeaOrmRelationType::BelongsTo);
            entity = extract_entity_name(&nv.value);
        } else if nv.path.is_ident("has_one") {
            relation_type = Some(SeaOrmRelationType::HasOne);
            entity = extract_entity_name(&nv.value);
        } else if nv.path.is_ident("has_many") {
            relation_type = Some(SeaOrmRelationType::HasMany);
            entity = extract_entity_name(&nv.value);
        } else if nv.path.is_ident("from") {
            from = extract_column_refs(&nv.value);
        } else if nv.path.is_ident("to") {
            to = extract_column_refs(&nv.value);
        }
    }

    let (Some(relation_type), Some(entity)) = (relation_type, entity) else {
        return Ok(None);
    };

    Ok(Some(SeaOrmRelation {
        name,
        relation_type,
        entity,
        from,
        to,
        on_delete: None,
        on_update: None,
    }))
}

/// Extract the target entity name from a relation value.
///
/// Handles both the conventional string form (`"super::user::Entity"`) and a
/// bare path (`super::user::Entity`), reducing both to the module name (`user`).
fn extract_entity_name(value: &syn::Expr) -> Option<String> {
    match value {
        syn::Expr::Lit(lit) => {
            if let syn::Lit::Str(s) = &lit.lit {
                s.value()
                    .split("::")
                    .filter(|seg| !seg.is_empty() && *seg != "super" && *seg != "Entity")
                    .last()
                    .map(str::to_string)
            } else {
                None
            }
        }
        syn::Expr::Path(path) => path
            .path
            .segments
            .iter()
            .filter(|seg| seg.ident != "super" && seg.ident != "Entity")
            .map(|seg| seg.ident.to_string())
            .next_back(),
        _ => None,
    }
}

/// Extract column references from a `from`/`to` value.
///
/// Maps the SeaORM `Column` enum path (`"Column::UserId"` or
/// `"super::user::Column::Id"`) to the snake_case struct field name it was
/// derived from (`user_id`, `id`).
fn extract_column_refs(value: &syn::Expr) -> Option<Vec<String>> {
    let column = match value {
        syn::Expr::Lit(lit) => {
            if let syn::Lit::Str(s) = &lit.lit {
                s.value()
                    .split("::")
                    .last()
                    .filter(|seg| !seg.is_empty())?
                    .to_string()
            } else {
                return None;
            }
        }
        syn::Expr::Path(path) => path.path.segments.last()?.ident.to_string(),
        _ => return None,
    };

    Some(vec![column.to_case(Case::Snake)])
}

/// Convert SeaORM entities to Prax schema.
fn convert_seaorm_to_prax(entities: Vec<SeaOrmEntity>) -> ImportResult<Schema> {
    let mut builder = SchemaBuilder::new();

    for entity in entities {
        let model = convert_entity(entity)?;
        builder.add_model(model);
    }

    Ok(builder.build())
}

/// Convert a SeaORM entity to a Prax model.
fn convert_entity(entity: SeaOrmEntity) -> ImportResult<Model> {
    // SeaORM's convention names every entity struct `Model`; that name is
    // useless (and collides across entity files), so derive the model name
    // from the table name instead. Explicit struct names are kept as-is.
    let model_name = if entity.name == "Model" {
        table_name_to_model_name(&entity.table_name)
    } else {
        entity.name.clone()
    };

    let mut model_builder = ModelBuilder::new(&model_name).with_db_name(&entity.table_name);

    // Track which scalar fields are optional so belongs_to relation fields
    // can mirror the nullability of their foreign key.
    let optional_fields: std::collections::HashSet<String> = entity
        .fields
        .iter()
        .filter(|f| f.is_optional)
        .map(|f| f.name.clone())
        .collect();

    // Convert fields
    for field in entity.fields {
        let prax_field = convert_field(field)?;
        model_builder.add_field(prax_field);
    }

    // Convert relations into relation fields:
    // - `belongs_to` (FK-owning side) carries
    //   `@relation(fields: [fk], references: [pk])`; the scalar FK itself is
    //   already emitted above as a regular struct field.
    // - `has_many` / `has_one` emit plain back-relation fields (no
    //   `fields:`/`references:` args), like Prisma's `posts Post[]` /
    //   `profile Profile?`. The field name is the SeaORM relation variant
    //   name (camelCased), which is where users put their own pluralization.
    //   Note: importing entity files one at a time means the target model
    //   may not be part of this import batch — the back-relation then
    //   references a model the rest of the batch is expected to provide
    //   (same constraint as the `belongs_to` side).
    for relation in &entity.relations {
        match relation.relation_type {
            SeaOrmRelationType::BelongsTo => {
                let (Some(from), Some(to)) = (&relation.from, &relation.to) else {
                    continue;
                };

                let target_model = table_name_to_model_name(&relation.entity);
                let field_name = relation.name.to_case(Case::Camel);
                let modifier = if from.iter().all(|fk| optional_fields.contains(fk.as_str())) {
                    TypeModifier::Optional
                } else {
                    TypeModifier::Required
                };

                let relation_field = FieldBuilder::new(
                    &field_name,
                    FieldType::Model(SmolStr::from(target_model.as_str())),
                    modifier,
                )
                .with_relation(None, from.clone(), to.clone(), None, None, None)
                .build();

                model_builder.add_field(relation_field);
            }
            SeaOrmRelationType::HasMany | SeaOrmRelationType::HasOne => {
                let target_model = table_name_to_model_name(&relation.entity);
                let field_name = relation.name.to_case(Case::Camel);
                let modifier = if relation.relation_type == SeaOrmRelationType::HasMany {
                    TypeModifier::List
                } else {
                    // The FK lives on the target side, which may not exist.
                    TypeModifier::Optional
                };

                let relation_field = FieldBuilder::new(
                    &field_name,
                    FieldType::Model(SmolStr::from(target_model.as_str())),
                    modifier,
                )
                .build();

                model_builder.add_field(relation_field);
            }
            SeaOrmRelationType::ManyToMany => {
                // Many-to-many needs a join table SeaORM expresses via a
                // separate pivot entity; import the pivot as its own model
                // instead of synthesizing an implicit join here.
                continue;
            }
        }
    }

    Ok(model_builder.build())
}

/// Convert a SeaORM field to a Prax field.
fn convert_field(field: SeaOrmField) -> ImportResult<Field> {
    let (prax_type, modifier) = convert_field_type(&field.field_type, field.is_optional)?;
    let field_name = field.name.clone();
    let mut field_builder = FieldBuilder::new(&field_name, prax_type, modifier);

    // Convert attributes
    for attr in field.attributes {
        match attr {
            SeaOrmFieldAttribute::PrimaryKey => {
                field_builder = field_builder.with_id();
            }
            SeaOrmFieldAttribute::AutoIncrement => {
                field_builder = field_builder.with_auto();
            }
            SeaOrmFieldAttribute::Unique => {
                field_builder = field_builder.with_unique();
            }
            SeaOrmFieldAttribute::ColumnName(col_name) => {
                field_builder = field_builder.with_map(col_name);
            }
            SeaOrmFieldAttribute::DefaultValue(val) => {
                // Parse default value
                let default_val = if val == "true" {
                    AttributeValue::Boolean(true)
                } else if val == "false" {
                    AttributeValue::Boolean(false)
                } else if let Ok(n) = val.parse::<i64>() {
                    AttributeValue::Int(n)
                } else if let Ok(f) = val.parse::<f64>() {
                    AttributeValue::Float(f)
                } else {
                    AttributeValue::String(val)
                };
                field_builder = field_builder.with_default(default_val);
            }
            _ => {}
        }
    }

    Ok(field_builder.build())
}

/// Convert SeaORM field type to Prax field type.
fn convert_field_type(
    field_type: &SeaOrmFieldType,
    is_optional: bool,
) -> ImportResult<(FieldType, TypeModifier)> {
    let base_type = match field_type {
        SeaOrmFieldType::I32 => FieldType::Scalar(ScalarType::Int),
        SeaOrmFieldType::I64 => FieldType::Scalar(ScalarType::BigInt),
        SeaOrmFieldType::F32 | SeaOrmFieldType::F64 => FieldType::Scalar(ScalarType::Float),
        SeaOrmFieldType::String => FieldType::Scalar(ScalarType::String),
        SeaOrmFieldType::Bool => FieldType::Scalar(ScalarType::Boolean),
        SeaOrmFieldType::DateTime => FieldType::Scalar(ScalarType::DateTime),
        SeaOrmFieldType::Date => FieldType::Scalar(ScalarType::Date),
        SeaOrmFieldType::Time => FieldType::Scalar(ScalarType::Time),
        SeaOrmFieldType::Decimal => FieldType::Scalar(ScalarType::Decimal),
        SeaOrmFieldType::Json => FieldType::Scalar(ScalarType::Json),
        SeaOrmFieldType::Bytes => FieldType::Scalar(ScalarType::Bytes),
        SeaOrmFieldType::Uuid => FieldType::Scalar(ScalarType::Uuid),
        SeaOrmFieldType::Custom(name) => FieldType::Enum(SmolStr::from(name.as_str())),
    };

    let modifier = if is_optional {
        TypeModifier::Optional
    } else {
        TypeModifier::Required
    };

    Ok((base_type, modifier))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_entity() {
        let entity_code = r#"
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "users")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i32,
            pub email: String,
            pub name: Option<String>,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}
        "#;

        let result = parse_seaorm_entity(entity_code);
        assert!(result.is_ok());

        let entity = result.unwrap();
        assert_eq!(entity.name, "Model");
        assert_eq!(entity.table_name, "users");
        assert_eq!(entity.fields.len(), 3);
    }

    #[test]
    fn test_import_entity() {
        let entity_code = r#"
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "posts")]
        pub struct Model {
            #[sea_orm(primary_key, auto_increment)]
            pub id: i32,
            pub title: String,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}
        "#;

        let result = import_seaorm_entity(entity_code);
        assert!(result.is_ok());

        let schema = result.unwrap();
        assert_eq!(schema.models.len(), 1);
    }

    #[test]
    fn test_model_struct_name_derived_from_table() {
        let entity_code = r#"
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "users")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i32,
        }
        "#;

        let schema = import_seaorm_entity(entity_code).unwrap();
        let model = schema.models.values().next().unwrap();
        assert_eq!(model.name(), "User");
        assert!(model.has_attribute("map"));
    }

    #[test]
    fn test_non_model_struct_name_kept() {
        let entity_code = r#"
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "users")]
        pub struct Account {
            #[sea_orm(primary_key)]
            pub id: i32,
        }
        "#;

        let schema = import_seaorm_entity(entity_code).unwrap();
        let model = schema.models.values().next().unwrap();
        assert_eq!(model.name(), "Account");
    }

    #[test]
    fn test_parse_belongs_to_relation() {
        let entity_code = r#"
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "posts")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i32,
            pub user_id: i32,
            pub title: String,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {
            #[sea_orm(belongs_to = "super::user::Entity", from = "Column::UserId", to = "Column::Id")]
            User,
        }
        "#;

        let entity = parse_seaorm_entity(entity_code).unwrap();
        assert_eq!(entity.relations.len(), 1);

        let relation = &entity.relations[0];
        assert_eq!(relation.name, "User");
        assert_eq!(relation.relation_type, SeaOrmRelationType::BelongsTo);
        assert_eq!(relation.entity, "user");
        assert_eq!(relation.from, Some(vec!["user_id".to_string()]));
        assert_eq!(relation.to, Some(vec!["id".to_string()]));
    }

    #[test]
    fn test_import_belongs_to_relation_field() {
        let entity_code = r#"
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "posts")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i32,
            pub user_id: i32,
            pub title: String,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {
            #[sea_orm(belongs_to = "super::user::Entity", from = "Column::UserId", to = "Column::Id")]
            User,
        }
        "#;

        let schema = import_seaorm_entity(entity_code).unwrap();
        let model = schema.models.values().next().unwrap();
        assert_eq!(model.name(), "Post");

        // Scalar FK is still imported as a regular field.
        assert!(model.get_field("user_id").is_some());

        // The belongs_to side gets a relation field pointing at User.
        let relation_field = model.get_field("user").expect("relation field `user`");
        assert_eq!(
            relation_field.field_type,
            FieldType::Model(SmolStr::from("User"))
        );
        assert!(relation_field.has_attribute("relation"));
    }

    #[test]
    fn test_import_has_many_back_relation_field() {
        let entity_code = r#"
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "users")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i32,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {
            #[sea_orm(has_many = "super::post::Entity")]
            Posts,
        }
        "#;

        let schema = import_seaorm_entity(entity_code).unwrap();
        let model = schema.models.values().next().unwrap();
        assert_eq!(model.name(), "User");

        // The has_many side gets a plain back-relation field (no @relation
        // args): `posts Post[]`.
        let field = model
            .get_field("posts")
            .expect("back-relation field `posts`");
        assert_eq!(field.field_type, FieldType::Model(SmolStr::from("Post")));
        assert_eq!(field.modifier, TypeModifier::List);
        assert!(
            !field.has_attribute("relation"),
            "back-relation carries no fields/references args"
        );
    }

    #[test]
    fn test_import_has_one_back_relation_field() {
        let entity_code = r#"
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "users")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i32,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {
            #[sea_orm(has_one = "super::profile::Entity")]
            Profile,
        }
        "#;

        let schema = import_seaorm_entity(entity_code).unwrap();
        let model = schema.models.values().next().unwrap();

        // The has_one side gets an optional back-relation field: `profile Profile?`.
        let field = model
            .get_field("profile")
            .expect("back-relation field `profile`");
        assert_eq!(field.field_type, FieldType::Model(SmolStr::from("Profile")));
        assert_eq!(field.modifier, TypeModifier::Optional);
    }

    #[test]
    fn test_i64_maps_to_bigint() {
        let entity_code = r#"
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "events")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i64,
            pub count: i32,
        }
        "#;

        let schema = import_seaorm_entity(entity_code).unwrap();
        let model = schema.models.values().next().unwrap();

        let id = model.get_field("id").unwrap();
        assert_eq!(id.field_type, FieldType::Scalar(ScalarType::BigInt));

        let count = model.get_field("count").unwrap();
        assert_eq!(count.field_type, FieldType::Scalar(ScalarType::Int));
    }

    #[test]
    fn test_parse_errors_are_seaorm_parse_error() {
        let result = parse_seaorm_entity("not valid rust {{{");
        assert!(matches!(
            result,
            Err(crate::error::ImportError::SeaOrmParseError(_))
        ));

        let result = parse_seaorm_entity("pub struct NotAnEntity;");
        assert!(matches!(
            result,
            Err(crate::error::ImportError::SeaOrmParseError(_))
        ));
    }
}
