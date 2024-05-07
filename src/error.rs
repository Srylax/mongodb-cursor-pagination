#![allow(clippy::absolute_paths)]
use thiserror::Error;

/// Possible errors that can arise during parsing and creating a cursor.
#[derive(Debug, Error)]
pub enum CursorError {
    #[error("Unable to deserialize to bson: {0}")]
    BsonDeError(#[from] bson::de::Error),
    #[error("Unable to deserialize to bson: {0}")]
    BsonSerError(#[from] bson::ser::Error),
    #[error("Error while accessing Value {0}")]
    BsonValueAccessError(#[from] bson::document::ValueAccessError),
    #[error("Unable to parse str to ObjectID: {0}")]
    ParseError(#[from] bson::oid::Error),
    #[error("Error while retrieving data: {0}")]
    MongoDBError(#[from] mongodb::error::Error),
    #[error("Invalid cursor")]
    InvalidCursor,
}
