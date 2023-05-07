use mongodb::options::{CountOptions, EstimatedDocumentCountOptions, FindOptions};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

const DEFAULT_LIMIT: i64 = 25;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CursorOptions(FindOptions);

impl Deref for CursorOptions {
    type Target = FindOptions;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CursorOptions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl From<FindOptions> for CursorOptions {
    fn from(mut options: FindOptions) -> Self {
        options.limit = Some(options.limit.unwrap_or(DEFAULT_LIMIT).saturating_add(1));
        let mut sort = options.sort.unwrap_or_default();
        if !sort.contains_key("_id") {
            sort.insert("_id", -1);
        }
        options.sort = Some(sort);
        Self(options)
    }
}

impl From<CursorOptions> for FindOptions {
    fn from(value: CursorOptions) -> Self {
        value.0
    }
}

impl From<CursorOptions> for CountOptions {
    fn from(value: CursorOptions) -> Self {
        Self::builder()
            .collation(value.collation.clone())
            .hint(value.hint.clone())
            .limit(value.limit.map(|i| i as u64))
            .max_time(value.max_time)
            .skip(value.skip)
            .build()
    }
}

impl From<CursorOptions> for EstimatedDocumentCountOptions {
    fn from(options: CursorOptions) -> Self {
        Self::builder()
            .max_time(options.max_time)
            .selection_criteria(options.selection_criteria.clone())
            .read_concern(options.read_concern.clone())
            .build()
    }
}
