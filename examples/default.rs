use crate::helper::{create_options, print_details, MyFruit};
use bson::doc;
use mongodb::Client;
use mongodb_cursor_pagination::DirectedCursor;
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
        doc! { "name": "Orange", "how_many": 3 },
        doc! { "name": "Blueberry", "how_many": 25 },
        doc! { "name": "Bananas", "how_many": 8 },
        doc! { "name": "Grapes", "how_many": 12 },
    ];

    db.collection("myfruits")
        .insert_many(docs, None)
        .await
        .expect("Unable to insert data");

    // query page 1, 2 at a time
    let mut options = create_options(2, 0, doc! { "name": 1 });
    let mut find_results: FindResult<MyFruit> = fruits
        .find_paginated(None, Some(options), None)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![MyFruit::new("Apple", 5), MyFruit::new("Bananas", 8),]
    );
    print_details("First page", &find_results);

    // get the second page
    options = create_options(2, 0, doc! { "name": 1 });
    let mut cursor = find_results.page_info.end_cursor;
    find_results = fruits
        .find_paginated(None, Some(options), cursor)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![MyFruit::new("Blueberry", 25), MyFruit::new("Grapes", 12),]
    );
    print_details("Second page", &find_results);

    // get previous page
    options = create_options(2, 0, doc! { "name": 1 });
    cursor = find_results.page_info.start_cursor;
    find_results = fruits
        .find_paginated(None, Some(options), cursor)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![MyFruit::new("Apple", 5), MyFruit::new("Bananas", 8),]
    );
    print_details("Previous page", &find_results);

    // with a skip
    options = create_options(2, 3, doc! { "name": 1 });
    find_results = fruits
        .find_paginated(None, Some(options), None)
        .await
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
    options = create_options(2, 0, doc! { "name": 1 });
    cursor = find_results.page_info.start_cursor;
    find_results = fruits
        .find_paginated(None, Some(options), cursor)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![MyFruit::new("Bananas", 8), MyFruit::new("Blueberry", 25),]
    );
    print_details("Previous from skip", &find_results);

    // backwards one more time and we are all the way back
    options = create_options(2, 0, doc! { "name": 1 });
    cursor = find_results.page_info.start_cursor;
    find_results = fruits
        .find_paginated(None, Some(options), cursor)
        .await
        .expect("Unable to find data");
    assert_eq!(find_results.items, vec![MyFruit::new("Apple", 5),]);
    print_details(
        "Previous again - at beginning, but cursor was for before Banana (so only apple)",
        &find_results,
    );

    db.collection::<MyFruit>("myfruits")
        .drop(None)
        .await
        .expect("Unable to drop collection");
}
