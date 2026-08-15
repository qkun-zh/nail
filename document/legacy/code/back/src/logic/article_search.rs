
use common::search::ArticleSearchParams;

use crate::logic::error::LogicError;
use crate::other::AppState;
use crate::repo;
use crate::repo::search::SearchQuery;

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub field: String,
    pub label: String,
    pub snippet: String,
}

#[derive(Debug, Clone)]
pub struct SearchArticleItem {
    pub id: String,
    pub title: String,
    pub author: String,
    pub time: String,
    pub hits: Vec<SearchHit>,
}

pub struct SearchPage {
    pub article_list: Vec<SearchArticleItem>,
    pub total: u64,
    pub page: u64,
    pub total_pages: u64,
    pub has_more: bool,
    pub has_prev: bool,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Range {
    Title,
    Summary,
    Author,
    Comment,
    Note,
    Tag,
}

impl Range {
    pub(crate) fn from_str(s: &str) -> Option<Range> {
        match s {
            "title" => Some(Range::Title),
            "summary" => Some(Range::Summary),
            "author" => Some(Range::Author),
            "comment" => Some(Range::Comment),
            "note" => Some(Range::Note),
            "tag" => Some(Range::Tag),
            _ => None,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Range::Title => "标题",
            Range::Summary => "摘要",
            Range::Author => "作者",
            Range::Comment => "评论",
            Range::Note => "版本说明",
            Range::Tag => "标签",
        }
    }

