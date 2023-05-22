use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use bson::Document;
use serde::{Deserialize, Serialize};

use crate::option::CursorOptions;

#[derive(Clone, Debug, PartialEq)]
pub struct Edge(Document);

impl Edge {
    pub fn new(document: Document, options: &CursorOptions) -> Self {
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            &URL_SAFE_NO_PAD.encode(bson::to_vec(&self.0).map_err(serde::ser::Error::custom)?)
        )
    }
}

impl Serialize for Edge {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(
            &URL_SAFE_NO_PAD.encode(bson::to_vec(&self.0).map_err(serde::ser::Error::custom)?),
        )
    }
}

impl<'de> Deserialize<'de> for Edge {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Vis;
        impl serde::de::Visitor<'_> for Vis {
            type Value = Edge;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a base64 string")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                let doc = URL_SAFE_NO_PAD
                    .decode(v)
                    .map_err(serde::de::Error::custom)?;
                Ok(Edge(
                    bson::from_slice(doc.as_slice()).map_err(serde::de::Error::custom)?,
                ))
            }
        }
        deserializer.deserialize_str(Vis)
    }
}

#[cfg(feature = "graphql")]
#[juniper::object]
impl Edge {
    fn cursor(&self) -> String {
        self.cursor.to_owned()
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
/// Note: has_xxx means if the next page has items, not if there is a next cursor
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct PageInfo {
    pub has_previous_page: bool,
    pub has_next_page: bool,
    pub start_cursor: Option<DirectedCursor>,
    pub end_cursor: Option<DirectedCursor>,
}

#[cfg(feature = "graphql")]
#[juniper::object]
impl PageInfo {
    fn has_next_page(&self) -> bool {
        self.has_next_page
    }

    fn has_previous_page(&self) -> bool {
        self.has_previous_page
    }

    fn start_cursor(&self) -> Option<String> {
        self.start_cursor.to_owned()
    }

    fn end_cursor(&self) -> Option<String> {
        self.next_cursor.to_owned()
    }
}

/// The result of a find method with the items, edges, pagination info, and total count of objects
#[derive(Debug, Default)]
pub struct FindResult<T> {
    pub page_info: PageInfo,
    pub edges: Vec<Edge>,
    pub total_count: u64,
    pub items: Vec<T>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DirectedCursor {
    Backwards(Edge),
    Forward(Edge),
}

impl DirectedCursor {
    pub fn reverse(self) -> Self {
        match self {
            Self::Backwards(edge) => Self::Forward(edge),
            Self::Forward(edge) => Self::Backwards(edge),
        }
    }
    pub fn inner(&self) -> &Edge {
        match self {
            Self::Backwards(edge) => edge,
            Self::Forward(edge) => edge,
        }
    }

    pub fn into_inner(self) -> Edge {
        match self {
            Self::Backwards(edge) => edge,
            Self::Forward(edge) => edge,
        }
    }
}

impl Display for DirectedCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner())
    }
}
