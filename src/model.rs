use std::ops::{Deref, DerefMut};

use bson::Document;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edge(Document);

impl Edge {
    pub(crate) fn new(doc: Document) -> Self {
        Self(doc)
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
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub next_cursor: Option<Edge>,
    pub start_cursor: Option<Edge>,
}

/// The result of a find method with the items, edges, pagination info, and total count of objects
#[derive(Debug, Default)]
pub struct FindResult<T> {
    pub page_info: PageInfo,
    pub edges: Vec<Edge>,
    pub total_count: u64,
    pub items: Vec<T>,
}
