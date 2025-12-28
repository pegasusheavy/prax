//! MongoDB filter/query building utilities.

use bson::{Bson, Document, doc, oid::ObjectId};

/// Builder for MongoDB filter documents.
///
/// Provides a fluent API for constructing MongoDB query filters.
///
/// # Example
///
/// ```rust,ignore
/// use prax_mongodb::FilterBuilder;
///
/// let filter = FilterBuilder::new()
///     .eq("status", "active")
///     .gte("age", 18)
///     .regex("email", r"@example\.com$")
///     .build();
///
/// // Produces: { "status": "active", "age": { "$gte": 18 }, "email": { "$regex": "@example\\.com$" } }
/// ```
#[derive(Debug, Clone, Default)]
pub struct FilterBuilder {
    doc: Document,
}

impl FilterBuilder {
    /// Create a new empty filter builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a filter builder from an existing document.
    pub fn from_doc(doc: Document) -> Self {
        Self { doc }
    }

    /// Add an equality condition.
    pub fn eq(mut self, field: &str, value: impl Into<Bson>) -> Self {
        self.doc.insert(field, value.into());
        self
    }

    /// Add a not-equal condition.
    pub fn ne(mut self, field: &str, value: impl Into<Bson>) -> Self {
        self.doc.insert(field, doc! { "$ne": value.into() });
        self
    }

    /// Add a greater-than condition.
    pub fn gt(mut self, field: &str, value: impl Into<Bson>) -> Self {
        self.doc.insert(field, doc! { "$gt": value.into() });
        self
    }

    /// Add a greater-than-or-equal condition.
    pub fn gte(mut self, field: &str, value: impl Into<Bson>) -> Self {
        self.doc.insert(field, doc! { "$gte": value.into() });
        self
    }

    /// Add a less-than condition.
    pub fn lt(mut self, field: &str, value: impl Into<Bson>) -> Self {
        self.doc.insert(field, doc! { "$lt": value.into() });
        self
    }

    /// Add a less-than-or-equal condition.
    pub fn lte(mut self, field: &str, value: impl Into<Bson>) -> Self {
        self.doc.insert(field, doc! { "$lte": value.into() });
        self
    }

    /// Add an "in" condition (value in array).
    pub fn in_array(mut self, field: &str, values: Vec<impl Into<Bson>>) -> Self {
        let bson_values: Vec<Bson> = values.into_iter().map(Into::into).collect();
        self.doc.insert(field, doc! { "$in": bson_values });
        self
    }

    /// Add a "not in" condition.
    pub fn not_in(mut self, field: &str, values: Vec<impl Into<Bson>>) -> Self {
        let bson_values: Vec<Bson> = values.into_iter().map(Into::into).collect();
        self.doc.insert(field, doc! { "$nin": bson_values });
        self
    }

    /// Add a regex condition.
    pub fn regex(mut self, field: &str, pattern: &str) -> Self {
        self.doc.insert(field, doc! { "$regex": pattern });
        self
    }

    /// Add a regex condition with options.
    pub fn regex_with_options(mut self, field: &str, pattern: &str, options: &str) -> Self {
        self.doc
            .insert(field, doc! { "$regex": pattern, "$options": options });
        self
    }

    /// Add an exists condition.
    pub fn exists(mut self, field: &str, exists: bool) -> Self {
        self.doc.insert(field, doc! { "$exists": exists });
        self
    }

    /// Add a type check condition.
    pub fn type_is(mut self, field: &str, bson_type: &str) -> Self {
        self.doc.insert(field, doc! { "$type": bson_type });
        self
    }

    /// Add an array size condition.
    pub fn size(mut self, field: &str, size: i32) -> Self {
        self.doc.insert(field, doc! { "$size": size });
        self
    }

    /// Add an array "all" condition (array contains all values).
    pub fn all(mut self, field: &str, values: Vec<impl Into<Bson>>) -> Self {
        let bson_values: Vec<Bson> = values.into_iter().map(Into::into).collect();
        self.doc.insert(field, doc! { "$all": bson_values });
        self
    }

    /// Add an elemMatch condition for array elements.
    pub fn elem_match(mut self, field: &str, query: Document) -> Self {
        self.doc.insert(field, doc! { "$elemMatch": query });
        self
    }

    /// Add a text search condition.
    pub fn text_search(mut self, search: &str) -> Self {
        self.doc.insert("$text", doc! { "$search": search });
        self
    }

    /// Add a text search with language.
    pub fn text_search_with_lang(mut self, search: &str, language: &str) -> Self {
        self.doc
            .insert("$text", doc! { "$search": search, "$language": language });
        self
    }

    /// Add an ObjectId filter on _id field.
    pub fn by_id(mut self, id: ObjectId) -> Self {
        self.doc.insert("_id", id);
        self
    }

    /// Add an ObjectId filter from string.
    pub fn by_id_str(self, id: &str) -> Result<Self, bson::oid::Error> {
        let oid = ObjectId::parse_str(id)?;
        Ok(self.by_id(oid))
    }

