# MongoDB Cursor Pagination
[![Crate][crate-image]][crate-link]
[![Docs][docs-image]][docs-link]
[![Build Status][build-image]][build-link]
![MIT licensed][license-image]

[Documentation][docs-link]  
[Examples][examples-link]

This package provides a cursor based pagination using the mongodb driver. Essentially instead of
page based pagination you receive cursors to both the start and end of the result set so that you can
ensure you get the next item, even if the data changes in between requests. That said, it also provides 
regular ole' page based pagination as well. If your options include skip and limit parameters then 
you'll do the page based. If you leave skip off or send a cursor, then it will use that instead (and ignore
the skip parameter.)

It's based on the [node.js module](https://github.com/mixmaxhq/mongo-cursor-pagination) but written in Rust.
You can read more about the concept on their [blog post](https://engineering.mixmax.com/blog/api-paging-built-the-right-way/). 

So far it only supports count and find. Search and aggregation will come when needed. 

[//]: # (badges)
[crate-image]: https://buildstats.info/crate/mongodb-cursor-pagination
[crate-link]: https://crates.io/crates/mongodb-cursor-pagination
[docs-image]: https://docs.rs/mongodb-cursor-pagination/badge.svg
[docs-link]: https://docs.rs/mongodb-cursor-pagination/
[build-image]: https://github.com/Srylax/mongodb-cursor-pagination/actions/workflows/rust.yml/badge.svg?branch=master
[build-link]: https://github.com/Srylax/mongodb-cursor-pagination/actions/workflows/rust.yml
[license-image]: https://img.shields.io/badge/license-MIT-blue.svg

[//]: # (other)
[examples-link]: https://github.com/Srylax/mongodb-cursor-pagination/tree/master/examples