use std::{error, fmt, io, str::Utf8Error};

/// Possible errors that can arise during parsing and creating a cursor.
#[derive(Debug)]
pub enum CursorError {
    IoError(io::Error),
    InvalidCursor(String),
    Unknown(String),
    InvalidId(String),
}

impl From<io::Error> for CursorError {
    fn from(err: io::Error) -> CursorError {
        CursorError::IoError(err)
    }
}

impl From<Utf8Error> for CursorError {
    fn from(err: Utf8Error) -> CursorError {
        CursorError::InvalidCursor(err.to_string())
    }
}

impl From<base64::DecodeError> for CursorError {
    fn from(err: base64::DecodeError) -> CursorError {
        CursorError::InvalidCursor(err.to_string())
    }
}

impl fmt::Display for CursorError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CursorError::IoError(ref inner) => inner.fmt(fmt),
            CursorError::InvalidCursor(ref cursor) => {
                write!(fmt, "Invalid cursor - unable to parse: {:?}", cursor)
            }
            CursorError::Unknown(ref inner) => inner.fmt(fmt),
            CursorError::InvalidId(ref id) => write!(fmt, "Invalid id - {:?}", id),
        }
    }
}

#[allow(deprecated)]
impl error::Error for CursorError {
    fn description(&self) -> &str {
        match *self {
            CursorError::IoError(ref inner) => inner.description(),
            CursorError::InvalidCursor(_) => "Invalid cursor value",
            CursorError::Unknown(ref inner) => inner,
            CursorError::InvalidId(_) => "Invalid mongodbid",
        }
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            CursorError::IoError(ref inner) => Some(inner),
            _ => None,
        }
    }
}
