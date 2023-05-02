#[macro_use]
extern crate bson;
#[macro_use]
extern crate serde;

extern crate mongodb;
extern crate mongodb_cursor_pagination;

use crate::helper::{create_options, print_details, MyFruit};
use mongodb::Client;
use mongodb_cursor_pagination::{CursorDirections, FindResult, PaginatedCursor};

mod helper;

fn main() {
    let client =
        Client::with_uri_str("mongodb://localhost:27017/").expect("Failed to initialize client.");
    let db = client.database("mongodb_cursor_pagination");

    // Ensure there is no collection myfruits
    let _ = db.collection("myfruits").drop(None);

    let docs = vec![
        doc! { "name": "Apple", "how_many": 5 },
        doc! { "name": "Orange", "how_many": 3 },
        doc! { "name": "Blueberry", "how_many": 25 },
        doc! { "name": "Bananas", "how_many": 8 },
        doc! { "name": "Grapes", "how_many": 12 },
    ];

    db.collection("myfruits")
        .insert_many(docs, None)
        .expect("Unable to insert data");

    // query page 1, 2 at a time
    let mut options = create_options(2, 0);
    let mut find_results: FindResult<MyFruit> = PaginatedCursor::new(Some(options), None, None)
        .find(&db.collection("myfruits"), None)
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![MyFruit::new("Apple", 5), MyFruit::new("Bananas", 8),]
    );
    print_details("First page", &find_results);

    // get the second page
    options = create_options(2, 0);
    let mut cursor = find_results.page_info.next_cursor;
    find_results = PaginatedCursor::new(Some(options), cursor, Some(CursorDirections::Next))
        .find(&db.collection("myfruits"), None)
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![MyFruit::new("Blueberry", 25), MyFruit::new("Grapes", 12),]
    );
    print_details("Second page", &find_results);

    // get previous page
    options = create_options(2, 0);
    cursor = find_results.page_info.start_cursor;
    find_results = PaginatedCursor::new(Some(options), cursor, Some(CursorDirections::Previous))
        .find(&db.collection("myfruits"), None)
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![MyFruit::new("Apple", 5), MyFruit::new("Bananas", 8),]
    );
    print_details("Previous page", &find_results);

    // with a skip
    options = create_options(2, 3);
    find_results = PaginatedCursor::new(Some(options), None, None)
        .find(&db.collection("myfruits"), None)
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![MyFruit::new("Grapes", 12), MyFruit::new("Orange", 3),]
    );
    print_details(
        "Skipped 3 (only two more left, so no more next page)",
        &find_results,
    );

    // backwards from skipping
    options = create_options(2, 0);
    cursor = find_results.page_info.start_cursor;
    find_results = PaginatedCursor::new(Some(options), cursor, Some(CursorDirections::Previous))
        .find(&db.collection("myfruits"), None)
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![MyFruit::new("Bananas", 8), MyFruit::new("Blueberry", 25),]
    );
    print_details("Previous from skip", &find_results);

    // backwards one more time and we are all the way back
    options = create_options(2, 0);
    cursor = find_results.page_info.start_cursor;
    find_results = PaginatedCursor::new(Some(options), cursor, Some(CursorDirections::Previous))
        .find(&db.collection("myfruits"), None)
        .expect("Unable to find data");
    assert_eq!(find_results.items, vec![MyFruit::new("Apple", 5),]);
    print_details(
        "Previous again - at beginning, but cursor was for before Banana (so only apple)",
        &find_results,
    );

    db.collection("myfruits")
        .drop(None)
        .expect("Unable to drop collection");
}
