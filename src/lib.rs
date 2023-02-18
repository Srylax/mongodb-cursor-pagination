//! Based on the [node module](https://github.com/mixmaxhq/mongo-cursor-pagination) but for Rust.
//! You can read more about it on their [blog post](https://engineering.mixmax.com/blog/api-paging-built-the-right-way/) and why it seems necessary.
//!
//! So far it only supports count and find. Search and aggregation will come when needed.
//!
//! The usage is a bit different than the node version. See the examples for more details.

pub mod error;
mod options;

use bson::{doc, oid::ObjectId, Bson, Document};
use error::CursorError;
use log::warn;
use mongodb::{options::FindOptions, Collection};
use options::CursorOptions;
use serde::{Deserialize, Serialize};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::de::DeserializeOwned;
use futures_util::stream::StreamExt;

/// Provides details about if there are more pages and the cursor to the start of the list and end
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: Option<String>,
    pub next_cursor: Option<String>,
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

/// Edges are the cursors on all of the items in the return
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Edge {
    pub cursor: String,
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

/// The result of a find method with the items, edges, pagination info, and total count of objects
#[derive(Debug)]
pub struct FindResult<T> {
    pub page_info: PageInfo,
    pub edges: Vec<Edge>,
    pub total_count: u64,
    pub items: Vec<T>,
}

/// The direction of the list, ie. you are sending a cursor for the next or previous items. Defaults to Next
#[derive(Clone, Debug, PartialEq)]
pub enum CursorDirections {
    Previous,
    Next,
}

/// The main entry point for finding documents
#[derive(Debug)]
pub struct PaginatedCursor {
    has_cursor: bool,
    cursor_doc: Document,
    direction: CursorDirections,
    options: CursorOptions,
}

impl<'a> PaginatedCursor {
    /// Updates or creates all of the find options to help with pagination and returns a PaginatedCursor object.
    ///
    /// # Arguments
    /// * `options` - Optional find options that you would like to perform any searches with
    /// * `cursor` - An optional existing cursor in base64. This would have come from a previous FindResult<T>
    /// * `direction` - Determines whether the cursor supplied is for a previous page or the next page. Defaults to Next
    ///
    pub fn new(
        options: Option<FindOptions>,
        cursor: Option<String>,
        direction: Option<CursorDirections>,
    ) -> Self {
        PaginatedCursor {
            // parse base64 for keys
            has_cursor: cursor.is_some(),
            cursor_doc: if let Some(b64) = cursor {
                map_from_base64(b64).expect("Unable to parse cursor")
            } else {
                Document::new()
            },
            direction: if let Some(d) = direction {
                d
            } else {
                CursorDirections::Next
            },
            options: CursorOptions::from(options),
        }
    }

    /// Estimates the number of documents in the collection using collection metadata.
    pub async fn estimated_document_count<T>(&self, collection: &Collection<T>) -> Result<u64, CursorError> {
        let count_options = self.options.clone();
        let total_count = collection.estimated_document_count(count_options).await.unwrap();
        Ok(total_count)
    }

    /// Gets the number of documents matching filter.
    /// Note that using [`PaginatedCursor::estimated_document_count`](#method.estimated_document_count)
    /// is recommended instead of this method is most cases.
    pub async fn count_documents<T>(
        &self,
        collection: &Collection<T>,
        query: Option<&Document>,
    ) -> Result<u64, CursorError> {
        let mut count_options = self.options.clone();
        count_options.limit = None;
        count_options.skip = None;
        let count_query = if let Some(q) = query {
            q.clone()
        } else {
            Document::new()
        };
        let total_count = collection
            .count_documents(count_query, count_options)
            .await
            .unwrap();
        Ok(total_count)
    }

    /// Finds the documents in the `collection` matching `filter`.
    pub async fn find<T>(
        &self,
        collection: &Collection<Document>,
        filter: Option<&Document>,
    ) -> Result<FindResult<T>, CursorError>
    where
        T: DeserializeOwned + Sync + Send + Unpin + Clone
    {
        // first count the docs
        let total_count = self.count_documents(collection, filter).await.unwrap();

        // setup defaults
        let mut items: Vec<T> = vec![];
        let mut edges: Vec<Edge> = vec![];
        let mut has_next_page = false;
        let mut has_previous_page = false;
        let mut has_skip = false;
        let mut start_cursor: Option<String> = None;
        let mut next_cursor: Option<String> = None;

        // make the query if we have some docs
        if total_count > 0 {
            // build the cursor
            let query_doc = self.get_query(filter)?;
            let mut options = self.options.clone();
            let skip_value = options.skip.unwrap_or_else(|| 0);
            if self.has_cursor || skip_value == 0 {
                options.skip = None;
            } else {
                has_skip = true;
            }
            // let has_previous
            let is_previous_query = self.has_cursor && self.direction == CursorDirections::Previous;
            // if it's a previous query we need to reverse the sort we were doing
            if is_previous_query {
                if let Some(sort) = options.sort {
                    let keys: Vec<&String> = sort.keys().collect();
                    let mut new_sort = Document::new();
                    for key in keys {
                        let bson_value = sort.get(key).unwrap();
                        match bson_value {
                            Bson::Int32(value) => {
                                new_sort.insert(key, Bson::Int32(-*value));
                            }
                            Bson::Int64(value) => {
                                new_sort.insert(key, Bson::Int64(-*value));
                            }
                            _ => {}
                        };
                    }
                    options.sort = Some(new_sort);
                }
            }
            let mut cursor = collection.find(query_doc, options).await.unwrap();
            while let Some(result) = cursor.next().await {
                match result {
                    Ok(doc) => {
                        let item = bson::from_bson(bson::Bson::Document(doc.clone())).unwrap();
                        edges.push(Edge {
                            cursor: self.create_from_doc(&doc),
                        });
                        items.push(item);
                    }
                    Err(error) => {
                        warn!("Error to find doc: {}", error);
                    }
                }
            }
            let has_more: bool;
            if has_skip {
                has_more = (items.len() as u64 + skip_value) < total_count;
                has_previous_page = true;
                has_next_page = has_more;
            } else {
                has_more = items.len() > (self.options.limit.unwrap() - 1) as usize;
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
                start_cursor = Some(edges[0].cursor.to_owned());
                next_cursor = Some(edges[items.len() - 1].cursor.to_owned());
            }
        }

        let page_info = PageInfo {
            has_next_page,
            has_previous_page,
            start_cursor,
            next_cursor,
        };
        Ok(FindResult {
            page_info,
            total_count,
            edges,
            items,
        })
    }

    fn get_value_from_doc(&self, key: &str, doc: Bson) -> Option<(String, Bson)> {
        let parts: Vec<&str> = key.splitn(2, ".").collect();
        match doc {
            Bson::Document(d) => {
                let some_value = d.get(parts[0]);
                match some_value {
                    Some(value) =>
                        match value {
                            Bson::Document(d) => {
                                self.get_value_from_doc(parts[1], Bson::Document(d.clone()))
                            }
                            _ => Some((parts[0].to_string(), value.clone())),
                        },
                    None => None
                }
            }
            _ => Some((parts[0].to_string(), doc)),
        }
    }

    fn create_from_doc(&self, doc: &Document) -> String {
        let mut only_sort_keys = Document::new();
        if let Some(sort) = &self.options.sort {
            for key in sort.keys() {
                if let Some((_, value)) = self.get_value_from_doc(key, Bson::Document(doc.clone())) {
                    only_sort_keys.insert(key, value);
                }
            }
            let buf = bson::to_vec(&only_sort_keys).unwrap();
            STANDARD.encode(buf)
        } else {
            "".to_owned()
        }
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
    fn get_query(&self, query: Option<&Document>) -> Result<Document, CursorError> {
        // now create the filter
        let mut query_doc = match query {
            Some(doc) => doc.clone(),
            None => Document::new(),
        };

        if self.cursor_doc.is_empty() {
            return Ok(query_doc);
        } else if let Some(sort) = &self.options.sort {
            if sort.len() > 1 {
                let keys: Vec<&String> = sort.keys().collect();
                let mut queries: Vec<Document> = Vec::new();
                #[allow(clippy::needless_range_loop)]
                for i in 0..keys.len() {
                    let mut query = query_doc.clone();
                    #[allow(clippy::needless_range_loop)]
                    for j in 0..i {
                        let value = self.cursor_doc.get(keys[j]).unwrap_or(&Bson::Null);
                        query.insert(keys[j], value.clone());
                    }
                    // insert the directional sort (ie. < or >)
                    let value = self.cursor_doc.get(keys[i]).unwrap_or(&Bson::Null);
                    let direction = self.get_direction_from_key(&sort, keys[i]);
                    query.insert(keys[i], doc! { direction: value.clone() });
                    queries.push(query);
                }
                if queries.len() > 1 {
                    query_doc = doc! { "$or": [] };
                    let or_array = query_doc.get_array_mut("$or").map_err(|_| CursorError::Unknown("Unable to process".into()))?;
                    for d in queries.iter() {
                        or_array.push(Bson::Document(d.clone()));
                    }
                } else {
                    query_doc = queries[0].clone();
                }
            } else {
                // this is the simplest form, it's just a sort by _id
                let object_id = self.cursor_doc.get("_id").unwrap().clone();
                let direction = self.get_direction_from_key(&sort, "_id");
                query_doc.insert("_id", doc! { direction: object_id });
            }
        }
        Ok(query_doc)
    }

    fn get_direction_from_key(&self, sort: &Document, key: &str) -> &'static str {
        let value = sort.get(key).unwrap().as_i32().unwrap();
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

fn map_from_base64(base64_string: String) -> Result<Document, CursorError> {
    // change from base64
    let decoded = STANDARD.decode(&base64_string)?;
    // decode from bson
    let cursor_doc = bson::from_slice(decoded.as_slice()).unwrap();
    Ok(cursor_doc)
}

/// Converts an id into a MongoDb ObjectId
pub fn get_object_id(id: &str) -> Result<ObjectId, CursorError> {
    let object_id = match ObjectId::parse_str(id) {
        Ok(object_id) => object_id,
        Err(_e) => return Err(CursorError::InvalidId(id.to_string())),
    };
    Ok(object_id)
}
