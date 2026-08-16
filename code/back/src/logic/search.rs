use std::collections::HashMap;

use nail_common::request::ArticleSearchParams;
use nail_common::response::search::{
    SearchArticleItem, SearchCommentItem, SearchHit, SearchPage, SearchVersionItem,
};
use nail_common::search::{SearchRange, SearchSortDirection, SearchSortField};

use crate::infrastructure::state::AppState;
use crate::logic::error::LogicError;
use crate::repository::search::{
    SearchCommentOutcome, SearchDocOutcome, SearchRequest, SearchSort,
};

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
            if trimmed.chars().count() as u64 > max_query_chars {
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

    let from_seconds = parse_iso8601_bound(params.from.as_deref(), "from")?;
    let to_seconds = parse_iso8601_bound(params.to.as_deref(), "to")?;
    if let (Some(from), Some(to)) = (from_seconds, to_seconds)
        && from > to
    {
        return Err(LogicError::bad_request("from must not be greater than to"));
    }

    let limit = params
        .limit
        .unwrap_or(state.config.server.search_page_size)
        .clamp(1, MAX_PAGE_SIZE);
    let page = params.page.unwrap_or(1).clamp(1, MAX_PAGE);
    let offset = page.saturating_sub(1).saturating_mul(limit);

    let outcome = state
        .search
        .read(
            &state.graph,
            SearchRequest {
                query,
                ranges,
                sort,
                from_seconds,
                to_seconds,
                offset,
                limit,
            },
        )
        .await
        .map_err(|error| LogicError::internal(format!("search failed: {error}")))?;

    let article_list = assemble_tree(&outcome.docs);
    let total = article_list.len() as u64;
    let sliced: Vec<SearchArticleItem> = article_list
        .into_iter()
        .skip(usize::try_from(offset).unwrap_or(usize::MAX))
        .take(usize::try_from(limit).unwrap_or(usize::MAX))
        .collect();

    let raw_total_pages = total.div_ceil(limit);
    let total_pages = raw_total_pages.min(state.config.server.max_search_pages);
    let truncated = raw_total_pages > state.config.server.max_search_pages;

    Ok(SearchPage {
        article_list: sliced,
        total,
        page,
        total_pages,
        has_next: page < total_pages,
        has_prev: page > 1,
        truncated,
    })
}

fn assemble_tree(docs: &[SearchDocOutcome]) -> Vec<SearchArticleItem> {
    struct VersionBuilder {
        version_id: String,
        version_number: String,
        time: String,
        version_hits: Vec<SearchHit>,
        comments: Vec<SearchCommentItem>,
    }
    struct ArticleBuilder {
        article_id: String,
        title: String,
        author_name: String,
        time: String,
        article_hits: Vec<SearchHit>,
        versions: Vec<VersionBuilder>,
    }

    let mut articles: Vec<ArticleBuilder> = Vec::new();
    let mut article_index: HashMap<String, usize> = HashMap::new();
    let mut version_index: HashMap<(String, String), usize> = HashMap::new();

    for doc in docs {
        match doc {
            SearchDocOutcome::Version(version) => {
                let article_pos = *article_index
                    .entry(version.article_id.clone())
                    .or_insert_with(|| {
                        articles.push(ArticleBuilder {
                            article_id: version.article_id.clone(),
                            title: version.title.clone(),
                            author_name: version.author_name.clone(),
                            time: format_search_time(uuidv7_secs(&version.article_id)),
                            article_hits: Vec::new(),
                            versions: Vec::new(),
                        });
                        articles.len() - 1
                    });
                let article = &mut articles[article_pos];
                for hit in &version.article_hits {
                    article.article_hits.push(hit_to_response(hit));
                }
                let show_version_card =
                    !version.version_hits.is_empty() || version.version_number_hit;
                if show_version_card {
                    let version_pos = *version_index
                        .entry((version.article_id.clone(), version.version_id.clone()))
                        .or_insert_with(|| {
                            article.versions.push(VersionBuilder {
                                version_id: version.version_id.clone(),
                                version_number: version.version_number.clone(),
                                time: format_search_time(uuidv7_secs(&version.version_id)),
                                version_hits: Vec::new(),
                                comments: Vec::new(),
                            });
                            article.versions.len() - 1
                        });
                    for hit in &version.version_hits {
                        article.versions[version_pos]
                            .version_hits
                            .push(hit_to_response(hit));
                    }
                }
            }
            SearchDocOutcome::Comment(comment) => {
                let article_pos = *article_index
                    .entry(comment.article_id.clone())
                    .or_insert_with(|| {
                        articles.push(ArticleBuilder {
                            article_id: comment.article_id.clone(),
                            title: comment.article_title.clone(),
                            author_name: comment.article_author_name.clone(),
                            time: format_search_time(uuidv7_secs(&comment.article_id)),
                            article_hits: Vec::new(),
                            versions: Vec::new(),
                        });
                        articles.len() - 1
                    });
                let article = &mut articles[article_pos];
                let version_pos = *version_index
                    .entry((comment.article_id.clone(), comment.version_id.clone()))
                    .or_insert_with(|| {
                        article.versions.push(VersionBuilder {
                            version_id: comment.version_id.clone(),
                            version_number: comment.version_number.clone(),
                            time: format_search_time(uuidv7_secs(&comment.version_id)),
                            version_hits: Vec::new(),
                            comments: Vec::new(),
                        });
                        article.versions.len() - 1
                    });
                article.versions[version_pos]
                    .comments
                    .push(comment_to_response(comment));
            }
        }
    }

    articles
        .into_iter()
        .map(|article| SearchArticleItem {
            article_id: article.article_id,
            title: article.title,
            author_name: article.author_name,
            time: article.time,
            article_hits: article.article_hits,
            versions: article
                .versions
                .into_iter()
                .map(|version| SearchVersionItem {
                    version_id: version.version_id,
                    version_number: version.version_number,
                    time: version.time,
                    version_hits: version.version_hits,
                    comments: version.comments,
                })
                .collect(),
        })
        .collect()
}

