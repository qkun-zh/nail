use common::search::SearchRange;
use database::Database;
use searcher::{DocHit, FieldHit, SearchField};

pub(crate) mod db;
pub mod document;

#[derive(Debug, Clone, Default)]
pub struct SearchRequest {
    pub query: Option<String>,
    pub ranges: Vec<SearchRange>,
    pub from_seconds: Option<u64>,
    pub to_seconds: Option<u64>,
    pub offset: u64,
    pub limit: u64,
}

#[derive(Debug, Clone)]
pub struct SearchHitOutcome {
    pub range: SearchRange,
    pub snippet: String,
}

#[derive(Debug, Clone)]
pub struct SearchVersionOutcome {
    pub article_id: String,
    pub version_id: String,
    pub version_number: String,
    pub title: String,
    pub author_id: String,
    pub author_name: String,
    pub article_hits: Vec<SearchHitOutcome>,
    pub version_hits: Vec<SearchHitOutcome>,
    pub version_number_hit: bool,
}

#[derive(Debug, Clone)]
pub struct SearchCommentOutcome {
    pub article_id: String,
    pub version_id: String,
    pub comment_id: String,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub article_title: String,
    pub article_author_id: String,
    pub article_author_name: String,
    pub version_number: String,
}

#[derive(Debug, Clone)]
pub enum SearchDocOutcome {
    Version(SearchVersionOutcome),
    Comment(SearchCommentOutcome),
}

#[derive(Debug, Clone)]
pub struct SearchOutcome {
    pub docs: Vec<SearchDocOutcome>,
}

/// Thin adapter between the server's graph-backed domain and the standalone
/// `searcher` crate. All seekstorm knowledge lives behind `searcher`; this
/// module only converts rows to documents, ranges to search fields, and hits
/// back to the outcome types the logic layer consumes.
#[derive(Clone)]
pub struct Searcher {
    inner: searcher::Searcher,
}

impl Searcher {
    pub async fn open_or_create(path: &str) -> anyhow::Result<Self> {
        Self::open_or_create_with_segments(path, searcher::DEFAULT_SEGMENT_NUMBER_BITS).await
    }

    pub async fn open_or_create_with_segments(
        path: &str,
        segment_number_bits: usize,
    ) -> anyhow::Result<Self> {
        let inner = searcher::Searcher::open_or_create_with_segments(path, segment_number_bits)
            .await
            .map_err(|error| anyhow::anyhow!("open search index {path}: {error}"))?;
        Ok(Self { inner })
    }

    #[must_use]
    pub fn was_recreated(&self) -> bool {
        self.inner.was_recreated()
    }

    pub async fn close(&self) {
        self.inner.close().await;
    }

    pub async fn sync(&self, db: &Database, article_id: &str) -> anyhow::Result<()> {
        let documents = document::build_documents(db, article_id)?;
        self.inner.replace_article(article_id, documents).await?;
        Ok(())
    }

    pub async fn sync_user(&self, db: &Database, user_id: &str) -> anyhow::Result<u64> {
        let article_ids = db::article_ids_of_user(db, user_id)?;
        let mut batch = Vec::with_capacity(article_ids.len());
        for article_id in &article_ids {
            let documents = document::build_documents(db, article_id)?;
            batch.push((article_id.clone(), documents));
        }
        let replaced = self.inner.replace_articles(batch).await?;
        Ok(replaced as u64)
    }

    pub async fn sync_all(&self, db: &Database) -> anyhow::Result<u64> {
        let article_ids = db::all_article_ids(db)?;
        let mut batch = Vec::with_capacity(article_ids.len());
        let mut count = 0u64;
        for article_id in &article_ids {
            let documents = document::build_documents(db, article_id)?;
            count += documents.len() as u64;
            batch.push((article_id.clone(), documents));
        }
        self.inner.rebuild(batch).await?;
        Ok(count)
    }