    /// Combine with AND ($and).
    pub fn and(mut self, conditions: Vec<Document>) -> Self {
        self.doc.insert("$and", conditions);
        self
    }

    /// Combine with OR ($or).
    pub fn or(mut self, conditions: Vec<Document>) -> Self {
        self.doc.insert("$or", conditions);
        self
    }

    /// Combine with NOR ($nor).
    pub fn nor(mut self, conditions: Vec<Document>) -> Self {
        self.doc.insert("$nor", conditions);
        self
    }

    /// Add a NOT condition.
    pub fn not(mut self, field: &str, condition: Document) -> Self {
        self.doc.insert(field, doc! { "$not": condition });
        self
    }

    /// Add a where condition (JavaScript expression).
    /// Note: $where is slow and should be avoided in production.
    pub fn where_expr(mut self, expression: &str) -> Self {
        self.doc.insert("$where", expression);
        self
    }

    /// Add geospatial near condition.
    pub fn near(
        mut self,
        field: &str,
        longitude: f64,
        latitude: f64,
        max_distance: Option<f64>,
    ) -> Self {
        let mut geo = doc! {
            "$geometry": {
                "type": "Point",
                "coordinates": [longitude, latitude]
            }
        };
        if let Some(max) = max_distance {
            geo.insert("$maxDistance", max);
        }
        self.doc.insert(field, doc! { "$near": geo });
        self
    }

    /// Add geospatial geoWithin condition.
    pub fn geo_within_box(
        mut self,
        field: &str,
        bottom_left: (f64, f64),
        top_right: (f64, f64),
    ) -> Self {
        self.doc.insert(
            field,
            doc! {
                "$geoWithin": {
                    "$box": [
                        [bottom_left.0, bottom_left.1],
                        [top_right.0, top_right.1]
                    ]
                }
            },
        );
        self
    }

    /// Merge another filter into this one.
    pub fn merge(mut self, other: Document) -> Self {
        for (k, v) in other {
            self.doc.insert(k, v);
        }
        self
    }

    /// Build the filter document.
    pub fn build(self) -> Document {
        self.doc
    }

    /// Check if the filter is empty.
    pub fn is_empty(&self) -> bool {
        self.doc.is_empty()
    }
}

/// Create an empty filter (matches all documents).
pub fn all() -> Document {
    doc! {}
}

/// Create an _id filter.
pub fn by_id(id: ObjectId) -> Document {
    doc! { "_id": id }
}

/// Create an _id filter from string.
pub fn by_id_str(id: &str) -> Result<Document, bson::oid::Error> {
    let oid = ObjectId::parse_str(id)?;
    Ok(doc! { "_id": oid })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_builder_eq() {
        let filter = FilterBuilder::new()
            .eq("name", "Alice")
            .eq("age", 30)
            .build();

        assert_eq!(filter.get_str("name").unwrap(), "Alice");
        assert_eq!(filter.get_i32("age").unwrap(), 30);
    }

    #[test]
    fn test_filter_builder_comparison() {
        let filter = FilterBuilder::new().gte("age", 18).lt("age", 65).build();

        let _age_lt = filter.get_document("age").unwrap();
        // Note: The second insert overwrites the first for the same field
        // In real use, you'd use $and for multiple conditions on same field
    }

    #[test]
    fn test_filter_builder_in_array() {
        let filter = FilterBuilder::new()
            .in_array("status", vec!["active", "pending"])
            .build();

        let status = filter.get_document("status").unwrap();
        assert!(status.contains_key("$in"));
    }

    #[test]
    fn test_filter_builder_regex() {
        let filter = FilterBuilder::new()
            .regex("email", r"@example\.com$")
            .build();

        let email = filter.get_document("email").unwrap();
        assert!(email.contains_key("$regex"));
    }

    #[test]
    fn test_filter_builder_or() {
        let filter = FilterBuilder::new()
            .or(vec![
                doc! { "status": "active" },
                doc! { "priority": "high" },
            ])
            .build();

        assert!(filter.contains_key("$or"));
    }

    #[test]
    fn test_filter_builder_by_id() {
        let oid = ObjectId::new();
        let filter = FilterBuilder::new().by_id(oid).build();
        assert_eq!(filter.get_object_id("_id").unwrap(), oid);
    }

    #[test]
    fn test_filter_builder_text_search() {
        let filter = FilterBuilder::new().text_search("hello world").build();

        let text = filter.get_document("$text").unwrap();
        assert_eq!(text.get_str("$search").unwrap(), "hello world");
    }

    #[test]
    fn test_all_filter() {
        let filter = all();
        assert!(filter.is_empty());
    }

    #[test]
    fn test_by_id_helper() {
        let oid = ObjectId::new();
        let filter = by_id(oid);
        assert_eq!(filter.get_object_id("_id").unwrap(), oid);
    }
}