    pub(crate) fn field(self) -> &'static str {
        match self {
            Range::Title => "title",
            Range::Summary => "summary",
            Range::Author => "author",
            Range::Comment => "comment",
            Range::Note => "note",
            Range::Tag => "tag",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortField {
    Time,
    Title,
    Author,
}

impl SortField {
    fn from_str(s: &str) -> Option<SortField> {
        match s {
            "time" => Some(SortField::Time),
            "title" => Some(SortField::Title),
            "author" => Some(SortField::Author),
            _ => None,
        }
    }

    fn index_field(self) -> &'static str {
        match self {
            SortField::Time => "ts",
            SortField::Title => "title",
            SortField::Author => "author",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy)]
struct SortKey {
    field: SortField,
    order: SortOrder,
}

pub async fn handle_search_articles(
    state: &AppState,
    params: &ArticleSearchParams,
) -> Result<SearchPage, LogicError> {
    let max_query_chars = state.config.server.max_search_query_chars as usize;
    let q = normalize_q(params.q.as_deref(), max_query_chars)?;
    let ranges = parse_ranges(params.ranges.as_deref())?;
    let sort_keys = parse_sort(params.sort.as_deref())?;
    let (from, to) = validate_from_to(params.from, params.to)?;

    let limit = params
        .limit
        .unwrap_or(state.config.server.search_page_size)
        .max(1)
        .min(state.config.server.max_search_page_size);
    let page = params
        .page
        .unwrap_or(1)
        .max(1)
        .min(state.config.server.max_search_pages);

    let fields: Vec<String> = ranges.iter().map(|r| r.field().to_string()).collect();
    let sort: Vec<(String, bool)> = sort_keys
        .iter()
        .map(|key| {
            (
                key.field.index_field().to_string(),
                key.order == SortOrder::Desc,
            )
        })
        .collect();
    let outcome = repo::search::search_articles(
        &state.search,
        &state.db,
        SearchQuery {
            q,
            fields,
            from,
            to,
            sort,
            offset: page.saturating_sub(1).saturating_mul(limit),
            limit,
        },
    )
    .await
    .map_err(|e| LogicError::internal(format!("search failed: {e}")))?;

    let max_search_pages = state.config.server.max_search_pages;
    let total = outcome.total;
    let total_pages = if total == 0 { 0 } else { total.div_ceil(limit) };
    let truncated = total_pages > max_search_pages;
    let shown_pages = total_pages.min(max_search_pages);
    if page > total_pages {
        return Ok(SearchPage {
            article_list: Vec::new(),
            has_more: false,
            has_prev: page > 1,
            total,
            page,
            total_pages: shown_pages,
            truncated,
        });
    }

    let article_list = build_items(outcome.docs, &ranges);
    Ok(SearchPage {
        has_more: page.saturating_mul(limit) < total && page < shown_pages,
        has_prev: page > 1,
        total,
        page,
        total_pages: shown_pages,
        truncated,
        article_list,
    })
}

fn build_items(docs: Vec<repo::search::SearchHitDoc>, ranges: &[Range]) -> Vec<SearchArticleItem> {
    docs.into_iter()
        .filter_map(|doc| {
            let by_field: std::collections::HashMap<String, String> =
                doc.hits.into_iter().collect();
            let mut hits = Vec::new();
            for range in ranges {
                if let Some(snippet) = by_field.get(range.field()) {
                    hits.push(SearchHit {
                        field: range.field().to_string(),
                        label: range.label().to_string(),
                        snippet: snippet.clone(),
                    });
                }
            }
            Some(SearchArticleItem {
                id: doc.id,
                title: doc.title,
                author: doc.author,
                time: format_time(doc.ts_secs),
                hits,
            })
        })
        .collect()
}

fn format_time(ts_secs: i64) -> String {
    use chrono::{FixedOffset, TimeZone};
    let tz = FixedOffset::east_opt(8 * 3600).expect("+08:00 is a valid fixed offset");
    chrono::Utc
        .timestamp_opt(ts_secs, 0)
        .single()
        .map(|dt| dt.with_timezone(&tz).to_rfc3339())
        .unwrap_or_default()
}

fn normalize_q(q: Option<&str>, max_chars: usize) -> Result<Option<String>, LogicError> {
    let Some(v) = q.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if v.chars().count() > max_chars {
        return Err(LogicError::bad_request(format!(
            "query string too long (max {max_chars} chars)"
        )));
    }
    Ok(Some(v.to_string()))
}

fn parse_ranges(s: Option<&str>) -> Result<Vec<Range>, LogicError> {
    const ALL: [Range; 6] = [
        Range::Title,
        Range::Summary,
        Range::Author,
        Range::Comment,
        Range::Note,
        Range::Tag,
    ];
    let Some(s) = s.map(str::trim).filter(|x| !x.is_empty()) else {
        return Ok(ALL.to_vec());
    };
    let mut seen = Vec::new();
    for piece in s.split(',') {
        let piece = piece.trim();
        if piece.is_empty() {
            continue;
        }
        let range = Range::from_str(piece)
            .ok_or_else(|| LogicError::bad_request(format!("invalid range: {piece}")))?;
        if !seen.contains(&range) {
            seen.push(range);
        }
    }
    if seen.is_empty() {
        Ok(ALL.to_vec())
    } else {
        Ok(seen)
    }
}

fn parse_sort(s: Option<&str>) -> Result<Vec<SortKey>, LogicError> {
    let Some(s) = s.map(str::trim).filter(|x| !x.is_empty()) else {
        return Ok(Vec::new());
    };
    let mut keys = Vec::new();
    for piece in s.split(',') {
        let piece = piece.trim();
        if piece.is_empty() {
            continue;
        }
        let (field_s, dir_s) = piece
            .split_once(':')
            .ok_or_else(|| LogicError::bad_request(format!("invalid sort: {piece}")))?;
        let field = SortField::from_str(field_s)
            .ok_or_else(|| LogicError::bad_request(format!("invalid sort key: {field_s}")))?;
        let order = match dir_s {
            "asc" => SortOrder::Asc,
            "desc" => SortOrder::Desc,
            _ => {
                return Err(LogicError::bad_request(format!(
                    "invalid sort direction: {dir_s}"
                )));
            }
        };
        keys.push(SortKey { field, order });
    }
    Ok(keys)
}

fn validate_from_to(
    from: Option<u64>,
    to: Option<u64>,
) -> Result<(Option<u64>, Option<u64>), LogicError> {
    if let (Some(f), Some(t)) = (from, to)
        && f > t
    {
        return Err(LogicError::bad_request("from cannot be later than to"));
    }
    Ok((from, to))
}