fn hit_to_response(hit: &crate::repository::search::SearchHitOutcome) -> SearchHit {
    SearchHit {
        field: hit.range,
        label: hit.range.label().to_string(),
        snippet: hit.snippet.clone(),
    }
}

fn comment_to_response(comment: &SearchCommentOutcome) -> SearchCommentItem {
    SearchCommentItem {
        comment_id: comment.comment_id.clone(),
        author_name: comment.author_name.clone(),
        time: format_search_time(uuidv7_secs(&comment.comment_id)),
        content: comment.content.clone(),
    }
}

fn uuidv7_secs(id: &str) -> i64 {
    nail_common::time::uuidv7_timestamp_secs(id).map_or(0, |secs| i64::try_from(secs).unwrap_or(0))
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
            "author_name" => SearchRange::AuthorName,
            "comment" => SearchRange::Comment,
            "note" => SearchRange::Note,
            "tag" => SearchRange::Tag,
            "version_number" => SearchRange::VersionNumber,
            _ => {
                return Err(LogicError::bad_request(format!(
                    "unknown search range: {token}"
                )));
            }
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
            return Err(LogicError::bad_request(format!(
                "invalid sort entry: {token}"
            )));
        };
        let field = match field.trim() {
            "time" => SearchSortField::Time,
            "title" => SearchSortField::Title,
            "author" => SearchSortField::Author,
            _ => {
                return Err(LogicError::bad_request(format!(
                    "unknown sort field: {field}"
                )));
            }
        };
        let direction = match direction.trim() {
            "asc" => SearchSortDirection::Asc,
            "desc" => SearchSortDirection::Desc,
            _ => {
                return Err(LogicError::bad_request(format!(
                    "unknown sort direction: {direction}"
                )));
            }
        };
        sort.push(SearchSort { field, direction });
    }
    Ok(sort)
}

fn format_search_time(timestamp_seconds: i64) -> String {
    let utc_ms = u64::try_from(timestamp_seconds.max(0))
        .unwrap_or(u64::MAX)
        .saturating_mul(1000);
    nail_common::time::format_rfc3339_utc(utc_ms).unwrap_or_default()
}

fn parse_iso8601_bound(value: Option<&str>, name: &str) -> Result<Option<u64>, LogicError> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let secs = nail_common::time::parse_iso8601_utc_secs(trimmed).ok_or_else(|| {
        LogicError::bad_request(format!(
            "{name} must be an ISO8601 datetime (year to second precision, no timezone means UTC)"
        ))
    })?;
    Ok(Some(u64::try_from(secs.max(0)).unwrap_or(u64::MAX)))
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
