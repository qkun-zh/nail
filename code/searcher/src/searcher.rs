use std::fs;
use std::path::Path;

use seekstorm::commit::Commit;
use seekstorm::index::{
    Close, DeleteDocuments, Document, IndexArc, IndexDocuments, create_index, open_index,
};
use seekstorm::search::{FacetFilter, QueryRewriting, QueryType, ResultType, Search, SearchMode};

use crate::doc::SearchDoc;
use crate::error::Error;
use crate::schema;

pub const DEFAULT_SEGMENT_NUMBER_BITS: usize = 11;
const REBUILD_COMMIT_CHUNK: usize = 1000;
const ARTICLE_PAGE_SIZE: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(test)]
pub struct Stats {
    pub indexed: usize,
    pub live: usize,
    pub deleted: usize,
}

#[derive(Clone)]
pub struct Searcher {
    pub(crate) index: IndexArc,
    recreated: bool,
}

impl Searcher {
    pub async fn open_or_create(path: &str) -> Result<Self, Error> {
        Self::open_or_create_with_segments(path, DEFAULT_SEGMENT_NUMBER_BITS).await
    }

    pub async fn open_or_create_with_segments(
        path: &str,
        segment_number_bits: usize,
    ) -> Result<Self, Error> {
        let index_path = Path::new(path);
        let mut recreated = false;
        if index_path.exists()
            && schema::read_marker(index_path).as_deref() != Some(schema::SCHEMA_VERSION)
        {
            fs::remove_dir_all(index_path)?;
            recreated = true;
        }
        let index = if recreated || !index_path.exists() {
            let index = create_index(
                index_path,
                schema::meta(),
                &schema::fields(),
                &Vec::new(),
                segment_number_bits,
                true,
                None,
            )
            .await
            .map_err(|error| Error::Engine(format!("create index failed: {error}")))?;
            schema::write_marker(index_path)?;
            recreated = true;
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

    pub async fn replace_article(
        &self,
        article_id: &str,
        documents: Vec<SearchDoc>,
    ) -> Result<(), Error> {
        self.replace_articles(vec![(article_id.to_string(), documents)])
            .await?;
        Ok(())
    }

    pub async fn replace_articles(
        &self,
        batch: Vec<(String, Vec<SearchDoc>)>,
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
            .map(SearchDoc::to_document)
            .collect::<Result<_, _>>()?;
        if !fresh.is_empty() {
            self.index.index_documents(fresh).await;
            changed = true;
        }

        if changed {
            self.index.commit().await;
        }
        Ok(batch.len())
    }

    pub async fn rebuild(
        &self,
        articles: impl IntoIterator<Item = (String, Vec<SearchDoc>)>,
    ) -> Result<usize, Error> {
        {
            let mut guard = self.index.write().await;
            guard.clear_index().await;
        }
        let mut indexed_count = 0usize;
        let mut chunk: Vec<Document> = Vec::new();
        for (_, documents) in articles {
            indexed_count += documents.len();
            let documents = documents
                .iter()
                .map(SearchDoc::to_document)
                .collect::<Result<Vec<_>, _>>()?;
            chunk.extend(documents);
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

    #[cfg(test)]
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
        let mut doc_ids = Vec::new();
        loop {
            let result = self
                .index
                .search(
                    String::new(),
                    None,
                    QueryType::Union,
                    SearchMode::Lexical,
                    true,
                    doc_ids.len(),
                    ARTICLE_PAGE_SIZE,
                    ResultType::TopkCount,
                    false,
                    Vec::new(),
                    Vec::new(),
                    facet_filter.clone(),
                    Vec::new(),
                    QueryRewriting::SearchOnly,
                )
                .await;
            let page: Vec<usize> = result.results.into_iter().map(|hit| hit.doc_id).collect();
            let complete = page.len() < ARTICLE_PAGE_SIZE;
            doc_ids.extend(page);
            if complete {
                return doc_ids;
            }
        }
    }
}
