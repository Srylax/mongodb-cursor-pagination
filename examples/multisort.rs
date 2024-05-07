#![allow(clippy::pedantic, clippy::restriction, clippy::cargo, missing_docs)]
use crate::helper::{create_options, print_details, MyFruit};
use bson::{doc, Document};
use mongodb::Client;
use mongodb_cursor_pagination::{FindResult, Pagination};

mod helper;

#[tokio::main]
async fn main() {
    let client = Client::with_uri_str("mongodb://localhost:27017/")
        .await
        .expect("Failed to initialize client.");
    let db = client.database("mongodb_cursor_pagination");
    let fruits = db.collection::<MyFruit>("myfruits");

    // Ensure there is no collection myfruits
    fruits.drop(None).await.expect("Failed to drop table");

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
    let mut options = create_options(3, 0, doc! { "how_many": 1, "name": -1, "non_existent": 1 });
    let mut find_results: FindResult<MyFruit> = fruits
        .find_paginated(None, Some(options), None)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![
            MyFruit::new("Orange", 3),
            MyFruit::new("Avocado", 5),
            MyFruit::new("Apple", 5)
        ]
    );
    print_details("First page", &find_results);

    // get the second page
    options = create_options(3, 0, doc! { "how_many": 1, "name": -1, "non_existent": 1 });
    let mut cursor = find_results.page_info.end_cursor;
    find_results = fruits
        .find_paginated(None, Some(options), cursor)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![
            MyFruit::new("Blueberry", 10),
            MyFruit::new("Bananas", 10),
            MyFruit::new("Grapes", 12)
        ]
    );
    print_details("Second page", &find_results);

    // get previous page
    options = create_options(3, 0, doc! { "how_many": 1, "name": -1, "non_existent": 1 });
    cursor = find_results.page_info.start_cursor;
    find_results = fruits
        .find_paginated(None, Some(options), cursor)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![
            MyFruit::new("Orange", 3),
            MyFruit::new("Avocado", 5),
            MyFruit::new("Apple", 5)
        ]
    );
    print_details("Previous page", &find_results);

    // with a skip
    options = create_options(3, 4, doc! { "how_many": 1, "name": -1, "non_existent": 1 });
    find_results = fruits
        .find_paginated(None, Some(options), None)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![
            MyFruit::new("Bananas", 10),
            MyFruit::new("Grapes", 12),
            MyFruit::new("Blackberry", 12)
        ]
    );
    print_details(
        "Skipped 4 (only three more left, so no more next page)",
        &find_results,
    );

    // backwards from skipping
    options = create_options(3, 0, doc! { "how_many": 1, "name": -1, "non_existent": 1 });
    cursor = find_results.page_info.start_cursor;
    find_results = fruits
        .find_paginated(None, Some(options), cursor)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![
            MyFruit::new("Avocado", 5),
            MyFruit::new("Apple", 5),
            MyFruit::new("Blueberry", 10),
        ]
    );
    print_details("Previous from skip", &find_results);

    // backwards one more time and we are all the way back
    options = create_options(3, 0, doc! { "how_many": 1, "name": -1, "non_existent": 1 });
    cursor = find_results.page_info.start_cursor;
    find_results = fruits
        .find_paginated(None, Some(options), cursor)
        .await
        .expect("Unable to find data");
    assert_eq!(find_results.items, vec![MyFruit::new("Orange", 3),]);
    print_details(
        "Previous again - at beginning, but cursor was for before Avocado (so only Apple)",
        &find_results,
    );

    db.collection::<Document>("myfruits")
        .drop(None)
        .await
        .expect("Unable to drop collection");
}
