#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::checked_conversions,
    clippy::implicit_saturating_sub,
    clippy::integer_arithmetic,
    clippy::mod_module_files,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used,
    missing_docs,
    rust_2018_idioms,
    unused_lifetimes,
    unused_qualifications
)]

//! ### Usage:
//! The usage is a bit different than the node version. See the examples for more details and a working example.
//! ```rust
//! use mongodb::{options::FindOptions, Client};
//! use mongodb_cursor_pagination::{CursorDirections, FindResult, PaginatedCursor};
//! use bson::doc;
//! use serde::Deserialize;
//!
//! // Note that your data structure must derive Deserialize
//! #[derive(Debug, Deserialize, PartialEq, Clone)]
//! pub struct MyFruit {
//!     name: String,
//!     how_many: i32,
//! }
//! #  impl MyFruit {
//! #     #[must_use]
//! #     pub fn new(name: impl Into<String>, how_many: i32) -> Self {
//! #         Self {
//! #             name: name.into(),
//! #             how_many,
//! #         }
//! #     }
//! # }
//!
//! #[tokio::main]
//! async fn main() {
//!     let client = Client::with_uri_str("mongodb://localhost:27017/")
//!         .await
//!         .expect("Failed to initialize client.");
//!     let db = client.database("mongodb_cursor_pagination");
//!   #  db.collection::<MyFruit>("myfruits")
//!   #      .drop(None)
//!   #      .await
//!   #      .expect("Failed to drop table");
//!
//!     let docs = vec![
//!         doc! { "name": "Apple", "how_many": 5 },
//!         doc! { "name": "Orange", "how_many": 3 },
//!         doc! { "name": "Blueberry", "how_many": 25 },
//!         doc! { "name": "Bananas", "how_many": 8 },
//!         doc! { "name": "Grapes", "how_many": 12 },
//!     ];
//!
//!     db.collection("myfruits")
//!         .insert_many(docs, None)
//!         .await
//!         .expect("Unable to insert data");
//!
//!     // query page 1, 2 at a time
//!     let options = FindOptions::builder()
//!             .limit(2)
//!             .sort(doc! { "name": 1 })
//!             .build();
//!
//!     let mut find_results: FindResult<MyFruit> = PaginatedCursor::new(Some(options.clone()), None, None)
//!         .find(&db.collection("myfruits"), None)
//!         .await
//!         .expect("Unable to find data");
//!   #  assert_eq!(
//!   #     find_results.items,
//!   #     vec![MyFruit::new("Apple", 5), MyFruit::new("Bananas", 8),]
//!   # );
//!     println!("First page: {:?}", find_results);
//!
//!     // get the second page
//!     let mut cursor = find_results.page_info.next_cursor;
//!     find_results = PaginatedCursor::new(Some(options), cursor, Some(CursorDirections::Next))
//!         .find(&db.collection("myfruits"), None)
//!         .await
//!         .expect("Unable to find data");
//!   #  assert_eq!(
//!   #    find_results.items,
//!   #     vec![MyFruit::new("Blueberry", 25), MyFruit::new("Grapes", 12),]
//!   # );
//!     println!("Second page: {:?}", find_results);
//! }
//! ```
//!
//! ### Response
//! The response `FindResult<T>` contains page info, cursors and edges (cursors for all of the items in the response).
//! ```rust
//! pub struct PageInfo {
//!     pub has_next_page: bool,
//!     pub has_previous_page: bool,
//!     pub start_cursor: Option<String>,
//!     pub next_cursor: Option<String>,
//! }
//!
//! pub struct Edge {
//!     pub cursor: String,
//! }
//!
//! pub struct FindResult<T> {
//!     pub page_info: PageInfo,
//!     pub edges: Vec<Edge>,
//!     pub total_count: i64,
//!     pub items: Vec<T>,
//! }
//! ```
//!
//! ## Features
//! It has support for graphql (using [juniper](https://github.com/graphql-rust/juniper)) if you enable the `graphql` flag. You can use it by just including the `PageInfo` into your code.
//!
//! ```ignore
//! use mongodb_cursor_pagination::{PageInfo, Edge};
//!
//! #[derive(Serialize, Deserialize)]
//! struct MyDataConnection {
//!     page_info: PageInfo,
//!     edges: Vec<Edge>,
//!     data: Vec<MyData>,
//!     total_count: i64,
//! }
//!
//! [juniper::object]
//! impl MyDataConnection {
//!     fn page_info(&self) -> &PageInfo {
//!         self.page_info
//!     }
//!
//!     fn edges(&self) -> &Vec<Edge> {
//!         &self.edges
//!     }
//! }
//! ```

