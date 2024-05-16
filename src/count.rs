use bson::Document;
use futures_util::StreamExt;
use mongodb::Collection;
use mongodb::options::CountOptions;

use crate::{DirectedCursor, get_filter};
use crate::error::CursorError;
use crate::option::CursorOptions;

#[cfg(feature = "count")]
pub(crate) async fn has_page(
    collection: &Collection<Document>,
    filter: Document,
    mut options: CursorOptions,
    cursor: Option<&DirectedCursor>,
) -> Result<bool, CursorError> {
    let Some(cursor) = cursor else {
        return Ok(false);
    };

    options.set_cursor(cursor.clone());
    options.skip = None;
    let filter = get_filter(filter, &options, Some(cursor))?;

    Ok(collection
        .find(Some(filter), Some(options.into()))
        .await?
        .next()
        .await
        .transpose()?
        .is_some())
}

#[cfg(feature = "count")]
pub(crate) async fn count_documents<T: Sync>(
    mut options: CountOptions,
    collection: &Collection<T>,
    filter: Option<&Document>,
) -> Result<u64, CursorError> {
    options.limit = None;
    options.skip = None;
    let count_query = filter.map_or_else(Document::new, Clone::clone);
    Ok(collection
        .count_documents(count_query, Some(options))
        .await?)
}
