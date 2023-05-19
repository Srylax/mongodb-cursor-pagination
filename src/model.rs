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
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub next_cursor: Option<Edge>,
    pub start_cursor: Option<Edge>,
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

    fn next_cursor(&self) -> Option<String> {
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

/// The direction of the list, ie. you are sending a cursor for the next or previous items. Defaults to Next
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CursorDirections {
    Previous,
    Next,
}
impl Default for CursorDirections {
    fn default() -> Self {
        Self::Next
    }
}
