//! Code generation for Prax enums.

use proc_macro2::TokenStream;
use quote::quote;

use prax_schema::ast::Enum;

use super::{generate_doc_comment, pascal_ident, snake_ident};

/// Generate the module for an enum definition.
pub fn generate_enum_module(enum_def: &Enum) -> Result<TokenStream, syn::Error> {
    let enum_name = pascal_ident(enum_def.name());
    let module_name = snake_ident(enum_def.name());

    let doc = generate_doc_comment(enum_def.documentation.as_ref().map(|d| d.text.as_str()));

    // Generate variants
    let variants: Vec<_> = enum_def
        .variants
        .iter()
        .map(|variant| {
            let variant_name = pascal_ident(variant.name());
            let variant_doc =
                generate_doc_comment(variant.documentation.as_ref().map(|d| d.text.as_str()));

            // Check for @map attribute to get the database value
            let db_value = variant.db_value().to_string();

            quote! {
                #variant_doc
                #[serde(rename = #db_value)]
                #variant_name
            }
        })
        .collect();

    // Generate variant names for serialization
    let variant_names: Vec<_> = enum_def
        .variants
        .iter()
        .map(|v| pascal_ident(v.name()))
        .collect();

    let variant_strs: Vec<_> = enum_def
        .variants
        .iter()
        .map(|v| v.db_value().to_string())
        .collect();

    // Get database enum name if @@map is present
    let db_name = enum_def.db_name().to_string();
    let db_name_str = db_name.as_str();

    Ok(quote! {
        #doc
        pub mod #module_name {
            use serde::{Deserialize, Serialize};

            /// Database enum name.
            pub const DB_NAME: &str = #db_name_str;

            #doc
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
            #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
            pub enum #enum_name {
                #(#variants,)*
            }

            impl #enum_name {
                /// Get all variants of this enum.
                pub const fn variants() -> &'static [Self] {
                    &[#(Self::#variant_names,)*]
                }

                /// Get the database value for this variant.
                pub const fn as_str(&self) -> &'static str {
                    match self {
                        #(Self::#variant_names => #variant_strs,)*
                    }
                }

                /// Parse from database value.
                pub fn from_str(s: &str) -> Option<Self> {
                    match s {
                        #(#variant_strs => Some(Self::#variant_names),)*
                        _ => None,
                    }
                }
            }

            impl std::fmt::Display for #enum_name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}", self.as_str())
                }
            }

            impl std::str::FromStr for #enum_name {
                type Err = String;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    Self::from_str(s).ok_or_else(|| {
                        format!("Unknown {} variant: {}", stringify!(#enum_name), s)
                    })
                }
            }

            impl Default for #enum_name {
                fn default() -> Self {
                    Self::variants()[0]
                }
            }
        }

        // Re-export the enum at the parent level
        pub use #module_name::#enum_name;
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_schema::ast::{EnumVariant, Ident, Span};

    fn make_span() -> Span {
        Span::new(0, 0)
    }

    fn make_ident(name: &str) -> Ident {
        Ident::new(name, make_span())
    }

    #[test]
    fn test_generate_simple_enum() {
        let mut enum_def = Enum::new(make_ident("Role"), make_span());
        enum_def.add_variant(EnumVariant::new(make_ident("USER"), make_span()));
        enum_def.add_variant(EnumVariant::new(make_ident("ADMIN"), make_span()));

        let result = generate_enum_module(&enum_def);
        assert!(result.is_ok());

        let code = result.unwrap().to_string();
        assert!(code.contains("pub mod role"));
        assert!(code.contains("pub enum Role"));
        assert!(code.contains("User"));
        assert!(code.contains("Admin"));
    }

    #[test]
    fn test_generate_enum_with_documentation() {
        use prax_schema::ast::Documentation;

        let doc = Documentation::new("User status", make_span());
        let mut enum_def =
            Enum::new(make_ident("Status"), make_span()).with_documentation(doc);
        enum_def.add_variant(EnumVariant::new(make_ident("ACTIVE"), make_span()));
        enum_def.add_variant(EnumVariant::new(make_ident("INACTIVE"), make_span()));

        let result = generate_enum_module(&enum_def);
        assert!(result.is_ok());

        let code = result.unwrap().to_string();
        assert!(code.contains("User status"));
    }
}

