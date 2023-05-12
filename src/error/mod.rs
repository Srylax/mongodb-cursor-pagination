use base64::DecodeError;
use thiserror::Error;

/// Possible errors that can arise during parsing and creating a cursor.
#[derive(Debug, Error)]
pub enum CursorError {
    #[error("Unable to decode Cursor: {0}")]
    DecodeError(#[from] DecodeError),
    #[error("Unable to parse str to ObjectID: {0}")]
    ParseError(#[from] bson::oid::Error),
}
