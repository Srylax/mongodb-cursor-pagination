# MongoDB Cursor Pagination

Based on the [node module](https://github.com/mixmaxhq/mongo-cursor-pagination) but for Rust.
You can read more about it on their [blog post](https://engineering.mixmax.com/blog/api-paging-built-the-right-way/) and why it seems necessary. 

So far it only supports count and find. Search and aggregation will come when needed. 

### Usage:
Anyway, the usage is a bit different than the node version. See the examples for more details.
```rust
use mongodb::{options::FindOptions, Client};
use mongodb_cursor_pagination::{CursorDirections, FindResult, PaginatedCursor};

// Note that your data structure must derive Deserialize
#[derive(Debug, Deserialize)]
pub struct MyFruit {
    name: String,
    how_many: i32,
}

fn main() {
    let client =
        Client::with_uri_str("mongodb://localhost:27017/").expect("Failed to initialize client.");
    let db = client.database("mydatabase");

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
    println!("First page: {:?}", find_results);

    // get the second page
    options = create_options(2, 0);
    let mut cursor = find_results.page_info.next_cursor;
    find_results = PaginatedCursor::new(Some(options), cursor, Some(CursorDirections::Next))
        .find(&db.collection("myfruits"), None)
        .expect("Unable to find data");
    println!("Second page: {:?}", find_results);
}
```

### Response
The response FindResult<T> contains page info, cursors and edges (cursors for all of the items in the response).
```rust
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: Option<String>,
    pub next_cursor: Option<String>,
}

pub struct Edge {
    pub cursor: String,
}

pub struct FindResult<T> {
    pub page_info: PageInfo,
    pub edges: Vec<Edge>,
    pub total_count: i64,
    pub items: Vec<T>,
}
```
