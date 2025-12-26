//! Select specifications for choosing which fields to return.

use std::collections::{HashMap, HashSet};

/// Specification for which fields to select from a model.
#[derive(Debug, Clone)]
pub struct SelectSpec {
    /// Model name this selection is for.
    pub model_name: String,
    /// Fields to include (empty means all).
    pub fields: FieldSelection,
    /// Relation selections.
    pub relations: HashMap<String, SelectSpec>,
}

impl SelectSpec {
    /// Create a new select spec for a model.
    pub fn new(model_name: impl Into<String>) -> Self {
        Self {
            model_name: model_name.into(),
            fields: FieldSelection::All,
            relations: HashMap::new(),
        }
    }

    /// Select all fields.
    pub fn all(model_name: impl Into<String>) -> Self {
        Self {
            model_name: model_name.into(),
            fields: FieldSelection::All,
            relations: HashMap::new(),
        }
    }

    /// Select only specific fields.
    pub fn only(
        model_name: impl Into<String>,
        fields: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            model_name: model_name.into(),
            fields: FieldSelection::Only(fields.into_iter().map(Into::into).collect()),
            relations: HashMap::new(),
        }
    }

    /// Exclude specific fields.
    pub fn except(
        model_name: impl Into<String>,
        fields: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            model_name: model_name.into(),
            fields: FieldSelection::Except(fields.into_iter().map(Into::into).collect()),
            relations: HashMap::new(),
        }
    }

    /// Add a field to the selection.
    pub fn field(mut self, name: impl Into<String>) -> Self {
        match &mut self.fields {
            FieldSelection::All => {
                self.fields = FieldSelection::Only(HashSet::from([name.into()]));
            }
            FieldSelection::Only(fields) => {
                fields.insert(name.into());
            }
            FieldSelection::Except(fields) => {
                fields.remove(&name.into());
            }
        }
        self
    }

    /// Add multiple fields to the selection.
    pub fn fields(mut self, names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for name in names {
            self = self.field(name);
        }
        self
    }

    /// Include a relation with its selection.
    pub fn relation(mut self, name: impl Into<String>, select: SelectSpec) -> Self {
        self.relations.insert(name.into(), select);
        self
    }

    /// Check if a field is selected.
    pub fn is_field_selected(&self, field: &str) -> bool {
        self.fields.includes(field)
    }

    /// Get the list of selected fields (if explicit).
    pub fn selected_fields(&self) -> Option<&HashSet<String>> {
        match &self.fields {
            FieldSelection::Only(fields) => Some(fields),
            _ => None,
        }
    }

    /// Get the list of excluded fields (if explicit).
    pub fn excluded_fields(&self) -> Option<&HashSet<String>> {
        match &self.fields {
            FieldSelection::Except(fields) => Some(fields),
            _ => None,
        }
    }

    /// Check if all fields are selected.
    pub fn is_all(&self) -> bool {
        matches!(self.fields, FieldSelection::All)
    }

    /// Generate the SQL column list for this selection.
    pub fn to_sql_columns(&self, all_columns: &[&str], table_alias: Option<&str>) -> String {
        let columns: Vec<_> = match &self.fields {
            FieldSelection::All => all_columns.iter().map(|&s| s.to_string()).collect(),
            FieldSelection::Only(fields) => all_columns
                .iter()
                .filter(|&c| fields.contains(*c))
                .map(|&s| s.to_string())
                .collect(),
            FieldSelection::Except(fields) => all_columns
                .iter()
                .filter(|&c| !fields.contains(*c))
                .map(|&s| s.to_string())
                .collect(),
        };

        match table_alias {
            Some(alias) => columns
                .into_iter()
                .map(|c| format!("{}.{}", alias, c))
                .collect::<Vec<_>>()
                .join(", "),
            None => columns.join(", "),
        }
    }
}

/// Field selection mode.
#[derive(Debug, Clone, Default)]
pub enum FieldSelection {
    /// Select all fields.
    #[default]
    All,
    /// Select only these fields.
    Only(HashSet<String>),
    /// Select all except these fields.
    Except(HashSet<String>),
}

impl FieldSelection {
    /// Check if a field is included in this selection.
    pub fn includes(&self, field: &str) -> bool {
        match self {
            Self::All => true,
            Self::Only(fields) => fields.contains(field),
            Self::Except(fields) => !fields.contains(field),
        }
    }

    /// Check if this is an "all" selection.
    pub fn is_all(&self) -> bool {
        matches!(self, Self::All)
    }
}

/// Helper function to create a select spec.
pub fn select(model: impl Into<String>) -> SelectSpec {
    SelectSpec::new(model)
}

/// Helper function to select only specific fields.
pub fn select_only(
    model: impl Into<String>,
    fields: impl IntoIterator<Item = impl Into<String>>,
) -> SelectSpec {
    SelectSpec::only(model, fields)
}

/// Helper function to select all fields except some.
pub fn select_except(
    model: impl Into<String>,
    fields: impl IntoIterator<Item = impl Into<String>>,
) -> SelectSpec {
    SelectSpec::except(model, fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_spec_all() {
        let spec = SelectSpec::all("User");
        assert!(spec.is_all());
        assert!(spec.is_field_selected("id"));
        assert!(spec.is_field_selected("email"));
    }

    #[test]
    fn test_select_spec_only() {
        let spec = SelectSpec::only("User", ["id", "email"]);
        assert!(!spec.is_all());
        assert!(spec.is_field_selected("id"));
        assert!(spec.is_field_selected("email"));
        assert!(!spec.is_field_selected("password"));
    }

    #[test]
    fn test_select_spec_except() {
        let spec = SelectSpec::except("User", ["password"]);
        assert!(!spec.is_all());
        assert!(spec.is_field_selected("id"));
        assert!(!spec.is_field_selected("password"));
    }

    #[test]
    fn test_select_spec_with_relation() {
        let spec = SelectSpec::only("User", ["id", "name"])
            .relation("posts", SelectSpec::only("Post", ["id", "title"]));

        assert!(spec.relations.contains_key("posts"));
    }

    #[test]
    fn test_to_sql_columns() {
        let spec = SelectSpec::only("User", ["id", "email"]);
        let columns = spec.to_sql_columns(&["id", "email", "name", "password"], None);
        assert!(columns.contains("id"));
        assert!(columns.contains("email"));
        assert!(!columns.contains("password"));
    }

    #[test]
    fn test_to_sql_columns_with_alias() {
        let spec = SelectSpec::only("User", ["id", "email"]);
        let columns = spec.to_sql_columns(&["id", "email"], Some("u"));
        assert!(columns.contains("u.id"));
        assert!(columns.contains("u.email"));
    }
}
