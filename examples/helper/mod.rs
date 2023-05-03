use bson::doc;
use mongodb::options::FindOptions;
use mongodb_cursor_pagination::FindResult;
use serde::Deserialize;
use std::fmt::Debug;

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
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

pub fn create_options(limit: i64, skip: u64) -> FindOptions {
    FindOptions::builder()
        .limit(limit)
        .skip(skip)
        .sort(doc! { "name": 1 })
        .build()
}

pub fn print_details<T: Debug>(name: &str, find_results: &FindResult<T>) {
    println!(
        "{}:\nitems: {:?}\ntotal: {}\nnext: {:?}\nprevious: {:?}\nhas_previous: {}\nhas_next: {}",
        name,
        find_results.items,
        find_results.total_count,
        find_results.page_info.next_cursor,
        find_results.page_info.start_cursor,
        find_results.page_info.has_previous_page,
        find_results.page_info.has_next_page,
    );
    println!("-----------------");
}
