//! Document mapping and conversion utilities.

use bson::{Bson, Document, oid::ObjectId};
use serde::{Serialize, de::DeserializeOwned};

use crate::error::{MongoError, MongoResult};

/// Extension trait for BSON documents.
pub trait DocumentExt {
    /// Get a string value from the document.
    fn get_str(&self, key: &str) -> MongoResult<&str>;

    /// Get an optional string value.
    fn get_str_opt(&self, key: &str) -> Option<&str>;

    /// Get an i32 value.
    fn get_i32(&self, key: &str) -> MongoResult<i32>;

    /// Get an optional i32 value.
    fn get_i32_opt(&self, key: &str) -> Option<i32>;

    /// Get an i64 value.
    fn get_i64(&self, key: &str) -> MongoResult<i64>;

    /// Get an optional i64 value.
    fn get_i64_opt(&self, key: &str) -> Option<i64>;

    /// Get a bool value.
    fn get_bool(&self, key: &str) -> MongoResult<bool>;

    /// Get an optional bool value.
    fn get_bool_opt(&self, key: &str) -> Option<bool>;

    /// Get an ObjectId value.
    fn get_object_id(&self, key: &str) -> MongoResult<ObjectId>;

    /// Get an optional ObjectId value.
    fn get_object_id_opt(&self, key: &str) -> Option<ObjectId>;

    /// Get a nested document.
    fn get_document(&self, key: &str) -> MongoResult<&Document>;

    /// Get an optional nested document.
    fn get_document_opt(&self, key: &str) -> Option<&Document>;

    /// Get an array value.
    fn get_array(&self, key: &str) -> MongoResult<&Vec<Bson>>;

    /// Get an optional array value.
    fn get_array_opt(&self, key: &str) -> Option<&Vec<Bson>>;

    /// Convert to a typed struct.
    fn to_struct<T: DeserializeOwned>(&self) -> MongoResult<T>;

    /// Get the `_id` field as ObjectId.
    fn id(&self) -> MongoResult<ObjectId>;
}

impl DocumentExt for Document {
    fn get_str(&self, key: &str) -> MongoResult<&str> {
        self.get_str(key)
            .map_err(|_| MongoError::query(format!("field '{}' is not a string", key)))
    }

    fn get_str_opt(&self, key: &str) -> Option<&str> {
        self.get_str(key).ok()
    }

    fn get_i32(&self, key: &str) -> MongoResult<i32> {
        self.get_i32(key)
            .map_err(|_| MongoError::query(format!("field '{}' is not an i32", key)))
    }

    fn get_i32_opt(&self, key: &str) -> Option<i32> {
        self.get_i32(key).ok()
    }

    fn get_i64(&self, key: &str) -> MongoResult<i64> {
        self.get_i64(key)
            .map_err(|_| MongoError::query(format!("field '{}' is not an i64", key)))
    }

    fn get_i64_opt(&self, key: &str) -> Option<i64> {
        self.get_i64(key).ok()
    }

    fn get_bool(&self, key: &str) -> MongoResult<bool> {
        self.get_bool(key)
            .map_err(|_| MongoError::query(format!("field '{}' is not a bool", key)))
    }

    fn get_bool_opt(&self, key: &str) -> Option<bool> {
        self.get_bool(key).ok()
    }

    fn get_object_id(&self, key: &str) -> MongoResult<ObjectId> {
        self.get_object_id(key)
            .map_err(|_| MongoError::query(format!("field '{}' is not an ObjectId", key)))
    }

    fn get_object_id_opt(&self, key: &str) -> Option<ObjectId> {
        self.get_object_id(key).ok()
    }

    fn get_document(&self, key: &str) -> MongoResult<&Document> {
        self.get_document(key)
            .map_err(|_| MongoError::query(format!("field '{}' is not a document", key)))
    }

    fn get_document_opt(&self, key: &str) -> Option<&Document> {
        self.get_document(key).ok()
    }

    fn get_array(&self, key: &str) -> MongoResult<&Vec<Bson>> {
        self.get_array(key)
            .map_err(|_| MongoError::query(format!("field '{}' is not an array", key)))
    }

    fn get_array_opt(&self, key: &str) -> Option<&Vec<Bson>> {
        self.get_array(key).ok()
    }

    fn to_struct<T: DeserializeOwned>(&self) -> MongoResult<T> {
        bson::from_document(self.clone()).map_err(|e| MongoError::serialization(e.to_string()))
    }

    fn id(&self) -> MongoResult<ObjectId> {
        self.get_object_id("_id")
            .map_err(|_| MongoError::query("field '_id' is not an ObjectId"))
    }
}

/// Convert a struct to a BSON document.
pub fn to_document<T: Serialize>(value: &T) -> MongoResult<Document> {
    bson::to_document(value).map_err(|e| MongoError::serialization(e.to_string()))
}

/// Convert a BSON document to a struct.
pub fn from_document<T: DeserializeOwned>(doc: Document) -> MongoResult<T> {
    bson::from_document(doc).map_err(|e| MongoError::serialization(e.to_string()))
}

