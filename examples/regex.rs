#![allow(clippy::pedantic, clippy::restriction, clippy::cargo, missing_docs)]

use crate::helper::{create_options, print_details, MyFruit};
use bson::doc;
use bson::{Bson, Regex};
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
        doc! { "name": "Apple", "how_many": 2, "spanish": "Manzana" },
        doc! { "name": "Avocado", "how_many": 5, "spanish": "Aguacate" },
        doc! { "name": "Orange", "how_many": 3, "spanish": "Naranja" },
        doc! { "name": "Bananas", "how_many": 10, "spanish": "Bananas" },
        doc! { "name": "Blackberry", "how_many": 12, "spanish": "Mora" },
        doc! { "name": "Blueberry", "how_many": 10, "spanish": "Arandano" },
        doc! { "name": "Lingonberry", "how_many": 5, "spanish": "Arandano roja" },
        doc! { "name": "Raspberry", "how_many": 5, "spanish": "Frambuesa" },
        doc! { "name": "Strawberry", "how_many": 5, "spanish": "Fresa" },
        doc! { "name": "Grapes", "how_many": 12, "spanish": "Uvas" },
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
    let mut options = create_options(2, 0, doc! { "name": 1 });
    let filter = doc! { "$or": [
        { "name": Bson::RegularExpression(Regex { pattern: String::from("berry"), options: String::from("i") })},
        { "spanish": Bson::RegularExpression(Regex { pattern: String::from("ana"), options: String::from("i") })},
    ] };
    let mut find_results: FindResult<MyFruit> = fruits
        .find_paginated(Some(filter.clone()), Some(options), None)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![MyFruit::new("Apple", 2), MyFruit::new("Bananas", 10),]
    );
    print_details("First page", &find_results);

    // get the second page
    options = create_options(2, 0, doc! { "name": 1 });
    let mut cursor = find_results.page_info.end_cursor;
    find_results = fruits
        .find_paginated(Some(filter.clone()), Some(options), cursor)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![
            MyFruit::new("Blackberry", 12),
            MyFruit::new("Blueberry", 10),
        ]
    );
    print_details("Second page", &find_results);

    // get previous page
    options = create_options(2, 0, doc! { "name": 1 });
    cursor = find_results.page_info.start_cursor;
    find_results = fruits
        .find_paginated(Some(filter.clone()), Some(options), cursor)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![MyFruit::new("Apple", 2), MyFruit::new("Bananas", 10),]
    );
    print_details("Previous (first) page", &find_results);

    // get the second page again
    options = create_options(2, 0, doc! { "name": 1 });
    cursor = find_results.page_info.end_cursor;
    find_results = fruits
        .find_paginated(Some(filter.clone()), Some(options), cursor)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![
            MyFruit::new("Blackberry", 12),
            MyFruit::new("Blueberry", 10),
        ]
    );
    print_details("Second page (again)", &find_results);

    // get the third page
    options = create_options(2, 0, doc! { "name": 1 });
    cursor = find_results.page_info.end_cursor;
    find_results = fruits
        .find_paginated(Some(filter.clone()), Some(options), cursor)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![MyFruit::new("Lingonberry", 5), MyFruit::new("Raspberry", 5),]
    );
    print_details("Third page", &find_results);

    // get previous page
    options = create_options(2, 0, doc! { "name": 1 });
    cursor = find_results.page_info.start_cursor;
    find_results = fruits
        .find_paginated(Some(filter.clone()), Some(options), cursor)
        .await
        .expect("Unable to find data");
    assert_eq!(
        find_results.items,
        vec![
            MyFruit::new("Blackberry", 12),
            MyFruit::new("Blueberry", 10),
        ]
    );
    print_details("Previous (second) page", &find_results);

    db.collection::<MyFruit>("myfruits")
        .drop(None)
        .await
        .expect("Unable to drop collection");
}
