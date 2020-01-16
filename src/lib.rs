mod error;
mod options;

use bson::{bson, doc, oid::ObjectId, Document};
use error::CursorError;
use log::warn;
use mongodb::{options::FindOptions, Collection};
use options::CursorOptions;
use serde::Deserialize;
use std::io::Cursor;

#[derive(Debug)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: Option<String>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub cursor: String,
}

#[derive(Debug)]
pub struct FindResult<T> {
    pub page_info: PageInfo,
    pub edges: Vec<Edge>,
    pub total_count: i64,
    pub items: Vec<T>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CursorDirections {
    Previous,
    Next,
}

#[derive(Debug)]
pub struct PaginatedCursor {
    has_cursor: bool,
    cursor_doc: Document,
    direction: CursorDirections,
    options: CursorOptions,
}

impl<'a> PaginatedCursor {
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

    pub fn count_documents(
        &self,
        collection: &Collection,
        query: Option<&Document>,
    ) -> Result<i64, CursorError> {
        let mut count_options = self.options.clone();
        count_options.limit = None;
        count_options.skip = None;
        let count_query = if let Some(q) = query {
            q.clone()
        } else {
            Document::new()
        };
        let total_count: i64 = collection
            .count_documents(count_query, count_options)
            .unwrap();
        Ok(total_count)
    }

    /// Finds the documents in the `collection` matching `filter`.
    pub fn find<T>(
        &self,
        collection: &Collection,
        filter: Option<&Document>,
    ) -> Result<FindResult<T>, CursorError>
    where
        T: Deserialize<'a>,
    {
        // first count the docs
        let total_count: i64 = self.count_documents(collection, filter).unwrap();

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
            let skip_value: i64 = if let Some(s) = options.skip { s } else { 0 };
            if self.has_cursor || skip_value == 0 {
                options.skip = None;
            } else {
                has_skip = true;
            }
            let cursor = collection.find(query_doc, options).unwrap();

            for result in cursor {
                match result {
                    Ok(doc) => {
                        let item = bson::from_bson(bson::Bson::Document(doc.clone()))
                            .expect("Unable to parse document");
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
            // let has_previous
            let is_previous_query = self.has_cursor && self.direction == CursorDirections::Previous;

            let has_more: bool;
            if has_skip {
                has_more = (items.len() as i64 + skip_value) < total_count;
                has_previous_page = true;
                has_next_page = has_more;
            } else {
                has_more = items.len() > (self.options.limit.unwrap() - 1) as usize;
                has_previous_page = (self.has_cursor && self.direction == CursorDirections::Next)
                    || (is_previous_query && has_more);
                has_next_page = (self.direction == CursorDirections::Next && has_more)
                    || (is_previous_query && self.has_cursor);
            }

            // remove the extra item to check if we have more
            if has_more && !is_previous_query {
                items.pop();
                edges.pop();
            } else if has_more {
                items.remove(0);
                edges.remove(0);
            }

            // // reorder if we are going backwards
            // if is_previous_query {
            //     items.reverse();
            // }

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

    fn create_from_doc(&self, doc: &Document) -> String {
        let mut only_sort_keys = Document::new();
        if let Some(sort) = &self.options.sort {
            for key in sort.keys() {
                if doc.contains_key(key) {
                    if let Some(value) = doc.get(key) {
                        only_sort_keys.insert(key, value.clone());
                    }
                } else {
                    warn!("Doc doesn't contain {}", key);
                }
            }
            let mut buf = Vec::new();
            bson::encode_document(&mut buf, &only_sort_keys).unwrap();
            base64::encode(&buf)
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

        if query_doc.contains_key("$or") || query_doc.contains_key("$and") {
            return Err(CursorError::Unknown(
                "We don't handle those fancy queries yet".to_owned(),
            ));
        }

        if self.cursor_doc.is_empty() {
            return Ok(query_doc);
        } else if let Some(sort) = &self.options.sort {
            if sort.len() > 1 {
                let mut search_a = query_doc.clone();
                let mut search_b = query_doc.clone();
                for key in sort.keys() {
                    if key != "_id" && self.cursor_doc.contains_key(key) {
                        let value = self.cursor_doc.get(key).unwrap();
                        let direction = self.get_direction_from_key(&sort, key);
                        search_a.insert(key, doc! { direction: value.clone() });
                        search_b.insert(key, value.clone());
                    }
                }
                let object_id = self.cursor_doc.get("_id").unwrap().clone();
                let direction = self.get_direction_from_key(&sort, "_id");
                search_b.insert("_id", doc! { direction: object_id });
                if !search_a.is_empty() {
                    query_doc = doc! { "$or": [search_a, search_b] };
                } else {
                    query_doc = search_b;
                }
            } else {
                // this is the simplest form, it's just a sort by _id
                // TODO: handle fancier queries like $and, $or
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
    let decoded = base64::decode(&base64_string)?;
    // decode from bson
    let cursor_doc = bson::decode_document(&mut Cursor::new(&decoded)).unwrap();
    Ok(cursor_doc)
}

pub fn get_object_id(id: &str) -> Result<ObjectId, CursorError> {
    let object_id = match ObjectId::with_string(id) {
        Ok(object_id) => object_id,
        Err(_e) => return Err(CursorError::InvalidId(id.to_string())),
    };
    Ok(object_id)
}
