use bson::Bson;
use mongodb::options::{CountOptions, EstimatedDocumentCountOptions, FindOptions};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut, Neg};

use crate::DirectedCursor;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CursorOptions {
    options: FindOptions,
    directed_options: FindOptions,
    cursor: Option<DirectedCursor>,
}

impl CursorOptions {
    pub fn new(options: impl Into<FindOptions>, cursor: Option<DirectedCursor>) -> Self {
        let mut options = options.into();

        let mut sort = options.sort.unwrap_or_default();
        if !sort.contains_key("_id") {
            sort.insert("_id", -1);
        }
        options.sort = Some(sort);
        Self {
            directed_options: Self::get_directed(options.clone(), cursor.as_ref()),
            cursor,
            options,
        }
    }

    pub fn set_cursor(&mut self, cursor: DirectedCursor) {
        self.cursor = Some(cursor);
        self.directed_options = Self::get_directed(self.options.clone(), self.cursor.as_ref());
    }

    fn get_directed(mut options: FindOptions, cursor: Option<&DirectedCursor>) -> FindOptions {
        if !matches!(cursor, Some(DirectedCursor::Backwards(_))) {
            return options;
        }

        if let Some(sort) = options.sort.as_mut() {
            sort.iter_mut().for_each(|(_key, value)| {
                if let Bson::Int32(num) = value {
                    *value = Bson::Int32(num.neg());
                }
                if let Bson::Int64(num) = value {
                    *value = Bson::Int64(num.neg());
                }
            });
        }
        options
    }
}

impl Deref for CursorOptions {
    type Target = FindOptions;

    fn deref(&self) -> &Self::Target {
        &self.directed_options
    }
}

impl DerefMut for CursorOptions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.directed_options
    }
}
impl From<CursorOptions> for FindOptions {
    fn from(value: CursorOptions) -> Self {
        value.directed_options
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
