#[macro_use]
extern crate bson;
#[macro_use]
extern crate serde;

extern crate mongodb;
extern crate mongodb_cursor_pagination;

use mongodb::{options::FindOptions, Client};
use mongodb_cursor_pagination::{CursorDirections, FindResult, PaginatedCursor};

#[derive(Debug, Deserialize)]
pub struct MyFruit {
    name: String,
    how_many: i32,
}

fn main() {
    let client =
        Client::with_uri_str("mongodb://localhost:27017/").expect("Failed to initialize client.");
    let db = client.database("mongodb_cursor_pagination");

    let docs = vec![
        doc! { "name": "Apple", "how_many": 5 },
        doc! { "name": "Avocado", "how_many": 5 },
        doc! { "name": "Orange", "how_many": 3 },
        doc! { "name": "Blueberry", "how_many": 10 },
        doc! { "name": "Bananas", "how_many": 10 },
        doc! { "name": "Blackberry", "how_many": 12 },
        doc! { "name": "Grapes", "how_many": 12 },
    ];

    // should result in...
    // Orange     | 3
    // Avocado    | 5
    // Apple      | 5
    // Blueberry  | 10
    // Bananas    | 10
    // Grapes     | 12
    // Blackberry | 12

    db.collection("myfruits")
        .insert_many(docs, None)
        .expect("Unable to insert data");

    // query page 1, 2 at a time
    let mut options = create_options(3, 0);
    let mut find_results: FindResult<MyFruit> = PaginatedCursor::new(Some(options), None, None)
        .find(&db.collection("myfruits"), None)
        .expect("Unable to find data");
    print_details("First page", &find_results);

    // get the second page
    options = create_options(3, 0);
    let mut cursor = find_results.page_info.next_cursor;
    find_results = PaginatedCursor::new(Some(options), cursor, Some(CursorDirections::Next))
        .find(&db.collection("myfruits"), None)
        .expect("Unable to find data");
    print_details("Second page", &find_results);

    // get previous page
    options = create_options(3, 0);
    cursor = find_results.page_info.start_cursor;
    find_results = PaginatedCursor::new(Some(options), cursor, Some(CursorDirections::Previous))
        .find(&db.collection("myfruits"), None)
        .expect("Unable to find data");
    print_details("Previous page", &find_results);

    // with a skip
    options = create_options(3, 4);
    find_results = PaginatedCursor::new(Some(options), None, None)
        .find(&db.collection("myfruits"), None)
        .expect("Unable to find data");
    print_details(
        "Skipped 4 (only three more left, so no more next page)",
        &find_results,
    );

    // backwards from skipping
    options = create_options(3, 0);
    cursor = find_results.page_info.start_cursor;
    find_results = PaginatedCursor::new(Some(options), cursor, Some(CursorDirections::Previous))
        .find(&db.collection("myfruits"), None)
        .expect("Unable to find data");
    print_details("Previous from skip", &find_results);

    // backwards one more time and we are all the way back
    options = create_options(3, 0);
    cursor = find_results.page_info.start_cursor;
    find_results = PaginatedCursor::new(Some(options), cursor, Some(CursorDirections::Previous))
        .find(&db.collection("myfruits"), None)
        .expect("Unable to find data");
    print_details(
        "Previous again - at beginning, but cursor was for before Avocado (so only Orange)",
        &find_results,
    );

    db.collection("myfruits")
        .drop(None)
        .expect("Unable to drop collection");
}

fn create_options(limit: i64, skip: i64) -> FindOptions {
    FindOptions::builder()
        .limit(limit)
        .skip(skip)
        .sort(doc! { "how_many": 1, "name": -1 })
        .build()
}

fn print_details(name: &str, find_results: &FindResult<MyFruit>) {
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