pub mod error;
mod model;
mod options;

pub use crate::model::*;

use crate::options::CursorOptions;
use bson::{doc, Bson, Document};
use error::CursorError;
use futures_util::stream::StreamExt;
use log::warn;
use mongodb::options::CountOptions;
use mongodb::{options::FindOptions, Collection};
use serde::de::DeserializeOwned;
use std::ops::Neg;

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

#[cfg(feature = "graphql")]
#[juniper::object]
impl Edge {
    fn cursor(&self) -> String {
        self.cursor.to_owned()
    }
}
// FIX: there's probably a better way to do this...but for now
#[cfg(feature = "graphql")]
impl From<&Edge> for Edge {
    fn from(edge: &Edge) -> Edge {
        Edge {
            cursor: edge.cursor.clone(),
        }
    }
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

/// The main entry point for finding documents
#[derive(Debug)]
pub struct PaginatedCursor {
    has_cursor: bool,
    cursor: Option<Edge>,
    direction: CursorDirections,
    options: CursorOptions,
}

impl PaginatedCursor {
    /// Updates or creates all of the find options to help with pagination and returns a `PaginatedCursor` object.
    ///
    /// # Arguments
    /// * `options` - Optional find options that you would like to perform any searches with
    /// * `cursor` - An optional existing cursor in base64. This would have come from a previous `FindResult<T>`
    /// * `direction` - Determines whether the cursor supplied is for a previous page or the next page. Defaults to Next
    ///
    #[must_use]
    pub fn new(
        options: Option<FindOptions>,
        cursor: Option<Edge>,
        direction: Option<CursorDirections>,
    ) -> Self {
        Self {
            // parse base64 for keys
            has_cursor: cursor.is_some(),
            cursor,
            direction: direction.unwrap_or_default(),
            options: CursorOptions::from(options.unwrap_or_default()),
        }
    }

    /// Gets the number of documents matching filter.
    /// Note that using [`Collection::estimated_document_count`](#method.estimated_document_count)
    /// is recommended instead of this method is most cases.
    pub async fn count_documents<T>(
        &self,
        collection: &Collection<T>,
        query: Option<&Document>,
    ) -> Result<u64, CursorError> {
        let mut count_options = self.options.clone();
        count_options.limit = None;
        count_options.skip = None;
        let count_query = query.map_or_else(Document::new, Clone::clone);
        let total_count = collection
            .count_documents(count_query, Some(CountOptions::from(count_options)))
            .await?;
        Ok(total_count)
    }

