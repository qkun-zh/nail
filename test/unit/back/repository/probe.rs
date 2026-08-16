use super::context::{build_state, test_config};

use crate::repository::article::{ArticleDraft, create_article};
use crate::repository::graph::resolve_node_id_sync;
use crate::repository::schema::{
    ArticleRow, EDGE_ARTICLE_APPLY_TAG, EDGE_ARTICLE_HOLD_VERSION, EDGE_USER_AUTHOR_ARTICLE,
    ENTITY_TYPE_ARTICLE, ENTITY_TYPE_USER, ENTITY_TYPE_VERSION, KEY_TYPE, UserRow, VersionRow,
};
use crate::repository::version::{VersionDraft, create_version};
use agdb::QueryBuilder;
use std::collections::{HashMap, HashSet};

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

#[tokio::test]
async fn probe_batch_comment_enrichment_matches_per_comment() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author = create_user(&state, "probe-p3-author@example.com").await;

    // Two articles, each with a known first version (article_id, version_id) pair.
    let (a1, v1) = {
        let article_id = uuid::Uuid::now_v7().to_string();
        let version_id = uuid::Uuid::now_v7().to_string();
        create_article(
            &state.graph,
            &ArticleDraft {
                article_id: article_id.clone(),
                author_id: author.clone(),
                title: "p3 alpha".to_string(),
                summary: "s".to_string(),
                tags: vec!["rust".to_string()],
                first_version: VersionDraft {
                    version_id: version_id.clone(),
                    version_number: "1.0.0".to_string(),
                    content_hash: pdf_hash(1),
                    note: "n".to_string(),
                },
            },
        )
        .await
        .expect("create article");
        (article_id, version_id)
    };
    let (a2, v2) = {
        let article_id = uuid::Uuid::now_v7().to_string();
        let version_id = uuid::Uuid::now_v7().to_string();
        create_article(
            &state.graph,
            &ArticleDraft {
                article_id: article_id.clone(),
                author_id: author.clone(),
                title: "p3 beta".to_string(),
                summary: "s".to_string(),
                tags: vec!["rust".to_string()],
                first_version: VersionDraft {
                    version_id: version_id.clone(),
                    version_number: "2.0.0".to_string(),
                    content_hash: pdf_hash(2),
                    note: "n".to_string(),
                },
            },
        )
        .await
        .expect("create article");
        (article_id, version_id)
    };

    // Comment hits are (article_id, version_id) pairs. Include a duplicate to show the
    // batch path resolves each distinct id only once.
    let comments: Vec<(&str, &str)> = vec![
        (&a1, &v1),
        (&a1, &v1),
        (&a2, &v2),
        (&a1, &v2), // cross-reference: same article, different version
    ];

    let guard = state.graph.read().await;

    // --- Current per-comment path (mirrors read_article_title/author/version_number) ---
    let current = |article_id: &str, version_id: &str| {
        let article_node = resolve_node_id_sync(&guard, ENTITY_TYPE_ARTICLE, article_id)
            .unwrap()
            .unwrap();
        let title = crate::repository::graph::read_rows_sync::<ArticleRow>(
            &guard,
            std::slice::from_ref(&article_node),
        )
        .unwrap()
        .into_iter()
        .next()
        .map(|row| row.title)
        .unwrap_or_default();
        let edges = guard
            .exec(
                QueryBuilder::search()
                    .to(article_node)
                    .where_()
                    .distance(agdb::CountComparison::Equal(1))
                    .and()
                    .edge()
                    .and()
                    .key(KEY_TYPE)
                    .value(EDGE_USER_AUTHOR_ARTICLE)
                    .query(),
            )
            .unwrap();
        let author = edges
            .elements
            .first()
            .and_then(|edge| {
                crate::repository::graph::read_rows_sync::<UserRow>(
                    &guard,
                    std::slice::from_ref(&edge.from),
                )
                .ok()
                .and_then(|rows| rows.into_iter().next())
                .map(|row| row.name)
            })
            .unwrap_or_default();
        let version_node = resolve_node_id_sync(&guard, ENTITY_TYPE_VERSION, version_id)
            .unwrap()
            .unwrap();
        let version_number = crate::repository::graph::read_rows_sync::<VersionRow>(
            &guard,
            std::slice::from_ref(&version_node),
        )
        .unwrap()
        .into_iter()
        .next()
        .map(|row| row.version_number)
        .unwrap_or_default();
        (title, author, version_number)
    };

    // --- Candidate batch path: resolve each distinct id once, batch-read rows ---
    let distinct_article_ids: HashSet<&str> = comments.iter().map(|(a, _)| *a).collect();
    let distinct_version_ids: HashSet<&str> = comments.iter().map(|(_, v)| *v).collect();

    let mut article_by_id: HashMap<&str, agdb::DbId> = HashMap::new();
    for id in &distinct_article_ids {
        article_by_id.insert(
            id,
            resolve_node_id_sync(&guard, ENTITY_TYPE_ARTICLE, id)
                .unwrap()
                .unwrap(),
        );
    }
    let mut version_by_id: HashMap<&str, agdb::DbId> = HashMap::new();
    for id in &distinct_version_ids {
        version_by_id.insert(
            id,
            resolve_node_id_sync(&guard, ENTITY_TYPE_VERSION, id)
                .unwrap()
                .unwrap(),
        );
    }

    let article_nodes: Vec<agdb::DbId> = article_by_id.values().copied().collect();
    let article_titles: HashMap<agdb::DbId, String> =
        crate::repository::graph::read_rows_sync::<ArticleRow>(&guard, &article_nodes)
            .unwrap()
            .into_iter()
            .filter_map(|row| row.db_id.map(|node| (node, row.title)))
            .collect();

    let version_nodes: Vec<agdb::DbId> = version_by_id.values().copied().collect();
    let version_numbers: HashMap<agdb::DbId, String> =
        crate::repository::graph::read_rows_sync::<VersionRow>(&guard, &version_nodes)
            .unwrap()
            .into_iter()
            .filter_map(|row| row.db_id.map(|node| (node, row.version_number)))
            .collect();

    // Authors: one targeted edge query per distinct article, then batch-read the users.
    let mut author_user_nodes: HashSet<agdb::DbId> = HashSet::new();
    let mut author_by_article: HashMap<agdb::DbId, agdb::DbId> = HashMap::new();
    for (_, article_node) in &article_by_id {
        let edges = guard
            .exec(
                QueryBuilder::search()
                    .to(*article_node)
                    .where_()
                    .distance(agdb::CountComparison::Equal(1))
                    .and()
                    .edge()
                    .and()
                    .key(KEY_TYPE)
                    .value(EDGE_USER_AUTHOR_ARTICLE)
                    .query(),
            )
            .unwrap();
        if let Some(edge) = edges.elements.first() {
            author_by_article.insert(*article_node, edge.from);
            author_user_nodes.insert(edge.from);
        }
    }
    let user_nodes: Vec<agdb::DbId> = author_user_nodes.into_iter().collect();
    let user_names: HashMap<agdb::DbId, String> =
        crate::repository::graph::read_rows_sync::<UserRow>(&guard, &user_nodes)
            .unwrap()
            .into_iter()
            .filter_map(|row| row.db_id.map(|node| (node, row.name)))
            .collect();

    for (article_id, version_id) in &comments {
        let article_node = article_by_id[article_id];
        let batch = (
            article_titles[&article_node].clone(),
            author_by_article
                .get(&article_node)
                .and_then(|user_node| user_names.get(user_node))
                .cloned()
                .unwrap_or_default(),
            version_numbers[&version_by_id[version_id]].clone(),
        );
        let per_comment = current(article_id, version_id);
        assert_eq!(
            batch, per_comment,
            "batch enrichment must match per-comment for {article_id}/{version_id}"
        );
    }

    eprintln!(
        "PROBE comments={} distinct_articles={} distinct_versions={} distinct_authors={}",
        comments.len(),
        article_by_id.len(),
        version_by_id.len(),
        user_names.len(),
    );
}

