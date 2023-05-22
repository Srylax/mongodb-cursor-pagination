use bson::{doc, Document};
use mongodb::options::FindOptions;
use mongodb_cursor_pagination::FindResult;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct MyFruit {
    name: String,
    how_many: i32,
}

impl MyFruit {
    #[must_use]
    pub fn new(name: impl Into<String>, how_many: i32) -> Self {
        Self {
            name: name.into(),
            how_many,
        }
    }
}

pub fn create_options(limit: i64, skip: u64, sort: Document) -> FindOptions {
    FindOptions::builder()
        .limit(limit)
        .skip(skip)
        .sort(sort)
        .build()
}

pub fn print_details<T: Debug>(name: &str, find_results: &FindResult<T>) {
    println!(
        "{}:\nitems: {:?}\ntotal: {}\nstart: {:?}\nend: {:?}\nhas_previous: {}\nhas_next: {}",
        name,
        find_results.items,
        find_results.total_count,
        find_results.page_info.start_cursor,
        find_results.page_info.end_cursor,
        find_results.page_info.has_previous_page,
        find_results.page_info.has_next_page,
    );
    println!("-----------------");
}