    pub async fn read(
        &self,
        db: &Database,
        request: SearchRequest,
    ) -> anyhow::Result<SearchOutcome> {
        let fields: Vec<SearchField> = request
            .ranges
            .iter()
            .map(|range| search_field(*range))
            .collect();
        let outcome = self
            .inner
            .read(searcher::SearchRequest {
                query: request.query,
                fields,
                from_seconds: request.from_seconds,
                to_seconds: request.to_seconds,
                offset: usize::try_from(request.offset).unwrap_or(usize::MAX),
                limit: usize::try_from(request.limit).unwrap_or(usize::MAX),
            })
            .await?;

        let mut versions = Vec::new();
        let mut comments = Vec::new();
        for hit in outcome.hits {
            match hit {
                DocHit::Version(version) => versions.push(SearchVersionOutcome {
                    article_id: version.article_id,
                    version_id: version.version_id,
                    version_number: version.version_number,
                    title: version.title,
                    author_id: version.author_id,
                    author_name: version.author_name,
                    article_hits: version.article_hits.into_iter().map(hit_outcome).collect(),
                    version_hits: version.version_hits.into_iter().map(hit_outcome).collect(),
                    version_number_hit: version.version_number_hit,
                }),
                DocHit::Comment(comment) => comments.push(SearchCommentOutcome {
                    article_id: comment.article_id,
                    version_id: comment.version_id,
                    comment_id: comment.comment_id,
                    author_id: comment.author_id,
                    author_name: comment.author_name,
                    content: comment.content,
                    article_title: String::new(),
                    article_author_id: String::new(),
                    article_author_name: String::new(),
                    version_number: String::new(),
                }),
            }
        }

        db::enrich_comment_headers(db, &mut comments)?;

        let mut docs = Vec::with_capacity(versions.len() + comments.len());
        docs.extend(versions.into_iter().map(SearchDocOutcome::Version));
        docs.extend(comments.into_iter().map(SearchDocOutcome::Comment));
        Ok(SearchOutcome { docs })
    }
}

fn hit_outcome(hit: FieldHit) -> SearchHitOutcome {
    let snippet = if search_range(hit.field) == SearchRange::Tag {
        clean_tag_snippet(&hit.snippet)
    } else {
        hit.snippet
    };
    SearchHitOutcome {
        range: search_range(hit.field),
        snippet,
    }
}

fn clean_tag_snippet(raw: &str) -> String {
    let trimmed = raw.trim();
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(trimmed);
    inner
        .split(',')
        .map(|piece| piece.trim().trim_matches('"').to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

const fn search_field(range: SearchRange) -> SearchField {
    match range {
        SearchRange::Title => SearchField::Title,
        SearchRange::Summary => SearchField::Summary,
        SearchRange::AuthorName => SearchField::AuthorName,
        SearchRange::Comment => SearchField::Comment,
        SearchRange::Note => SearchField::Note,
        SearchRange::Tag => SearchField::Tag,
        SearchRange::VersionNumber => SearchField::VersionNumber,
        SearchRange::ArticleId => SearchField::ArticleId,
        SearchRange::VersionId => SearchField::VersionId,
        SearchRange::CommentId => SearchField::CommentId,
        SearchRange::AuthorId => SearchField::AuthorId,
        SearchRange::Role => SearchField::Role,
    }
}

const fn search_range(field: SearchField) -> SearchRange {
    match field {
        SearchField::Title => SearchRange::Title,
        SearchField::Summary => SearchRange::Summary,
        SearchField::AuthorName => SearchRange::AuthorName,
        SearchField::Comment => SearchRange::Comment,
        SearchField::Note => SearchRange::Note,
        SearchField::Tag => SearchRange::Tag,
        SearchField::VersionNumber => SearchRange::VersionNumber,
        SearchField::ArticleId => SearchRange::ArticleId,
        SearchField::VersionId => SearchRange::VersionId,
        SearchField::CommentId => SearchRange::CommentId,
        SearchField::AuthorId => SearchRange::AuthorId,
        SearchField::Role => SearchRange::Role,
    }
}

#[cfg(test)]
mod tests {
    use super::clean_tag_snippet;

    #[test]
    fn clean_tag_snippet_strips_the_json_array_shell() {
        assert_eq!(
            clean_tag_snippet("[\"<mark>rust</mark>\", \"search\"]"),
            "<mark>rust</mark> search"
        );
    }

    #[test]
    fn clean_tag_snippet_passes_plain_text_through() {
        assert_eq!(clean_tag_snippet("<mark>rust</mark>"), "<mark>rust</mark>");
    }
}
