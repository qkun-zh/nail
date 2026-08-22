use std::fs;
use std::path::PathBuf;

use crate::{Database, EdgeKind, Error, NodeId, NodeKind, Row, Value, ValueLookup};

fn memory_database() -> Database {
    Database::open_memory("nail_test", &[]).expect("open memory database")
}

fn mapped_path(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("database_test_{name}.agdb"));
    if path.exists() {
        fs::remove_file(&path).expect("remove stale test database file");
    }
    path
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UserRow {
    id: String,
    name: String,
    email_address_hash: String,
}

impl Row for UserRow {
    const KIND: NodeKind = NodeKind::User;

    fn business_id(&self) -> &str {
        &self.id
    }

    fn to_row(&self) -> Vec<(String, Value)> {
        vec![
            ("id".to_string(), Value::Text(self.id.clone())),
            ("name".to_string(), Value::Text(self.name.clone())),
            (
                "email_address_hash".to_string(),
                Value::Text(self.email_address_hash.clone()),
            ),
        ]
    }

    fn from_lookup(lookup: &dyn ValueLookup) -> Result<Self, Error> {
        Ok(Self {
            id: lookup.required_text("id")?,
            name: lookup.required_text("name")?,
            email_address_hash: lookup.required_text("email_address_hash")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArticleRow {
    id: String,
    title: String,
    latest_version_id: Option<String>,
}

impl Row for ArticleRow {
    const KIND: NodeKind = NodeKind::Article;

    fn business_id(&self) -> &str {
        &self.id
    }

    fn to_row(&self) -> Vec<(String, Value)> {
        let mut values = vec![
            ("id".to_string(), Value::Text(self.id.clone())),
            ("title".to_string(), Value::Text(self.title.clone())),
        ];
        if let Some(latest) = &self.latest_version_id {
            values.push(("latest_version_id".to_string(), Value::Text(latest.clone())));
        }
        values
    }

    fn from_lookup(lookup: &dyn ValueLookup) -> Result<Self, Error> {
        Ok(Self {
            id: lookup.required_text("id")?,
            title: lookup.required_text("title")?,
            latest_version_id: lookup.optional_text("latest_version_id")?,
        })
    }
}

fn user(id: &str, name: &str) -> UserRow {
    UserRow {
        id: id.to_string(),
        name: name.to_string(),
        email_address_hash: format!("hash-{id}"),
    }
}

#[test]
fn open_memory_is_ready_for_scopes() {
    let database = memory_database();
    let count = database
        .read(|r| r.all_nodes(NodeKind::User))
        .expect("list nodes")
        .len();
    assert_eq!(count, 0);
}

#[test]
fn open_mapped_roundtrip_persists_rows() {
    let path = mapped_path("roundtrip");
    let business_id = "018f0000-0000-7000-8000-000000000001";
    {
        let database = Database::open_mapped(&path, &[]).expect("open mapped database");
        database
            .write(|w| w.insert_node(&user(business_id, "alice")).map(|_| ()))
            .expect("insert node");
    }
    let reopened = Database::open_mapped(&path, &[]).expect("reopen mapped database");
    let node = reopened
        .read(|r| r.resolve(NodeKind::User, business_id))
        .expect("resolve")
        .expect("node present after reopen");
    let row = reopened
        .read(|r| r.read_node::<UserRow>(node))
        .expect("read node")
        .expect("row present");
    assert_eq!(row.name, "alice");
    fs::remove_file(&path).expect("clean up test database file");
}

#[test]
fn open_mapped_creates_missing_parent_directories() {
    let path = std::env::temp_dir()
        .join(format!("database_test_dir_{}", std::process::id()))
        .join("nested")
        .join("inner.agdb");
    if path.exists() {
        fs::remove_file(&path).expect("remove stale test database file");
    }
    let database = Database::open_mapped(&path, &[]).expect("open nested mapped database");
    database
        .read(|r| r.all_nodes(NodeKind::User))
        .expect("list nodes");
    fs::remove_file(&path).expect("clean up test database file");
}

#[test]
fn insert_then_resolve_then_read_roundtrip() {
    let database = memory_database();
    let business_id = "018f0000-0000-7000-8000-000000000002";
    let node = database
        .write(|w| w.insert_node(&user(business_id, "bob")))
        .expect("insert node");
    let resolved = database
        .read(|r| r.resolve(NodeKind::User, business_id))
        .expect("resolve")
        .expect("resolved after insert");
    assert_eq!(node, resolved);
    let row = database
        .read(|r| r.read_node::<UserRow>(node))
        .expect("read node")
        .expect("row present");
    assert_eq!(row, user(business_id, "bob"));
}

#[test]
fn resolve_missing_returns_none() {
    let database = memory_database();
    let resolved = database
        .read(|r| r.resolve(NodeKind::User, "does-not-exist"))
        .expect("resolve");
    assert_eq!(resolved, None);
}

#[test]
fn find_by_key_matches_indexed_values() {
    let database = Database::open_memory("nail_test_indexed", &["email_address_hash".to_string()])
        .expect("open indexed database");
    let row = user("018f0000-0000-7000-8000-0000000000a1", "indexed");
    let node = database
        .write(|w| w.insert_node(&row))
        .expect("insert node");
    let found = database
        .read(|r| r.find_by_key("email_address_hash", &row.email_address_hash))
        .expect("find by key");
    assert_eq!(found, Some(node));
    let missing = database
        .read(|r| r.find_by_key("email_address_hash", "absent"))
        .expect("find by key");
    assert_eq!(missing, None);
}

#[test]
fn upsert_reuses_node_and_updates_fields() {
    let database = memory_database();
    let business_id = "018f0000-0000-7000-8000-000000000003";
    let first = database
        .write(|w| w.insert_node(&user(business_id, "carol")))
        .expect("first insert");
    let second = database
        .write(|w| w.insert_node(&user(business_id, "carol-2")))
        .expect("second insert");
    assert_eq!(first, second);
    let row = database
        .read(|r| r.read_node::<UserRow>(second))
        .expect("read node")
        .expect("row present");
    assert_eq!(row.name, "carol-2");
}

#[test]
fn upsert_clears_keys_absent_from_new_row() {
    let database = memory_database();
    let business_id = "018f0000-0000-7000-8000-000000000004";
    let with_latest = ArticleRow {
        id: business_id.to_string(),
        title: "t".to_string(),
        latest_version_id: Some("v1".to_string()),
    };
    let node = database
        .write(|w| w.insert_node(&with_latest))
        .expect("insert article");
    let stored = database
        .read(|r| r.read_node::<ArticleRow>(node))
        .expect("read node")
        .expect("row present");
    assert_eq!(stored.latest_version_id.as_deref(), Some("v1"));
    let without_latest = ArticleRow {
        id: business_id.to_string(),
        title: "t".to_string(),
        latest_version_id: None,
    };
    database
        .write(|w| w.insert_node(&without_latest).map(|_| ()))
        .expect("upsert article");
    let cleared = database
        .read(|r| r.read_node::<ArticleRow>(node))
        .expect("read node")
        .expect("row present");
    assert_eq!(cleared.latest_version_id, None);
}

#[test]
fn read_nodes_strict_on_missing_ids() {
    let database = memory_database();
    let node = database
        .write(|w| w.insert_node(&user("018f0000-0000-7000-8000-000000000005", "dave")))
        .expect("insert node");
    let error = database
        .read(|r| r.read_nodes::<UserRow>(&[node, NodeId::from_raw(999_999)]))
        .expect_err("missing id must error");
    assert!(matches!(error, Error::NotFound { .. }));
}

#[test]
fn read_nodes_empty_slice_returns_empty() {
    let database = memory_database();
    let rows = database
        .read(|r| r.read_nodes::<UserRow>(&[]))
        .expect("read nodes");
    assert!(rows.is_empty());
}

#[test]
fn edges_connect_navigate_and_count() {
    let database = memory_database();
    let author = database
        .write(|w| w.insert_node(&user("018f0000-0000-7000-8000-000000000006", "erin")))
        .expect("insert author");
    let article = database
        .write(|w| {
            w.insert_node(&ArticleRow {
                id: "018f0000-0000-7000-8000-000000000007".to_string(),
                title: "hello".to_string(),
                latest_version_id: None,
            })
        })
        .expect("insert article");
    database
        .write(|w| {
            w.insert_edge(
                NodeKind::User,
                author,
                EdgeKind::UserAuthorArticle,
                NodeKind::Article,
                article,
            )
        })
        .expect("insert edge");
    database
        .write(|w| {
            w.insert_edge(
                NodeKind::User,
                author,
                EdgeKind::UserAuthorArticle,
                NodeKind::Article,
                article,
            )
        })
        .expect("duplicate edge insert is idempotent");

    let outgoing = database
        .read(|r| r.outgoing(author, EdgeKind::UserAuthorArticle))
        .expect("outgoing");
    assert_eq!(outgoing, vec![article]);
    let incoming = database
        .read(|r| r.incoming(article, EdgeKind::UserAuthorArticle))
        .expect("incoming");
    assert_eq!(incoming, vec![author]);
    let count = database
        .read(|r| r.count_outgoing(author, EdgeKind::UserAuthorArticle))
        .expect("count outgoing");
    assert_eq!(count, 1);
    let incoming_count = database
        .read(|r| r.count_incoming(article, EdgeKind::UserAuthorArticle))
        .expect("count incoming");
    assert_eq!(incoming_count, 1);
}

#[test]
fn remove_edge_removes_only_the_target_edge() {
    let database = memory_database();
    let (author, first, second) = database
        .write(|w| {
            let author = w.insert_node(&user("018f0000-0000-7000-8000-000000000008", "frank"))?;
            let first = w.insert_node(&ArticleRow {
                id: "018f0000-0000-7000-8000-000000000009".to_string(),
                title: "one".to_string(),
                latest_version_id: None,
            })?;
            let second = w.insert_node(&ArticleRow {
                id: "018f0000-0000-7000-8000-00000000000a".to_string(),
                title: "two".to_string(),
                latest_version_id: None,
            })?;
            w.insert_edge(
                NodeKind::User,
                author,
                EdgeKind::UserAuthorArticle,
                NodeKind::Article,
                first,
            )?;
            w.insert_edge(
                NodeKind::User,
                author,
                EdgeKind::UserAuthorArticle,
                NodeKind::Article,
                second,
            )?;
            Ok((author, first, second))
        })
        .expect("seed graph");
    database
        .write(|w| w.remove_edge(author, EdgeKind::UserAuthorArticle, first))
        .expect("remove edge");
    let remaining = database
        .read(|r| r.outgoing(author, EdgeKind::UserAuthorArticle))
        .expect("outgoing");
    assert_eq!(remaining, vec![second]);
    database
        .write(|w| w.remove_edge(author, EdgeKind::UserAuthorArticle, first))
        .expect("removing absent edge is a no-op");
}

#[test]
fn remove_node_cascades_attached_edges() {
    let database = memory_database();
    let (author, article) = database
        .write(|w| {
            let author = w.insert_node(&user("018f0000-0000-7000-8000-00000000000b", "grace"))?;
            let article = w.insert_node(&ArticleRow {
                id: "018f0000-0000-7000-8000-00000000000c".to_string(),
                title: "doomed".to_string(),
                latest_version_id: None,
            })?;
            w.insert_edge(
                NodeKind::User,
                author,
                EdgeKind::UserAuthorArticle,
                NodeKind::Article,
                article,
            )?;
            Ok((author, article))
        })
        .expect("seed graph");
    database
        .write(|w| w.remove(&[article]))
        .expect("remove article");
    let remaining = database
        .read(|r| r.outgoing(author, EdgeKind::UserAuthorArticle))
        .expect("outgoing after cascade");
    assert!(remaining.is_empty());
    let resolved = database
        .read(|r| r.resolve(NodeKind::Article, "018f0000-0000-7000-8000-00000000000c"))
        .expect("resolve removed node");
    assert_eq!(resolved, None);
}

#[test]
fn all_nodes_isolates_kinds() {
    let database = memory_database();
    database
        .write(|w| {
            w.insert_node(&user("018f0000-0000-7000-8000-00000000000f", "hana"))?;
            w.insert_node(&ArticleRow {
                id: "018f0000-0000-7000-8000-000000000010".to_string(),
                title: "isolation".to_string(),
                latest_version_id: None,
            })?;
            Ok(())
        })
        .expect("seed kinds");
    let users = database
        .read(|r| r.all_nodes(NodeKind::User))
        .expect("all users");
    let articles = database
        .read(|r| r.all_nodes(NodeKind::Article))
        .expect("all articles");
    assert_eq!(users.len(), 1);
    assert_eq!(articles.len(), 1);
}

#[test]
fn set_and_clear_key_roundtrip_values() {
    let database = memory_database();
    let node = database
        .write(|w| w.insert_node(&user("018f0000-0000-7000-8000-000000000011", "ivan")))
        .expect("insert node");
    database
        .write(|w| w.set_key(node, "name", Value::Text("ivan-2".to_string())))
        .expect("set key");
    let renamed = database
        .read(|r| r.read_node::<UserRow>(node))
        .expect("read node")
        .expect("row present");
    assert_eq!(renamed.name, "ivan-2");
    database
        .write(|w| w.clear_key(node, "name"))
        .expect("clear key");
    let error = database
        .read(|r| r.read_node::<UserRow>(node))
        .expect_err("cleared required key must surface as a row error");
    assert!(matches!(error, Error::Invalid(_)));
}

#[test]
fn write_scope_rolls_back_on_error() {
    let database = memory_database();
    let business_id = "018f0000-0000-7000-8000-000000000013";
    let error = database
        .write(|w| -> Result<(), Error> {
            w.insert_node(&user(business_id, "karl"))?;
            Err(Error::Invalid("boom".to_string()))
        })
        .expect_err("closure error must propagate");
    assert!(matches!(error, Error::Invalid(_)));
    let resolved = database
        .read(|r| r.resolve(NodeKind::User, business_id))
        .expect("resolve after rollback");
    assert_eq!(resolved, None);
}

#[test]
fn insert_edge_with_missing_endpoint_errors() {
    let database = memory_database();
    let error = database
        .write(|w| {
            w.insert_edge(
                NodeKind::User,
                NodeId::from_raw(999_999),
                EdgeKind::UserAuthorArticle,
                NodeKind::Article,
                NodeId::from_raw(999_998),
            )
        })
        .expect_err("edge to missing nodes must error");
    assert!(
        matches!(
            error,
            Error::NotFound {
                kind: NodeKind::User,
                ..
            }
        ),
        "unexpected error: {error:?}"
    );
}

#[test]
fn open_mapped_ensures_indexes_idempotently() {
    let path = mapped_path("indexes");
    let indexes = ["email_address_hash".to_string()];
    {
        let database = Database::open_mapped(&path, &indexes).expect("open with indexes");
        database
            .write(|w| w.insert_node(&user("018f0000-0000-7000-8000-000000000014", "kate")))
            .expect("insert indexed node");
    }
    let reopened = Database::open_mapped(&path, &indexes).expect("reopen with same indexes");
    let count = reopened
        .read(|r| r.all_nodes(NodeKind::User))
        .expect("list after reopen")
        .len();
    assert_eq!(count, 1);
    fs::remove_file(&path).expect("clean up test database file");
}
