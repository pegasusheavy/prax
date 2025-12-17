//! Top-level schema definition.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use super::{CompositeType, Enum, Model, Relation, View};

/// A complete Prax schema.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Schema {
    /// All models in the schema.
    pub models: IndexMap<SmolStr, Model>,
    /// All enums in the schema.
    pub enums: IndexMap<SmolStr, Enum>,
    /// All composite types in the schema.
    pub types: IndexMap<SmolStr, CompositeType>,
    /// All views in the schema.
    pub views: IndexMap<SmolStr, View>,
    /// Raw SQL definitions.
    pub raw_sql: Vec<RawSql>,
    /// Resolved relations (populated after validation).
    pub relations: Vec<Relation>,
}

impl Schema {
    /// Create a new empty schema.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a model to the schema.
    pub fn add_model(&mut self, model: Model) {
        self.models.insert(model.name.name.clone(), model);
    }

    /// Add an enum to the schema.
    pub fn add_enum(&mut self, e: Enum) {
        self.enums.insert(e.name.name.clone(), e);
    }

    /// Add a composite type to the schema.
    pub fn add_type(&mut self, t: CompositeType) {
        self.types.insert(t.name.name.clone(), t);
    }

    /// Add a view to the schema.
    pub fn add_view(&mut self, v: View) {
        self.views.insert(v.name.name.clone(), v);
    }

    /// Add a raw SQL definition.
    pub fn add_raw_sql(&mut self, sql: RawSql) {
        self.raw_sql.push(sql);
    }

    /// Get a model by name.
    pub fn get_model(&self, name: &str) -> Option<&Model> {
        self.models.get(name)
    }

    /// Get a mutable model by name.
    pub fn get_model_mut(&mut self, name: &str) -> Option<&mut Model> {
        self.models.get_mut(name)
    }

    /// Get an enum by name.
    pub fn get_enum(&self, name: &str) -> Option<&Enum> {
        self.enums.get(name)
    }

    /// Get a composite type by name.
    pub fn get_type(&self, name: &str) -> Option<&CompositeType> {
        self.types.get(name)
    }

    /// Get a view by name.
    pub fn get_view(&self, name: &str) -> Option<&View> {
        self.views.get(name)
    }

    /// Check if a type name exists (model, enum, type, or view).
    pub fn type_exists(&self, name: &str) -> bool {
        self.models.contains_key(name)
            || self.enums.contains_key(name)
            || self.types.contains_key(name)
            || self.views.contains_key(name)
    }

    /// Get all model names.
    pub fn model_names(&self) -> impl Iterator<Item = &str> {
        self.models.keys().map(|s| s.as_str())
    }

    /// Get all enum names.
    pub fn enum_names(&self) -> impl Iterator<Item = &str> {
        self.enums.keys().map(|s| s.as_str())
    }

    /// Get relations for a specific model.
    pub fn relations_for(&self, model: &str) -> Vec<&Relation> {
        self.relations
            .iter()
            .filter(|r| r.from_model == model || r.to_model == model)
            .collect()
    }

    /// Get relations originating from a specific model.
    pub fn relations_from(&self, model: &str) -> Vec<&Relation> {
        self.relations
            .iter()
            .filter(|r| r.from_model == model)
            .collect()
    }

    /// Merge another schema into this one.
    pub fn merge(&mut self, other: Schema) {
        self.models.extend(other.models);
        self.enums.extend(other.enums);
        self.types.extend(other.types);
        self.views.extend(other.views);
        self.raw_sql.extend(other.raw_sql);
    }
}

/// A raw SQL definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawSql {
    /// Name/identifier for the SQL (e.g., view name).
    pub name: SmolStr,
    /// The raw SQL content.
    pub sql: String,
}

impl RawSql {
    /// Create a new raw SQL definition.
    pub fn new(name: impl Into<SmolStr>, sql: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sql: sql.into(),
        }
    }
}

/// Schema statistics for debugging/info.
#[derive(Debug, Clone, Default)]
pub struct SchemaStats {
    /// Number of models.
    pub model_count: usize,
    /// Number of enums.
    pub enum_count: usize,
    /// Number of composite types.
    pub type_count: usize,
    /// Number of views.
    pub view_count: usize,
    /// Total number of fields across all models.
    pub field_count: usize,
    /// Number of relations.
    pub relation_count: usize,
}

impl Schema {
    /// Get statistics about the schema.
    pub fn stats(&self) -> SchemaStats {
        SchemaStats {
            model_count: self.models.len(),
            enum_count: self.enums.len(),
            type_count: self.types.len(),
            view_count: self.views.len(),
            field_count: self.models.values().map(|m| m.fields.len()).sum(),
            relation_count: self.relations.len(),
        }
    }
}

impl std::fmt::Display for Schema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stats = self.stats();
        write!(
            f,
            "Schema({} models, {} enums, {} types, {} views, {} fields, {} relations)",
            stats.model_count,
            stats.enum_count,
            stats.type_count,
            stats.view_count,
            stats.field_count,
            stats.relation_count
        )
    }
}

