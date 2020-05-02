# MongoDB Cursor Pagination

This package provides a cursor based pagination using the mongodb driver. Essentially instead of
page based pagination you receive cursors to both the start and end of the result set so that you can
ensure you get the next item, even if the data changes in between requests. That said, it also provides 
regular ole' page based pagination as well. If your options include skip and limit parameters then 
you'll do the page based. If you leave skip off or send a cursor, then it will use that instead (and ignore
the skip parameter.)

It's based on the [node.js module](https://github.com/mixmaxhq/mongo-cursor-pagination) but written in Rust.
You can read more about the concept on their [blog post](https://engineering.mixmax.com/blog/api-paging-built-the-right-way/). 

So far it only supports count and find. Search and aggregation will come when needed. 

For more examples of how to use, take a look at [graphql-mongodb-boilerplate](https://github.com/briandeboer/graphql-mongodb-boilerplate) and [mongodb-base-service](https://github.com/briandeboer/mongodb-base-service).

### Usage:
The usage is a bit different than the node version. See the examples for more details and a working example.
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

## Features
It has support for graphql (using [juniper](https://github.com/graphql-rust/juniper)) if you enable the `graphql` flag. You can use it by just including the PageInfo into your code.

```rust
use mongodb_cursor_pagination::{PageInfo, Edge};

#[derive(Serialize, Deserialize)]
struct MyDataConnection {
    page_info: PageInfo,
    edges: Vec<Edge>,
    data: Vec<MyData>,
    total_count: i64,
}

[juniper::object]
impl MyDataConnection {
    fn page_info(&self) -> &PageInfo {
        self.page_info
    }

    fn edges(&self) -> &Vec<Edge> {
        &self.edges
    }
    ...
}
```

Inside your cargo.toml dependencies

```
[dependencies]
mongodb_cursor_pagination = { version = "0.2.0", features = ["graphql"] }
```
