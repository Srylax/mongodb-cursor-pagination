#![doc = include_str!("../README.md")]

//! ### Usage:
//! The usage is a bit different than the node version. See the examples for more details and a working example.
//! ```rust
//! use mongodb::{options::FindOptions, Client};
//! use mongodb_cursor_pagination::{FindResult, Pagination};
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
//!     let fruits = db.collection::<MyFruit>("myfruits");
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
//!     let mut find_results: FindResult<MyFruit> = fruits
//!         .find_paginated(None, Some(options.clone()), None)
//!         .await
//!         .expect("Unable to find data");
//!   #  assert_eq!(
//!   #     find_results.items,
//!   #     vec![MyFruit::new("Apple", 5), MyFruit::new("Bananas", 8),]
//!   # );
//!     println!("First page: {:?}", find_results);
//!
//!     // get the second page
//!     let mut cursor = find_results.page_info.end_cursor;
//!     find_results = fruits
//!         .find_paginated(None, Some(options), cursor)
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

mod error;
mod model;
mod option;
pub use model::*;

use crate::option::CursorOptions;
use bson::{doc, Bson, Document};
use error::CursorError;
use futures_util::stream::StreamExt;
use futures_util::TryStreamExt;
use mongodb::options::CountOptions;
use mongodb::{options::FindOptions, Collection};
use serde::de::DeserializeOwned;

use async_trait::async_trait;

#[async_trait]
/// Used to paginate through a collection.
pub trait Pagination {
    /// Finds the items in the collection matching `filter` based on the `cursor`.
    ///
    /// # Arguments
    /// * `filter`: Optional filter to restrict the result set of the query.
    /// * `options`: Optional find options that you would like to perform any searches with
    /// * `cursor`: An optional existing cursor in base64. This would have come from a previous `FindResult<T>`
    async fn find_paginated<T>(
        &self,
        filter: Option<Document>,
        options: Option<FindOptions>,
        cursor: Option<DirectedCursor>,
    ) -> Result<FindResult<T>, CursorError>
    where
        T: DeserializeOwned + Send;
}

#[async_trait]
impl<I: Send + Sync> Pagination for Collection<I> {
    async fn find_paginated<T>(
        &self,
        filter: Option<Document>,
        options: Option<FindOptions>,
        cursor: Option<DirectedCursor>,
    ) -> Result<FindResult<T>, CursorError>
    where
        T: DeserializeOwned + Send,
    {
        let options = CursorOptions::new(options.unwrap_or_default(), cursor.clone());

        let filter = filter.unwrap_or_default();

        let query = get_query(filter.clone(), &options, cursor.as_ref())?;

        let mut documents = self
            .clone_with_type::<Document>()
            .find(query.clone(), Some(options.clone().into()))
            .await?
            .try_collect::<Vec<Document>>()
            .await?;

        if matches!(cursor, Some(DirectedCursor::Backwards(_))) {
            documents.reverse();
        }

        let items = documents
            .clone()
            .into_iter()
            .map(|doc| bson::from_bson(Bson::Document(doc)))
            .collect::<Result<Vec<T>, _>>()?;

        let edges = documents
            .clone()
            .into_iter()
            .map(|doc| Edge::new(&doc, &options))
            .collect::<Vec<Edge>>();

        let end_cursor = edges.last().cloned().map(DirectedCursor::Forward);
        let start_cursor = edges.first().cloned().map(DirectedCursor::Backwards);

        let has_next_page = has_page(
            &self.clone_with_type(),
            filter.clone(),
            options.clone(),
            end_cursor.as_ref(),
        )
        .await?;

        let has_previous_page = has_page(
            &self.clone_with_type(),
            filter.clone(),
            options.clone(),
            start_cursor.as_ref(),
        )
        .await?;

        let page_info = PageInfo {
            has_previous_page,
            has_next_page,
            start_cursor,
            end_cursor,
        };

        Ok(FindResult {
            page_info,
            edges,
            total_count: count_documents(options.clone().into(), self, Some(&filter)).await?,
            items,
        })
    }
}

async fn count_documents<T: Sync>(
    mut options: CountOptions,
    collection: &Collection<T>,
    filter: Option<&Document>,
) -> Result<u64, CursorError> {
    options.limit = None;
    options.skip = None;
    let count_query = filter.map_or_else(Document::new, Clone::clone);
    Ok(collection
        .count_documents(count_query, Some(options))
        .await?)
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
fn get_query(
    mut filter: Document,
    options: &CursorOptions,
    cursor: Option<&DirectedCursor>,
) -> Result<Document, CursorError> {
    let Some(cursor) = cursor else {
        return Ok(filter);
    };

    let Some(sort) = options.sort.clone() else {
        return Ok(filter);
    };

    // this is the simplest form, it's just a sort by _id
    if sort.len() <= 1 {
        let object_id = cursor
            .inner()
            .get("_id")
            .ok_or(CursorError::InvalidCursor)?
            .clone();
        let direction = if sort.get_i32("_id")? >= 0_i32 {
            "$gt"
        } else {
            "$lt"
        };
        filter.insert("_id", doc! { direction: object_id });
        return Ok(filter);
    }

    let mut queries: Vec<Document> = Vec::new();
    let mut previous_conditions: Vec<(String, Bson)> = Vec::new();

    // Add each sort condition with it's direction and all previous condition with fixed values
    for key in sort.keys() {
        let mut query = filter.clone();
        query.extend(previous_conditions.clone().into_iter()); // Add previous conditions

        let value = cursor.inner().get(key).unwrap_or(&Bson::Null);

        let direction = if sort.get_i32(key)? >= 0_i32 {
            "$gt"
        } else {
            "$lt"
        };

        query.insert(key, doc! { direction: value.clone() });
        previous_conditions.push((key.clone(), value.clone())); // Add self without direction to previous conditions

        queries.push(query);
    }

    filter = if queries.len() > 1 {
        doc! { "$or": queries.iter().as_ref() }
    } else {
        queries.pop().unwrap_or_default()
    };
    Ok(filter)
}

async fn has_page(
    collection: &Collection<Document>,
    filter: Document,
    mut options: CursorOptions,
    cursor: Option<&DirectedCursor>,
) -> Result<bool, CursorError> {
    let Some(cursor) = cursor else {
        return Ok(false);
    };

    options.set_cursor(cursor.clone());
    options.skip = None;
    let filter = get_query(filter, &options, Some(cursor))?;

    Ok(collection
        .find(Some(filter), Some(options.into()))
        .await?
        .next()
        .await
        .transpose()?
        .is_some())
}
