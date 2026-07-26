use serde::{Deserialize, Serialize};

use super::{Ident, Span};

/// A stored procedure or function definition in the schema AST.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Procedure {
    pub name: Ident,
    pub is_function: bool,
    pub body: String,
    pub span: Span,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<crate::loader::SourceId>,
}

impl Procedure {
    pub fn new(name: Ident, is_function: bool, body: String, span: Span) -> Self {
        Self {
            name,
            is_function,
            body,
            span,
            source_id: None,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}
