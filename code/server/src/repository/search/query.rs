use common::search::SearchRange;

use super::schema::{
    FIELD_ARTICLE_ID, FIELD_AUTHOR_ID, FIELD_AUTHOR_NAME, FIELD_COMMENT_ID, FIELD_CONTENT,
    FIELD_NOTE, FIELD_ROLE, FIELD_SUMMARY, FIELD_TAGS, FIELD_TITLE, FIELD_VERSION_ID,
    FIELD_VERSION_NUMBER,
};

pub(super) fn effective_ranges(ranges: &[SearchRange]) -> Vec<SearchRange> {
    ranges.to_vec()
}

pub(super) fn request_field_names(ranges: &[SearchRange]) -> Vec<String> {
    effective_ranges(ranges)
        .iter()
        .map(|range| range_field_name(*range).to_string())
        .collect()
}

fn range_field_name(range: SearchRange) -> &'static str {
    match range {
        SearchRange::Title => FIELD_TITLE,
        SearchRange::Summary => FIELD_SUMMARY,
        SearchRange::AuthorName => FIELD_AUTHOR_NAME,
        SearchRange::Comment => FIELD_CONTENT,
        SearchRange::Note => FIELD_NOTE,
        SearchRange::Tag => FIELD_TAGS,
        SearchRange::VersionNumber => FIELD_VERSION_NUMBER,
        SearchRange::ArticleId => FIELD_ARTICLE_ID,
        SearchRange::VersionId => FIELD_VERSION_ID,
        SearchRange::CommentId => FIELD_COMMENT_ID,
        SearchRange::AuthorId => FIELD_AUTHOR_ID,
        SearchRange::Role => FIELD_ROLE,
    }
}
