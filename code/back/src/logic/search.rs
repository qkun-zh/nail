use nail_common::search::{
    ArticleSearchParams, SearchArticleItem, SearchHit, SearchPage, SearchRange, SearchSortDirection,
    SearchSortField,
};

use crate::infrastructure::state::AppState;
use crate::logic::error::LogicError;
use crate::repository::search::{SearchRequest, SearchSort};

const MAX_SEARCH_PAGES: u64 = 1024;
const DEFAULT_PAGE_SIZE: u64 = 8;
const MAX_PAGE_SIZE: u64 = 200;
const MAX_PAGE: u64 = 10_000;

pub async fn search_articles(
    state: &AppState,
    params: &ArticleSearchParams,
) -> Result<SearchPage, LogicError> {
    let max_query_chars = state.config.server.max_search_query_chars;

    let query = match params.q.as_deref() {
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.len() as u64 > max_query_chars {
                return Err(LogicError::bad_request(format!(
                    "search query too long (max {max_query_chars} chars)"
                )));
            }
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        None => None,
    };

    let ranges = parse_ranges(params.ranges.as_deref())?;
    let sort = parse_sort(params.sort.as_deref())?;

    let from_seconds = params.from;
    let to_seconds = params.to;
    if let (Some(from), Some(to)) = (from_seconds, to_seconds)
        && from > to
    {
        return Err(LogicError::bad_request(
            "from must not be greater than to",
        ));
    }

    let limit = params.limit.unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, MAX_PAGE_SIZE);
    let page = params.page.unwrap_or(1).clamp(1, MAX_PAGE);
    let offset = page.saturating_sub(1).saturating_mul(limit);

    let outcome = state
        .search
        .read(SearchRequest {
            query,
            ranges,
            sort,
            from_seconds,
            to_seconds,
            offset,
            limit,
        })
        .await
        .map_err(|error| LogicError::internal(format!("search failed: {error}")))?;

    let raw_total_pages = outcome.total.div_ceil(limit);
    let total_pages = raw_total_pages.min(MAX_SEARCH_PAGES);
    let truncated = raw_total_pages > MAX_SEARCH_PAGES;

    let article_list = outcome
        .articles
        .into_iter()
        .map(|article| {
            let time = format_search_time(
                article.timestamp_seconds,
                state.config.server.timezone_offset_seconds,
            );
            SearchArticleItem {
                id: article.id,
                title: article.title,
                author: article.author,
                time,
                hits: article
                    .hits
                    .into_iter()
                    .map(|hit| SearchHit {
                        field: hit.range,
                        label: hit.range.label().to_string(),
                        snippet: hit.snippet,
                    })
                    .collect(),
            }
        })
        .collect();

    Ok(SearchPage {
        article_list,
        total: outcome.total,
        page,
        total_pages,
        has_more: page < total_pages,
        has_prev: page > 1,
        truncated,
    })
}

fn parse_ranges(raw: Option<&str>) -> Result<Vec<SearchRange>, LogicError> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    let mut ranges = Vec::new();
    for token in raw.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let range = match token {
            "title" => SearchRange::Title,
            "summary" => SearchRange::Summary,
            "author" => SearchRange::Author,
            "comment" => SearchRange::Comment,
            "note" => SearchRange::Note,
            "tag" => SearchRange::Tag,
            _ => return Err(LogicError::bad_request(format!("unknown search range: {token}"))),
        };
        if !ranges.contains(&range) {
            ranges.push(range);
        }
    }
    Ok(ranges)
}

fn parse_sort(raw: Option<&str>) -> Result<Vec<SearchSort>, LogicError> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    let mut sort = Vec::new();
    for token in raw.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let Some((field, direction)) = token.split_once(':') else {
            return Err(LogicError::bad_request(format!("invalid sort entry: {token}")));
        };
        let field = match field.trim() {
            "time" => SearchSortField::Time,
            "title" => SearchSortField::Title,
            "author" => SearchSortField::Author,
            _ => return Err(LogicError::bad_request(format!("unknown sort field: {field}"))),
        };
        let direction = match direction.trim() {
            "asc" => SearchSortDirection::Asc,
            "desc" => SearchSortDirection::Desc,
            _ => {
                return Err(LogicError::bad_request(format!(
                    "unknown sort direction: {direction}"
                )))
            }
        };
        sort.push(SearchSort { field, direction });
    }
    Ok(sort)
}

fn format_search_time(timestamp_seconds: i64, offset_seconds: i32) -> String {
    let utc_ms = (timestamp_seconds.max(0) as u64).saturating_mul(1000);
    nail_common::time::format_rfc3339_with_offset(utc_ms, offset_seconds).unwrap_or_default()
}

pub(crate) async fn sync_article_best_effort(state: &AppState, article_id: &str) {
    if let Err(error) = state.search.sync(&state.graph, article_id).await {
        tracing::warn!(
            article_id = %article_id,
            error = %error,
            "failed to sync search index"
        );
    }
}

pub(crate) async fn sync_user_best_effort(state: &AppState, user_id: &str) {
    if let Err(error) = state.search.sync_user(&state.graph, user_id).await {
        tracing::warn!(
            user_id = %user_id,
            error = %error,
            "failed to sync user search index"
        );
    }
}

pub(crate) async fn sync_all_best_effort(state: &AppState) {
    if let Err(error) = state.search.sync_all(&state.graph).await {
        tracing::warn!(error = %error, "failed to rebuild search index");
    }
}
