use std::fs;
use std::path::PathBuf;

use crate::doc::{CommentDoc, IndexDoc, VersionDoc};
use crate::index::Searcher;

pub fn scratch_dir(label: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("searcher_ix_{}_{}", std::process::id(), label));
    let _ = fs::remove_dir_all(&directory);
    directory
}

pub fn version_doc(article_id: &str, version_id: &str, title: &str) -> IndexDoc {
    IndexDoc::Version(VersionDoc {
        version_id: version_id.to_string(),
        article_id: article_id.to_string(),
        version_number: "1".to_string(),
        title: title.to_string(),
        summary: format!("{title} summary"),
        author_name: "alice".to_string(),
        author_id: "u-1".to_string(),
        role: "author".to_string(),
        note: String::new(),
        tags: vec!["rust".to_string()],
        ts: 1_700_000_000,
    })
}

pub fn comment_doc(article_id: &str, comment_id: &str, content: &str) -> IndexDoc {
    IndexDoc::Comment(CommentDoc {
        comment_id: comment_id.to_string(),
        version_id: format!("v-{article_id}"),
        article_id: article_id.to_string(),
        author_name: "bob".to_string(),
        author_id: "u-2".to_string(),
        role: "reviewer".to_string(),
        content: content.to_string(),
        ts: 1_700_000_100,
    })
}

pub async fn fresh_index(label: &str) -> (Searcher, PathBuf) {
    let path = scratch_dir(label);
    let index = Searcher::open_or_create(path.to_str().unwrap())
        .await
        .unwrap();
    (index, path)
}