#[tokio::test]
async fn probe_recycler_selection_hashset_matches_vec_exclude() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let r1 = create_user(&state, "probe-r1@example.com").await;
    let r2 = create_user(&state, "probe-r2@example.com").await;
    let r3 = create_user(&state, "probe-r3@example.com").await;
    // Distinct workload totals: r1 owns 2 articles, r2 owns 1, r3 owns 0.
    for i in 0..2u8 {
        make_article(&state, &r1, &format!("r1-{i}"), &pdf_hash(10 + i)).await;
    }
    make_article(&state, &r2, "r2-0", &pdf_hash(20)).await;

    let candidates = vec![r1.clone(), r2.clone(), r3.clone()];
    // Duplicate entries plus entries absent from the list stress the dedup behavior.
    let exclude: Vec<String> = vec![r1.clone(), r1.clone(), r3.clone()];

    let guard = state.graph.read().await;
    let total_of = |user_id: &str| -> u64 {
        let node = resolve_node_id_sync(&guard, ENTITY_TYPE_USER, user_id)
            .unwrap()
            .unwrap();
        guard
            .exec(
                QueryBuilder::search()
                    .from(node)
                    .where_()
                    .distance(agdb::CountComparison::Equal(1))
                    .and()
                    .edge()
                    .and()
                    .key(KEY_TYPE)
                    .value(EDGE_USER_AUTHOR_ARTICLE)
                    .query(),
            )
            .unwrap()
            .elements
            .len() as u64
    };

    // Replicates pick_recycler_target's loop; the only variable is how exclusions are tested.
    let select = |recyclers: &[String], is_excluded: &dyn Fn(&str) -> bool| -> Option<String> {
        let mut best: Option<(String, u64)> = None;
        for user_id in recyclers {
            if is_excluded(user_id) {
                continue;
            }
            let total = total_of(user_id);
            let better = match &best {
                None => true,
                Some((best_id, best_total)) => {
                    total < *best_total || (total == *best_total && *user_id > *best_id)
                }
            };
            if better {
                best = Some((user_id.clone(), total));
            }
        }
        best.map(|(id, _)| id)
    };

    let vec_best = select(&candidates, &|id| exclude.iter().any(|x| x.as_str() == id));
    let exclude_set: HashSet<String> = exclude.iter().cloned().collect();
    let set_best = select(&candidates, &|id| exclude_set.contains(id));

    // Membership equivalence over every candidate: the only behavioral difference.
    for id in &candidates {
        assert_eq!(exclude.contains(id), exclude_set.contains(id));
    }
    assert_eq!(vec_best, set_best);
    assert_eq!(set_best.as_deref(), Some(r2.as_str()));

    eprintln!(
        "PROBE candidates={} exclude_len={} distinct_exclude={} vec_best={:?} set_best={:?}",
        candidates.len(),
        exclude.len(),
        exclude_set.len(),
        vec_best,
        set_best,
    );
}

