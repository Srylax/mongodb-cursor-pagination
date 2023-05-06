use bson::Document;
use mongodb::options::{CountOptions, EstimatedDocumentCountOptions, FindOptions};

const DEFAULT_LIMIT: i64 = 25;

// FIX: This isn't the best but I can't figure out the Rc or other options to make it work yet
#[derive(Clone, Debug)]
pub struct CursorOptions {
    pub allow_partial_results: Option<bool>,
    pub batch_size: Option<u32>,
    pub collation: Option<mongodb::options::Collation>,
    pub comment: Option<String>,
    pub cursor_type: Option<mongodb::options::CursorType>,
    pub hint: Option<mongodb::options::Hint>,
    pub limit: Option<i64>,
    pub max: Option<Document>,
    pub max_await_time: Option<std::time::Duration>,
    pub max_scan: Option<u64>,
    pub max_time: Option<std::time::Duration>,
    pub min: Option<Document>,
    pub no_cursor_timeout: Option<bool>,
    pub projection: Option<Document>,
    pub read_concern: Option<mongodb::options::ReadConcern>,
    pub return_key: Option<bool>,
    pub selection_criteria: Option<mongodb::options::SelectionCriteria>,
    pub show_record_id: Option<bool>,
    pub skip: Option<u64>,
    pub sort: Option<Document>,
}

impl From<Option<FindOptions>> for CursorOptions {
    fn from(options: Option<FindOptions>) -> Self {
        let old_opts = options.unwrap_or_else(|| FindOptions::builder().build());
        let limit = old_opts.limit.unwrap_or(DEFAULT_LIMIT).wrapping_add(1);
        // check the sort
        let mut sort = old_opts.sort.unwrap_or_default();
        if !sort.contains_key("_id") {
            sort.insert("_id", -1);
        }
        Self {
            allow_partial_results: old_opts.allow_partial_results,
            batch_size: old_opts.batch_size,
            collation: old_opts.collation.clone(),
            comment: old_opts.comment.clone(),
            cursor_type: old_opts.cursor_type,
            hint: old_opts.hint.clone(),
            limit: Some(limit),
            max: old_opts.max.clone(),
            max_await_time: old_opts.max_await_time,
            max_scan: old_opts.max_scan,
            max_time: old_opts.max_time,
            min: old_opts.min.clone(),
            no_cursor_timeout: old_opts.no_cursor_timeout,
            projection: old_opts.projection.clone(),
            read_concern: old_opts.read_concern,
            return_key: old_opts.return_key,
            selection_criteria: old_opts.selection_criteria,
            show_record_id: old_opts.show_record_id,
            skip: old_opts.skip,
            sort: Some(sort),
        }
    }
}

impl From<CursorOptions> for Option<FindOptions> {
    fn from(options: CursorOptions) -> Self {
        let find_options = FindOptions::builder()
            .allow_partial_results(options.allow_partial_results)
            .batch_size(options.batch_size)
            .collation(options.collation)
            .comment(options.comment)
            .cursor_type(options.cursor_type)
            .hint(options.hint)
            .limit(options.limit)
            .max(options.max)
            .max_await_time(options.max_await_time)
            .max_scan(options.max_scan)
            .max_time(options.max_time)
            .min(options.min)
            .no_cursor_timeout(options.no_cursor_timeout)
            .projection(options.projection)
            .read_concern(options.read_concern)
            .return_key(options.return_key)
            .selection_criteria(options.selection_criteria)
            .show_record_id(options.show_record_id)
            .skip(options.skip)
            .sort(options.sort)
            .build();
        Some(find_options)
    }
}

impl From<CursorOptions> for Option<CountOptions> {
    fn from(options: CursorOptions) -> Self {
        let count_options = CountOptions::builder()
            .collation(options.collation)
            .hint(options.hint)
            .limit(options.limit.map(|i| i as u64))
            .max_time(options.max_time)
            .skip(options.skip)
            .build();
        Some(count_options)
    }
}

impl From<CursorOptions> for Option<EstimatedDocumentCountOptions> {
    fn from(options: CursorOptions) -> Self {
        let count_options = EstimatedDocumentCountOptions::builder()
            .max_time(options.max_time)
            .selection_criteria(options.selection_criteria)
            .read_concern(options.read_concern)
            .build();
        Some(count_options)
    }
}
