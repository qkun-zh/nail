use std::fs;
use std::path::Path;

use seekstorm::commit::Commit;
use seekstorm::index::{
    Close, DeleteDocuments, Document, IndexArc, IndexDocuments, create_index, open_index,
};
use seekstorm::search::{FacetFilter, QueryRewriting, QueryType, ResultType, Search, SearchMode};

use crate::doc::IndexDoc;
use crate::error::Error;
use crate::schema;

const SEGMENT_NUMBER_BITS: usize = 11;
const REBUILD_COMMIT_CHUNK: usize = 1000;
const ARTICLE_SCAN_LIMIT: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub indexed: usize,
    pub live: usize,
    pub deleted: usize,
}

pub struct SearchIndex {
    index: IndexArc,
    recreated: bool,
}

impl SearchIndex {
    /// Opens the index at `path`, or creates it when absent.
    ///
    /// A directory with a corrupt payload or a stale schema marker is wiped
    /// and recreated empty; [`Self::was_recreated`] then reports true so the
    /// caller can reseed the content.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] for filesystem failures and [`Error::Engine`]
    /// when the engine cannot create or open the index.
    pub async fn open_or_create(path: &str) -> Result<Self, Error> {
        let index_path = Path::new(path);
        let mut recreated = false;
        if index_path.exists() {
            let healthy = schema::validate_dir(index_path).is_ok()
                && schema::read_marker(index_path).as_deref() == Some(schema::SCHEMA_VERSION);
            if !healthy {
                fs::remove_dir_all(index_path)?;
                recreated = true;
            }
        }
        let index = if recreated || !index_path.exists() {
            let index = create_index(
                index_path,
                schema::meta(),
                &schema::fields(),
                &Vec::new(),
                SEGMENT_NUMBER_BITS,
                true,
                None,
            )
            .await
            .map_err(|error| Error::Engine(format!("create index failed: {error}")))?;
            schema::write_marker(index_path)?;
            index
        } else {
            open_index(index_path)
                .await
                .map_err(|error| Error::Engine(format!("open index failed: {error}")))?
        };
        Ok(Self { index, recreated })
    }

    #[must_use]
    pub fn was_recreated(&self) -> bool {
        self.recreated
    }

    /// Atomically swaps the complete document set of one article.
    ///
    /// An empty document list removes the article from the index. A no-op
    /// (unknown article, empty list) skips deletion and commit entirely.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Engine`] when any document carries an article id
    /// different from `article_id`; nothing is written in that case.
    pub async fn replace_article(
        &self,
        article_id: &str,
        documents: Vec<IndexDoc>,
    ) -> Result<(), Error> {
        self.replace_articles(vec![(article_id.to_string(), documents)])
            .await?;
        Ok(())
    }

    /// Atomically swaps the document sets of many articles with a single
    /// commit, amortizing the per-commit cost across the whole batch.
    ///
    /// Returns the number of articles processed.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Engine`] when any document's own article id differs
    /// from the key it is filed under; nothing is written in that case.
    pub async fn replace_articles(
        &self,
        batch: Vec<(String, Vec<IndexDoc>)>,
    ) -> Result<usize, Error> {
        for (article_id, documents) in &batch {
            for document in documents {
                if document.article_id() != article_id {
                    return Err(Error::Engine(format!(
                        "document carries article id {} but is filed under {article_id}",
                        document.article_id()
                    )));
                }
            }
        }

        let mut stale_ids = Vec::new();
        for (article_id, _) in &batch {
            stale_ids.extend(self.find_article_doc_ids(article_id).await);
        }

        let mut changed = false;
        if !stale_ids.is_empty() {
            self.index
                .delete_documents(stale_ids.iter().map(|id| *id as u64).collect())
                .await;
            changed = true;
        }

        let fresh: Vec<Document> = batch
            .iter()
            .flat_map(|(_, documents)| documents.iter())
            .map(IndexDoc::to_document)
            .collect();
        if !fresh.is_empty() {
            self.index.index_documents(fresh).await;
            changed = true;
        }

        if changed {
            self.index.commit().await;
        }
        Ok(batch.len())
    }

    /// Wipes the index (including tombstones) and indexes the given article
    /// set from scratch, committing in bounded chunks.
    ///
    /// Returns the number of documents indexed.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Engine`] when any document's own article id differs
    /// from the key it is filed under; nothing is written in that case.
    pub async fn rebuild(
        &self,
        articles: impl IntoIterator<Item = (String, Vec<IndexDoc>)>,
    ) -> Result<usize, Error> {
        {
            let mut guard = self.index.write().await;
            guard.clear_index().await;
        }
        let mut indexed_count = 0usize;
        let mut chunk: Vec<Document> = Vec::new();
        for (_, documents) in articles {
            indexed_count += documents.len();
            chunk.extend(documents.iter().map(IndexDoc::to_document));
            if chunk.len() >= REBUILD_COMMIT_CHUNK {
                self.index.index_documents(chunk).await;
                self.index.commit().await;
                chunk = Vec::new();
            }
        }
        if !chunk.is_empty() {
            self.index.index_documents(chunk).await;
        }
        self.index.commit().await;
        Ok(indexed_count)
    }

    pub async fn stats(&self) -> Stats {
        let guard = self.index.read().await;
        let indexed = guard.indexed_doc_count().await;
        let live = guard.current_doc_count().await;
        Stats {
            indexed,
            live,
            deleted: indexed.saturating_sub(live),
        }
    }

    pub async fn close(&self) {
        self.index.close().await;
    }

    async fn find_article_doc_ids(&self, article_id: &str) -> Vec<usize> {
        let facet_filter = vec![FacetFilter::String32 {
            field: "article_id".to_string(),
            filter: vec![article_id.to_string()],
        }];
        let result = self
            .index
            .search(
                String::new(),
                None,
                QueryType::Union,
                SearchMode::Lexical,
                true,
                0,
                ARTICLE_SCAN_LIMIT,
                ResultType::TopkCount,
                false,
                Vec::new(),
                Vec::new(),
                facet_filter,
                Vec::new(),
                QueryRewriting::SearchOnly,
            )
            .await;
        result.results.into_iter().map(|hit| hit.doc_id).collect()
    }
}
