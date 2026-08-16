use super::context::{build_state, test_config};

use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::graph::resolve_node_id_sync;
use crate::repository::schema::{
    EDGE_ARTICLE_APPLY_TAG, EDGE_USER_AUTHOR_ARTICLE, ENTITY_TYPE_ARTICLE, KEY_TYPE,
};
use crate::repository::version::VersionDraft;
use agdb::QueryBuilder;

fn pdf_hash(seed: u8) -> String {
    format!("{seed:x}").repeat(32)
}

async fn create_user(state: &crate::infrastructure::state::AppState, email: &str) -> String {
    crate::repository::user::create_user(&state.graph, &nail_common::hash::email(email))
        .await
        .expect("user")
}

async fn make_article(
    state: &crate::infrastructure::state::AppState,
    author_id: &str,
    title: &str,
    hash: &str,
) -> String {
    let article_id = uuid::Uuid::now_v7().to_string();
    create_article(
        &state.graph,
        &ArticleDraft {
            article_id: article_id.clone(),
            author_id: author_id.to_string(),
            title: title.to_string(),
            summary: "a summary".to_string(),
            tags: vec!["rust".to_string()],
            first_version: VersionDraft {
                version_id: uuid::Uuid::now_v7().to_string(),
                version_number: "1.0.0".to_string(),
                content_hash: hash.to_string(),
                note: "initial note".to_string(),
            },
        },
    )
    .await
    .expect("create article");
    article_id
}

#[tokio::test]
async fn probe_targeted_queries_localize_by_endpoint() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author = create_user(&state, "probe-author@example.com").await;
    let a1 = make_article(&state, &author, "alpha", &pdf_hash(1)).await;
    let a2 = make_article(&state, &author, "beta", &pdf_hash(2)).await;

    let guard = state.graph.read().await;
    let a1_node = resolve_node_id_sync(&guard, ENTITY_TYPE_ARTICLE, &a1)
        .expect("resolve")
        .expect("article a1");
    let a2_node = resolve_node_id_sync(&guard, ENTITY_TYPE_ARTICLE, &a2)
        .expect("resolve")
        .expect("article a2");

    // Current behavior: scan ALL owner edges in the graph, then filter to a1.
    let all_owner = guard
        .exec(
            QueryBuilder::search()
                .elements()
                .where_()
                .key(KEY_TYPE)
                .value(EDGE_USER_AUTHOR_ARTICLE)
                .query(),
        )
        .expect("all owner edges");
    let mut current_owner_ids: Vec<agdb::DbId> = all_owner
        .elements
        .iter()
        .filter(|e| e.to == a1_node)
        .map(|e| e.id)
        .collect();
    current_owner_ids.sort();

    // Current behavior: scan ALL tag edges, then filter to a1.
    let all_tag = guard
        .exec(
            QueryBuilder::search()
                .elements()
                .where_()
                .key(KEY_TYPE)
                .value(EDGE_ARTICLE_APPLY_TAG)
                .query(),
        )
        .expect("all tag edges");
    let mut current_tag_ids: Vec<agdb::DbId> = all_tag
        .elements
        .iter()
        .filter(|e| e.from == a1_node)
        .map(|e| e.id)
        .collect();
    current_tag_ids.sort();

    // Candidate (localized): targeted .to(a1) / .from(a1) returns only a1's edges.
    let targeted_owner = guard
        .exec(
            QueryBuilder::search()
                .to(a1_node)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(KEY_TYPE)
                .value(EDGE_USER_AUTHOR_ARTICLE)
                .query(),
        )
        .expect("targeted owner edges");
    let mut targeted_owner_ids: Vec<agdb::DbId> =
        targeted_owner.elements.iter().map(|e| e.id).collect();
    targeted_owner_ids.sort();

    let targeted_tag = guard
        .exec(
            QueryBuilder::search()
                .from(a1_node)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(KEY_TYPE)
                .value(EDGE_ARTICLE_APPLY_TAG)
                .query(),
        )
        .expect("targeted tag edges");
    let mut targeted_tag_ids: Vec<agdb::DbId> =
        targeted_tag.elements.iter().map(|e| e.id).collect();
    targeted_tag_ids.sort();

    eprintln!(
        "PROBE graph_owner_edges={} graph_tag_edges={} current_filtered_owner_a1={} current_filtered_tag_a1={} targeted_owner_a1={} targeted_tag_a1={}",
        all_owner.elements.len(),
        all_tag.elements.len(),
        current_owner_ids.len(),
        current_tag_ids.len(),
        targeted_owner_ids.len(),
        targeted_tag_ids.len(),
    );

    // Behavior preservation: the targeted query must return EXACTLY the same edge
    // ids that the current scan+filter produces for a1.
    assert_eq!(current_owner_ids, targeted_owner_ids);
    assert_eq!(current_tag_ids, targeted_tag_ids);
    assert_eq!(
        all_owner.elements.len(),
        2,
        "two articles => two owner edges"
    );
    assert_eq!(all_tag.elements.len(), 2, "two articles => two tag edges");
    assert!(a2_node != a1_node);
}
