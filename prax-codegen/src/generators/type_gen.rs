//! Code generation for Prax composite types.

use proc_macro2::TokenStream;
use quote::quote;

use prax_schema::ast::CompositeType;

use super::{generate_doc_comment, pascal_ident, snake_ident};
use crate::types::field_type_to_rust;

/// Generate the module for a composite type definition.
pub fn generate_type_module(type_def: &CompositeType) -> Result<TokenStream, syn::Error> {
    let type_name = pascal_ident(type_def.name());
    let module_name = snake_ident(type_def.name());

    let doc = generate_doc_comment(type_def.documentation.as_ref().map(|d| d.text.as_str()));

    // Generate struct fields
    let fields: Vec<_> = type_def
        .fields
        .values()
        .map(|field| {
            let field_name = snake_ident(field.name());
            let field_type = field_type_to_rust(&field.field_type, &field.modifier);
            let field_doc =
                generate_doc_comment(field.documentation.as_ref().map(|d| d.text.as_str()));

            // Check for @map attribute to get the serialized name
            let serde_rename = field
                .attributes
                .iter()
                .find(|a| a.name() == "map")
                .and_then(|a| a.first_arg())
                .and_then(|v| v.as_string())
                .map(|name| quote! { #[serde(rename = #name)] })
                .unwrap_or_default();

            quote! {
                #field_doc
                #serde_rename
                pub #field_name: #field_type
            }
        })
        .collect();

    // Generate field names for the new() constructor
    let field_names: Vec<_> = type_def
        .fields
        .values()
        .map(|f| snake_ident(f.name()))
        .collect();

    let field_types: Vec<_> = type_def
        .fields
        .values()
        .map(|f| field_type_to_rust(&f.field_type, &f.modifier))
        .collect();

    Ok(quote! {
        #doc
        pub mod #module_name {
            use serde::{Deserialize, Serialize};

            #doc
            #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
            pub struct #type_name {
                #(#fields,)*
            }

            impl #type_name {
                /// Create a new instance with all required fields.
                pub fn new(#(#field_names: #field_types),*) -> Self {
                    Self {
                        #(#field_names,)*
                    }
                }
            }

            impl Default for #type_name {
                fn default() -> Self {
                    Self {
                        #(#field_names: Default::default(),)*
                    }
                }
            }
        }

        // Re-export the type at the parent level
        pub use #module_name::#type_name;
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_schema::ast::{Field, FieldType, Ident, ScalarType, Span, TypeModifier};

    fn make_span() -> Span {
        Span::new(0, 0)
    }

    fn make_ident(name: &str) -> Ident {
        Ident::new(name, make_span())
    }

    #[test]
    fn test_generate_simple_type() {
        let mut type_def = CompositeType::new(make_ident("Address"), make_span());
        type_def.add_field(Field::new(
            make_ident("street"),
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
            vec![],
            make_span(),
        ));
        type_def.add_field(Field::new(
            make_ident("city"),
            FieldType::Scalar(ScalarType::String),
            TypeModifier::Required,
            vec![],
            make_span(),
        ));

        let result = generate_type_module(&type_def);
        assert!(result.is_ok());

        let code = result.unwrap().to_string();
        assert!(code.contains("pub mod address"));
        assert!(code.contains("pub struct Address"));
        assert!(code.contains("pub street : String"));
        assert!(code.contains("pub city : String"));
    }
}

