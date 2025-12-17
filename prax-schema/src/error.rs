//! Error types for schema parsing and validation.

use miette::Diagnostic;
use thiserror::Error;

/// Result type for schema operations.
pub type SchemaResult<T> = Result<T, SchemaError>;

/// Errors that can occur during schema parsing and validation.
#[derive(Error, Debug, Diagnostic)]
pub enum SchemaError {
    /// Error reading a file.
    #[error("failed to read file: {path}")]
    #[diagnostic(code(prax::schema::io_error))]
    IoError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Syntax error in the schema file.
    #[error("syntax error in schema")]
    #[diagnostic(code(prax::schema::syntax_error))]
    SyntaxError {
        #[source_code]
        src: String,
        #[label("error here")]
        span: miette::SourceSpan,
        message: String,
    },

    /// Invalid model definition.
    #[error("invalid model `{name}`: {message}")]
    #[diagnostic(code(prax::schema::invalid_model))]
    InvalidModel { name: String, message: String },

    /// Invalid field definition.
    #[error("invalid field `{model}.{field}`: {message}")]
    #[diagnostic(code(prax::schema::invalid_field))]
    InvalidField {
        model: String,
        field: String,
        message: String,
    },

    /// Invalid relation definition.
    #[error("invalid relation `{model}.{field}`: {message}")]
    #[diagnostic(code(prax::schema::invalid_relation))]
    InvalidRelation {
        model: String,
        field: String,
        message: String,
    },

    /// Duplicate definition.
    #[error("duplicate {kind} `{name}`")]
    #[diagnostic(code(prax::schema::duplicate))]
    Duplicate { kind: String, name: String },

    /// Unknown type reference.
    #[error("unknown type `{type_name}` in `{model}.{field}`")]
    #[diagnostic(code(prax::schema::unknown_type))]
    UnknownType {
        model: String,
        field: String,
        type_name: String,
    },

    /// Invalid attribute.
    #[error("invalid attribute `@{attribute}`: {message}")]
    #[diagnostic(code(prax::schema::invalid_attribute))]
    InvalidAttribute { attribute: String, message: String },

    /// Missing required attribute.
    #[error("model `{model}` is missing required `@id` field")]
    #[diagnostic(code(prax::schema::missing_id))]
    MissingId { model: String },

    /// Configuration error.
    #[error("configuration error: {message}")]
    #[diagnostic(code(prax::schema::config_error))]
    ConfigError { message: String },

    /// TOML parsing error.
    #[error("failed to parse TOML")]
    #[diagnostic(code(prax::schema::toml_error))]
    TomlError {
        #[source]
        source: toml::de::Error,
    },

    /// Validation error with multiple issues.
    #[error("schema validation failed with {count} error(s)")]
    #[diagnostic(code(prax::schema::validation_failed))]
    ValidationFailed {
        count: usize,
        #[related]
        errors: Vec<SchemaError>,
    },
}

impl SchemaError {
    /// Create a syntax error with source location.
    pub fn syntax(src: impl Into<String>, offset: usize, len: usize, message: impl Into<String>) -> Self {
        Self::SyntaxError {
            src: src.into(),
            span: (offset, len).into(),
            message: message.into(),
        }
    }

    /// Create an invalid model error.
    pub fn invalid_model(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidModel {
            name: name.into(),
            message: message.into(),
        }
    }

    /// Create an invalid field error.
    pub fn invalid_field(
        model: impl Into<String>,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::InvalidField {
            model: model.into(),
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create an invalid relation error.
    pub fn invalid_relation(
        model: impl Into<String>,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::InvalidRelation {
            model: model.into(),
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create a duplicate definition error.
    pub fn duplicate(kind: impl Into<String>, name: impl Into<String>) -> Self {
        Self::Duplicate {
            kind: kind.into(),
            name: name.into(),
        }
    }

    /// Create an unknown type error.
    pub fn unknown_type(
        model: impl Into<String>,
        field: impl Into<String>,
        type_name: impl Into<String>,
    ) -> Self {
        Self::UnknownType {
            model: model.into(),
            field: field.into(),
            type_name: type_name.into(),
        }
    }
}