/// Parse an ObjectId from a string.
pub fn parse_object_id(s: &str) -> MongoResult<ObjectId> {
    ObjectId::parse_str(s).map_err(MongoError::from)
}

/// Create a new ObjectId.
pub fn new_object_id() -> ObjectId {
    ObjectId::new()
}

/// BSON type helpers.
pub mod bson_types {
    use super::*;
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    /// Convert a UUID to BSON Binary.
    pub fn uuid_to_bson(uuid: Uuid) -> Bson {
        Bson::Binary(bson::Binary {
            subtype: bson::spec::BinarySubtype::Uuid,
            bytes: uuid.as_bytes().to_vec(),
        })
    }

    /// Convert BSON Binary to UUID.
    pub fn bson_to_uuid(bson: &Bson) -> MongoResult<Uuid> {
        match bson {
            Bson::Binary(binary) => {
                let bytes: [u8; 16] = binary
                    .bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| MongoError::serialization("invalid UUID bytes"))?;
                Ok(Uuid::from_bytes(bytes))
            }
            Bson::String(s) => Uuid::parse_str(s)
                .map_err(|e| MongoError::serialization(format!("invalid UUID string: {}", e))),
            _ => Err(MongoError::serialization(
                "expected Binary or String for UUID",
            )),
        }
    }

    /// Convert a DateTime to BSON DateTime.
    pub fn datetime_to_bson(dt: DateTime<Utc>) -> Bson {
        Bson::DateTime(bson::DateTime::from_chrono(dt))
    }

    /// Convert BSON DateTime to chrono DateTime.
    pub fn bson_to_datetime(bson: &Bson) -> MongoResult<DateTime<Utc>> {
        match bson {
            Bson::DateTime(dt) => Ok(dt.to_chrono()),
            _ => Err(MongoError::serialization("expected DateTime")),
        }
    }

    /// Get the Prax schema type for a BSON type.
    pub fn bson_type_to_prax(bson: &Bson) -> &'static str {
        match bson {
            Bson::Double(_) => "Float",
            Bson::String(_) => "String",
            Bson::Array(_) => "List",
            Bson::Document(_) => "Json",
            Bson::Boolean(_) => "Boolean",
            Bson::Null => "Null",
            Bson::Int32(_) => "Int",
            Bson::Int64(_) => "BigInt",
            Bson::DateTime(_) => "DateTime",
            Bson::Binary(b) if b.subtype == bson::spec::BinarySubtype::Uuid => "Uuid",
            Bson::Binary(_) => "Bytes",
            Bson::ObjectId(_) => "String", // ObjectId maps to String in Prax
            Bson::Decimal128(_) => "Decimal",
            _ => "Unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bson::doc;
    use uuid::Uuid;

    #[test]
    fn test_document_ext_get_str() {
        let doc = doc! { "name": "Alice", "age": 30 };
        assert_eq!(DocumentExt::get_str(&doc, "name").unwrap(), "Alice");
        assert!(DocumentExt::get_str(&doc, "age").is_err());
        assert!(DocumentExt::get_str(&doc, "missing").is_err());
    }

    #[test]
    fn test_document_ext_get_i32() {
        let doc = doc! { "count": 42, "name": "test" };
        assert_eq!(DocumentExt::get_i32(&doc, "count").unwrap(), 42);
        assert!(DocumentExt::get_i32(&doc, "name").is_err());
    }

    #[test]
    fn test_document_ext_get_object_id() {
        let oid = ObjectId::new();
        let doc = doc! { "_id": oid };
        assert_eq!(DocumentExt::get_object_id(&doc, "_id").unwrap(), oid);
    }

    #[test]
    fn test_to_document() {
        #[derive(Serialize)]
        struct User {
            name: String,
            age: i32,
        }

        let user = User {
            name: "Bob".to_string(),
            age: 25,
        };

        let doc = to_document(&user).unwrap();
        assert_eq!(doc.get_str("name").unwrap(), "Bob");
        assert_eq!(doc.get_i32("age").unwrap(), 25);
    }

    #[test]
    fn test_from_document() {
        #[derive(Debug, PartialEq, serde::Deserialize)]
        struct User {
            name: String,
            age: i32,
        }

        let doc = doc! { "name": "Carol", "age": 35 };
        let user: User = from_document(doc).unwrap();
        assert_eq!(
            user,
            User {
                name: "Carol".to_string(),
                age: 35
            }
        );
    }

    #[test]
    fn test_parse_object_id() {
        let oid = new_object_id();
        let parsed = parse_object_id(&oid.to_hex()).unwrap();
        assert_eq!(oid, parsed);

        assert!(parse_object_id("invalid").is_err());
    }

    #[test]
    fn test_uuid_conversion() {
        use bson_types::*;

        let uuid = Uuid::new_v4();
        let bson = uuid_to_bson(uuid);
        let parsed = bson_to_uuid(&bson).unwrap();
        assert_eq!(uuid, parsed);
    }
}