#[tokio::test]
async fn probe_offset_limit_pagination_tiles_default_order() {
    let (state, _) = build_state(&test_config(), 0).await.expect("state");
    let author = create_user(&state, "probe-p5-author@example.com").await;
    let article_id = {
        let article_id = uuid::Uuid::now_v7().to_string();
        create_article(
            &state.graph,
            &ArticleDraft {
                article_id: article_id.clone(),
                author_id: author.clone(),
                title: "p5 versions".to_string(),
                summary: "s".to_string(),
                tags: vec!["rust".to_string()],
                first_version: VersionDraft {
                    version_id: uuid::Uuid::now_v7().to_string(),
                    version_number: "1.0.0".to_string(),
                    content_hash: pdf_hash(30),
                    note: "n".to_string(),
                },
            },
        )
        .await
        .expect("create article");
        article_id
    };
    // Add 3 more versions; semver must strictly increase and content hashes be unique.
    for (i, version_number) in ["1.0.1", "1.0.2", "1.0.3"].iter().enumerate() {
        create_version(
            &state.graph,
            &article_id,
            &VersionDraft {
                version_id: uuid::Uuid::now_v7().to_string(),
                version_number: version_number.to_string(),
                content_hash: pdf_hash(40 + i as u8),
                note: "n".to_string(),
            },
        )
        .await
        .expect("create version");
    }

    let guard = state.graph.read().await;
    let article = resolve_node_id_sync(&guard, ENTITY_TYPE_ARTICLE, &article_id)
        .unwrap()
        .unwrap();

    let version_edges = |offset: u64, limit: u64| -> Vec<agdb::DbId> {
        guard
            .exec(
                QueryBuilder::search()
                    .from(article)
                    .offset(offset)
                    .limit(limit)
                    .where_()
                    .distance(agdb::CountComparison::Equal(1))
                    .and()
                    .edge()
                    .and()
                    .key(KEY_TYPE)
                    .value(EDGE_ARTICLE_HOLD_VERSION)
                    .query(),
            )
            .unwrap()
            .elements
            .iter()
            .map(|edge| edge.to)
            .collect()
    };

    // Full default (storage) order, no limit/offset.
    let all_edges = guard
        .exec(
            QueryBuilder::search()
                .from(article)
                .where_()
                .distance(agdb::CountComparison::Equal(1))
                .and()
                .edge()
                .and()
                .key(KEY_TYPE)
                .value(EDGE_ARTICLE_HOLD_VERSION)
                .query(),
        )
        .unwrap();
    let all_ids: Vec<agdb::DbId> = all_edges.elements.iter().map(|edge| edge.to).collect();
    assert_eq!(all_ids.len(), 4, "first version + 3 added = 4 versions");

    // Page through with offset/limit; must tile the full set, no gaps, no overlaps.
    let limit = 2u64;
    let mut paged: Vec<agdb::DbId> = Vec::new();
    let mut offset = 0u64;
    loop {
        let page = version_edges(offset, limit);
        assert!(page.len() as u64 <= limit, "page never exceeds limit");
        if page.is_empty() {
            break;
        }
        let peek = version_edges(offset, limit + 1);
        let has_next = peek.len() as u64 > page.len() as u64;
        let before = paged.len();
        paged.extend(page.clone());
        // No duplicate ids within or across pages.
        let unique: std::collections::HashSet<&agdb::DbId> = paged.iter().collect();
        assert_eq!(unique.len(), paged.len(), "no duplicate ids across pages");
        assert_eq!(paged.len() - before, page.len(), "page appended in order");
        offset += page.len() as u64;
        if !has_next {
            break;
        }
    }

    // The paged union equals the full default-order list (identical order).
    assert_eq!(
        all_ids, paged,
        "offset/limit pages tile the default-order full set"
    );

    eprintln!(
        "PROBE versions={} paged={} limit={} tiled_in_default_order=true",
        all_ids.len(),
        paged.len(),
        limit,
    );
}
