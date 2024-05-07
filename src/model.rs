use std::fmt;
use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use bson::Document;
use serde::de::{self, Visitor};
use serde::{ser, Deserialize, Serialize};

use crate::option::CursorOptions;

/// Represents a Cursor to an Item with no special direction.
/// To Debug the contents, use `Debug`
/// When serializing or converting to String, the [`Edge`] gets encoded as url-safe Base64 String.
#[derive(Clone, Debug, PartialEq)]
pub struct Edge(Document);

impl Edge {
    /// Creates a new [`Edge`] using a value Document and the sorting keys.
    /// Only retains the values of the keys specified in the sort options to optimize storage.
    ///
    /// # Arguments
    /// * `document`: The Item to which the Edge will point to
    /// * `options`: Used to extract the sorting keys
    #[must_use]
    pub fn new(document: &Document, options: &CursorOptions) -> Self {
        let mut cursor = Document::new();
        options
            .sort
            .clone()
            .unwrap_or_default()
            .keys()
            .filter_map(|key| document.get(key).map(|value| (key, value)))
            .for_each(|(key, value)| {
                cursor.insert(key, value);
            });
        Self(cursor)
    }
}

impl Display for Edge {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "{}",
            &URL_SAFE_NO_PAD.encode(bson::to_vec(&self.0).map_err(ser::Error::custom)?)
        )
    }
}

impl Serialize for Edge {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(
            &URL_SAFE_NO_PAD.encode(bson::to_vec(&self.0).map_err(ser::Error::custom)?),
        )
    }
}

impl<'de> Deserialize<'de> for Edge {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Vis;
        impl Visitor<'_> for Vis {
            type Value = Edge;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a base64 string")
            }

            fn visit_str<E: de::Error>(self, str: &str) -> Result<Self::Value, E> {
                let doc = URL_SAFE_NO_PAD.decode(str).map_err(de::Error::custom)?;
                Ok(Edge(
                    bson::from_slice(doc.as_slice()).map_err(de::Error::custom)?,
                ))
            }
        }
        deserializer.deserialize_str(Vis)
    }
}

#[cfg(feature = "graphql")]
#[juniper::graphql_object]
#[allow(clippy::multiple_inherent_impl)]
impl Edge {
    fn cursor(&self) -> String {
        self.0.to_string()
    }
}

impl Deref for Edge {
    type Target = Document;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Edge {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Contains information about the current Page
/// The cursor have the respecting direction to be used as is in an subsequent find:
/// `start_cursor`: `Backwards`
/// `end_cursor`: `Forward`
///
/// Note: `has_xxx` means if the next page has items, not if there is a next cursor
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[non_exhaustive]
pub struct PageInfo {
    /// True if there is a previous page which contains items
    pub has_previous_page: bool,
    /// True if there is a next page which contains items
    pub has_next_page: bool,
    /// Cursor to the first item of the page. Is set even when there is no previous page.
    pub start_cursor: Option<DirectedCursor>,
    /// Cursor to the last item of the page. Is set even when there is no next page.
    pub end_cursor: Option<DirectedCursor>,
}

#[cfg(feature = "graphql")]
#[juniper::graphql_object]
impl PageInfo {
    fn has_next_page(&self) -> bool {
        self.has_next_page
    }

    fn has_previous_page(&self) -> bool {
        self.has_previous_page
    }

    fn start_cursor(&self) -> Option<String> {
        self.start_cursor.as_ref().map(ToString::to_string)
    }

    fn end_cursor(&self) -> Option<String> {
        self.end_cursor.as_ref().map(ToString::to_string)
    }
}

/// The result of a find method with the items, edges, pagination info, and total count of objects
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct FindResult<T> {
    /// Current Page
    pub page_info: PageInfo,
    /// Edges to all items in the current Page, including start & end-cursor
    pub edges: Vec<Edge>,
    /// Total count of items in the whole collection
    pub total_count: u64,
    /// All items in the current Page
    pub items: Vec<T>,
}

/// Cursor to an item with direction information.
/// Serializing pertains the direction Information.
/// To send only the Cursor use `to_string` which drops the direction information
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(clippy::exhaustive_enums)] // If there would ever be more Variants we would want the Code to break
pub enum DirectedCursor {
    /// Use to invert the search e.g. go back a page
    Backwards(Edge),
    /// Normal direction to search
    Forward(Edge),
}

impl DirectedCursor {
    /// Reverses the direction of Cursor.
    #[must_use]
    pub fn reverse(self) -> Self {
        match self {
            Self::Backwards(edge) => Self::Forward(edge),
            Self::Forward(edge) => Self::Backwards(edge),
        }
    }
    /// Returns a reference to the inner of this [`DirectedCursor`].
    #[must_use]
    pub const fn inner(&self) -> &Edge {
        match self {
            Self::Forward(edge) | Self::Backwards(edge) => edge,
        }
    }

    /// Removes the direction information and returns an Edge
    #[must_use]
    pub fn into_inner(self) -> Edge {
        match self {
            Self::Forward(edge) | Self::Backwards(edge) => edge,
        }
    }
}

impl Display for DirectedCursor {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}", self.inner())
    }
}