    /// Finds the documents in the `collection` matching `filter`.
    pub async fn find<T>(
        &self,
        collection: &Collection<Document>,
        filter: Option<&Document>,
    ) -> Result<FindResult<T>, CursorError>
    where
        T: DeserializeOwned + Sync + Send + Unpin + Clone,
    {
        // first count the docs
        let total_count = self.count_documents(collection, filter).await?;

        // setup defaults
        let mut items: Vec<T> = vec![];
        let mut edges: Vec<Edge> = vec![];
        let mut has_skip = false;
        let has_next_page;
        let has_previous_page;
        let mut start_cursor: Option<Edge> = None;
        let mut next_cursor: Option<Edge> = None;

        // return if we if have no docs
        if total_count == 0 {
            return Ok(FindResult {
                page_info: PageInfo::default(),
                edges: vec![],
                total_count: 0,
                items: vec![],
            });
        }

        // build the cursor
        let query_doc = self.get_query(filter.cloned())?;
        let mut options = self.options.clone();
        let skip_value = options.skip.unwrap_or(0);
        if self.has_cursor || skip_value == 0 {
            options.skip = None;
        } else {
            has_skip = true;
        }
        // let has_previous
        let is_previous_query = self.has_cursor && self.direction == CursorDirections::Previous;
        // if it's a previous query we need to reverse the sort we were doing
        if is_previous_query {
            if let Some(sort) = options.sort.as_mut() {
                sort.iter_mut().for_each(|(_key, value)| {
                    if let Bson::Int32(num) = value {
                        *value = Bson::Int32(num.neg());
                    }
                    if let Bson::Int64(num) = value {
                        *value = Bson::Int64(num.neg());
                    }
                });
            }
        }
        let mut cursor = collection.find(query_doc, Some(options.into())).await?;
        while let Some(result) = cursor.next().await {
            match result {
                Ok(doc) => {
                    let item = bson::from_bson(Bson::Document(doc.clone()))?;
                    edges.push(Edge::new(doc));
                    items.push(item);
                }
                Err(error) => {
                    warn!("Error to find doc: {}", error);
                }
            }
        }
        let has_more: bool;
        if has_skip {
            has_more = (items.len() as u64).saturating_add(skip_value) < total_count;
            has_previous_page = true;
            has_next_page = has_more;
        } else {
            has_more = items.len() as i64 > self.options.limit.unwrap_or(0).saturating_sub(1);
            has_previous_page = (self.has_cursor && self.direction == CursorDirections::Next)
                || (is_previous_query && has_more);
            has_next_page = (self.direction == CursorDirections::Next && has_more)
                || (is_previous_query && self.has_cursor);
        }

        // reorder if we are going backwards
        if is_previous_query {
            items.reverse();
            edges.reverse();
        }
        // remove the extra item to check if we have more
        if has_more && !is_previous_query {
            items.pop();
            edges.pop();
        } else if has_more {
            items.remove(0);
            edges.remove(0);
        }

        // create the next cursor
        if !items.is_empty() && edges.len() == items.len() {
            start_cursor = Some(edges[0].clone());
            next_cursor = Some(edges[items.len().saturating_sub(1)].clone());
        }

        let page_info = PageInfo {
            has_next_page,
            has_previous_page,
            next_cursor,
            start_cursor,
        };

        Ok(FindResult {
            page_info,
            edges,
            total_count,
            items,
        })
    }

    /*
    $or: [{
        launchDate: { $lt: nextLaunchDate }
    }, {
        // If the launchDate is an exact match, we need a tiebreaker, so we use the _id field from the cursor.
        launchDate: nextLaunchDate,
    _id: { $lt: nextId }
    }]
    */
    fn get_query(&self, query: Option<Document>) -> Result<Document, CursorError> {
        // now create the filter
        let mut query_doc = query.unwrap_or_default();

        // Don't do anything if no cursor is provided
        let Some(cursor) = &self.cursor else {
            return Ok(query_doc);
        };
        let Some(sort) = &self.options.sort else {
            return Ok(query_doc);
        };

        // this is the simplest form, it's just a sort by _id
        if sort.len() <= 1 {
            let object_id = cursor.get("_id").ok_or(CursorError::InvalidCursor)?.clone();
            let direction = self.get_direction_from_key(sort, "_id");
            query_doc.insert("_id", doc! { direction: object_id });
            return Ok(query_doc);
        }

        let mut queries: Vec<Document> = Vec::new();
        let mut previous_conditions: Vec<(String, Bson)> = Vec::new();

        // Add each sort condition with it's direction and all previous condition with fixed values
        for key in sort.keys() {
            let mut query = query_doc.clone();
            query.extend(previous_conditions.clone().into_iter()); // Add previous conditions

            let value = cursor.get(key).unwrap_or(&Bson::Null);

            let direction = self.get_direction_from_key(sort, key);
            query.insert(key, doc! { direction: value.clone() });
            previous_conditions.push((key.clone(), value.clone())); // Add self without direction to previous conditions

            queries.push(query);
        }

        query_doc = if queries.len() > 1 {
            doc! { "$or": queries.iter().as_ref() }
        } else {
            queries.pop().unwrap_or_default()
        };
        Ok(query_doc)
    }

    fn get_direction_from_key(&self, sort: &Document, key: &str) -> &'static str {
        let value = sort.get(key).and_then(Bson::as_i32).unwrap_or(0);
        match self.direction {
            CursorDirections::Next => {
                if value >= 0 {
                    "$gt"
                } else {
                    "$lt"
                }
            }
            CursorDirections::Previous => {
                if value >= 0 {
                    "$lt"
                } else {
                    "$gt"
                }
            }
        }
    }
}
