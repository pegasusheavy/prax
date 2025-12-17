//! Relation analysis for the Prax schema AST.

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use super::ReferentialAction;

/// The type of relation between two models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationType {
    /// One-to-one relation.
    OneToOne,
    /// One-to-many relation.
    OneToMany,
    /// Many-to-one relation (inverse of one-to-many).
    ManyToOne,
    /// Many-to-many relation.
    ManyToMany,
}

impl RelationType {
    /// Check if this is a "to-one" relation.
    pub fn is_to_one(&self) -> bool {
        matches!(self, Self::OneToOne | Self::ManyToOne)
    }

    /// Check if this is a "to-many" relation.
    pub fn is_to_many(&self) -> bool {
        matches!(self, Self::OneToMany | Self::ManyToMany)
    }

    /// Check if this is a "from-many" relation.
    pub fn is_from_many(&self) -> bool {
        matches!(self, Self::ManyToOne | Self::ManyToMany)
    }
}

impl std::fmt::Display for RelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OneToOne => write!(f, "1:1"),
            Self::OneToMany => write!(f, "1:n"),
            Self::ManyToOne => write!(f, "n:1"),
            Self::ManyToMany => write!(f, "m:n"),
        }
    }
}

/// A resolved relation between two models.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relation {
    /// Relation name (for disambiguation when multiple relations exist).
    pub name: Option<SmolStr>,
    /// The model containing the foreign key.
    pub from_model: SmolStr,
    /// The field on the from model.
    pub from_field: SmolStr,
    /// The foreign key field(s) on the from model.
    pub from_fields: Vec<SmolStr>,
    /// The model being referenced.
    pub to_model: SmolStr,
    /// The field on the to model (back-relation).
    pub to_field: Option<SmolStr>,
    /// The referenced field(s) on the to model.
    pub to_fields: Vec<SmolStr>,
    /// The type of relation.
    pub relation_type: RelationType,
    /// On delete action.
    pub on_delete: Option<ReferentialAction>,
    /// On update action.
    pub on_update: Option<ReferentialAction>,
}

impl Relation {
    /// Create a new relation.
    pub fn new(
        from_model: impl Into<SmolStr>,
        from_field: impl Into<SmolStr>,
        to_model: impl Into<SmolStr>,
        relation_type: RelationType,
    ) -> Self {
        Self {
            name: None,
            from_model: from_model.into(),
            from_field: from_field.into(),
            from_fields: vec![],
            to_model: to_model.into(),
            to_field: None,
            to_fields: vec![],
            relation_type,
            on_delete: None,
            on_update: None,
        }
    }

    /// Set the relation name.
    pub fn with_name(mut self, name: impl Into<SmolStr>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the foreign key fields.
    pub fn with_from_fields(mut self, fields: Vec<SmolStr>) -> Self {
        self.from_fields = fields;
        self
    }

    /// Set the referenced fields.
    pub fn with_to_fields(mut self, fields: Vec<SmolStr>) -> Self {
        self.to_fields = fields;
        self
    }

    /// Set the back-relation field.
    pub fn with_to_field(mut self, field: impl Into<SmolStr>) -> Self {
        self.to_field = Some(field.into());
        self
    }

    /// Set the on delete action.
    pub fn with_on_delete(mut self, action: ReferentialAction) -> Self {
        self.on_delete = Some(action);
        self
    }

    /// Set the on update action.
    pub fn with_on_update(mut self, action: ReferentialAction) -> Self {
        self.on_update = Some(action);
        self
    }

    /// Check if this is an implicit many-to-many relation.
    pub fn is_implicit_many_to_many(&self) -> bool {
        self.relation_type == RelationType::ManyToMany && self.from_fields.is_empty()
    }

    /// Get the join table name for many-to-many relations.
    pub fn join_table_name(&self) -> Option<String> {
        if self.relation_type != RelationType::ManyToMany {
            return None;
        }

        // Sort model names for consistent naming
        let mut names = [self.from_model.as_str(), self.to_model.as_str()];
        names.sort();

        Some(format!("_{}_to_{}", names[0], names[1]))
    }
}

/// Index definition for a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Index {
    /// Index name (auto-generated if not specified).
    pub name: Option<SmolStr>,
    /// Fields included in the index.
    pub fields: Vec<IndexField>,
    /// Whether this is a unique index.
    pub is_unique: bool,
    /// Index type (btree, hash, etc.).
    pub index_type: Option<IndexType>,
}

impl Index {
    /// Create a new index.
    pub fn new(fields: Vec<IndexField>) -> Self {
        Self {
            name: None,
            fields,
            is_unique: false,
            index_type: None,
        }
    }

    /// Create a unique index.
    pub fn unique(fields: Vec<IndexField>) -> Self {
        Self {
            name: None,
            fields,
            is_unique: true,
            index_type: None,
        }
    }

    /// Set the index name.
    pub fn with_name(mut self, name: impl Into<SmolStr>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the index type.
    pub fn with_type(mut self, index_type: IndexType) -> Self {
        self.index_type = Some(index_type);
        self
    }
}

/// A field in an index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexField {
    /// Field name.
    pub name: SmolStr,
    /// Sort order.
    pub sort: SortOrder,
}

impl IndexField {
    /// Create a new index field with ascending order.
    pub fn asc(name: impl Into<SmolStr>) -> Self {
        Self {
            name: name.into(),
            sort: SortOrder::Asc,
        }
    }

    /// Create a new index field with descending order.
    pub fn desc(name: impl Into<SmolStr>) -> Self {
        Self {
            name: name.into(),
            sort: SortOrder::Desc,
        }
    }
}

/// Sort order for index fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SortOrder {
    /// Ascending order.
    #[default]
    Asc,
    /// Descending order.
    Desc,
}

/// Index type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexType {
    /// B-tree index (default).
    BTree,
    /// Hash index.
    Hash,
    /// GiST index (PostgreSQL).
    Gist,
    /// GIN index (PostgreSQL).
    Gin,
    /// Full-text search index.
    FullText,
}

impl IndexType {
    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "btree" => Some(Self::BTree),
            "hash" => Some(Self::Hash),
            "gist" => Some(Self::Gist),
            "gin" => Some(Self::Gin),
            "fulltext" => Some(Self::FullText),
            _ => None,
        }
    }
}

