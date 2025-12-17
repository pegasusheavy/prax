//! Pest grammar parser for Prax schema files.

use pest_derive::Parser;

/// The Prax schema parser.
#[derive(Parser)]
#[grammar = "parser/prax.pest"]
pub struct PraxParser;

#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser;

    #[test]
    fn test_parse_identifier() {
        let result = PraxParser::parse(Rule::identifier, "User");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_field_type() {
        let result = PraxParser::parse(Rule::field_type, "String?");
        assert!(result.is_ok());

        let result = PraxParser::parse(Rule::field_type, "Post[]");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_attribute() {
        let result = PraxParser::parse(Rule::field_attribute, "@id");
        assert!(result.is_ok());

        let result = PraxParser::parse(Rule::field_attribute, "@default(now())");
        assert!(result.is_ok());

        let result = PraxParser::parse(
            Rule::field_attribute,
            "@relation(fields: [author_id], references: [id])",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_model() {
        let input = r#"model User {
            id    Int    @id @auto
            email String @unique
        }"#;
        let result = PraxParser::parse(Rule::model_def, input);
        assert!(result.is_ok(), "Failed to parse model: {:?}", result.err());
    }

    #[test]
    fn test_parse_enum() {
        let input = r#"enum Role {
            User
            Admin
        }"#;
        let result = PraxParser::parse(Rule::enum_def, input);
        assert!(result.is_ok());
    }
}
