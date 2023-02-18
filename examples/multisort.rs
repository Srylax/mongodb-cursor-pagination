use crate::helper::{create_options, print_details, MyFruit};
use bson::{doc, Document};
use mongodb::Client;
use mongodb::{options::FindOptions, Client};
use mongodb_cursor_pagination::{CursorDirections, FindResult, PaginatedCursor};
use serde::Deserialize;

mod helper;

#[tokio::main]
async fn main() {
    let client = Client::with_uri_str("mongodb://localhost:27017/")
        .await
        .expect("Failed to initialize client.");
    let db = client.database("mongodb_cursor_pagination");

    // Ensure there is no collection myfruits
    let _ = db.collection("myfruits").drop(None);

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
        .await
        .expect("Unable to insert data");

    // query page 1, 2 at a time
    let mut options = create_options(3, 0);
    let mut find_results: FindResult<MyFruit> = PaginatedCursor::new(Some(options), None, None)
        .find(&db.collection("myfruits"), None)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![
            MyFruit::new("Apple", 5),
            MyFruit::new("Avocado", 5),
            MyFruit::new("Bananas", 10)
        ]
    );
    print_details("First page", &find_results);

    // get the second page
    options = create_options(3, 0);
    let mut cursor = find_results.page_info.next_cursor;
    find_results = PaginatedCursor::new(Some(options), cursor, Some(CursorDirections::Next))
        .find(&db.collection("myfruits"), None)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![
            MyFruit::new("Blackberry", 12),
            MyFruit::new("Blueberry", 10),
            MyFruit::new("Grapes", 12)
        ]
    );
    print_details("Second page", &find_results);

    // get previous page
    options = create_options(3, 0);
    cursor = find_results.page_info.start_cursor;
    find_results = PaginatedCursor::new(Some(options), cursor, Some(CursorDirections::Previous))
        .find(&db.collection("myfruits"), None)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![
            MyFruit::new("Apple", 5),
            MyFruit::new("Avocado", 5),
            MyFruit::new("Bananas", 10)
        ]
    );
    print_details("Previous page", &find_results);

    // with a skip
    options = create_options(3, 4);
    find_results = PaginatedCursor::new(Some(options), None, None)
        .find(&db.collection("myfruits"), None)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![
            MyFruit::new("Blueberry", 10),
            MyFruit::new("Grapes", 12),
            MyFruit::new("Orange", 3)
        ]
    );
    print_details(
        "Skipped 4 (only three more left, so no more next page)",
        &find_results,
    );

    // backwards from skipping
    options = create_options(3, 0);
    cursor = find_results.page_info.start_cursor;
    find_results = PaginatedCursor::new(Some(options), cursor, Some(CursorDirections::Previous))
        .find(&db.collection("myfruits"), None)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![
            MyFruit::new("Avocado", 5),
            MyFruit::new("Bananas", 10),
            MyFruit::new("Blackberry", 12),
        ]
    );
    print_details("Previous from skip", &find_results);

    // backwards one more time and we are all the way back
    options = create_options(3, 0);
    cursor = find_results.page_info.start_cursor;
    find_results = PaginatedCursor::new(Some(options), cursor, Some(CursorDirections::Previous))
        .find(&db.collection("myfruits"), None)
        .await
        .expect("Unable to find data");
    assert_eq!(find_results.items, vec![MyFruit::new("Apple", 5),]);
    print_details(
        "Previous again - at beginning, but cursor was for before Avocado (so only Apple)",
        &find_results,
    );

    db.collection::<Document>("myfruits")
        .drop(None)
        .await
        .expect("Unable to drop collection");
}
