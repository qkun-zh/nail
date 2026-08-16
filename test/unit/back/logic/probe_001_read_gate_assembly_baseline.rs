// B0 read-gate assembly baseline (authz-refactor plan, Phase B0): measures the
// per-request marginal cost that B1 adds to the hot read paths
// (`/article/read`, `/article/{id}/read`). Source: the single enforcement entry
// `logic/authorize.rs` = assembly (`repository/authorization.rs`: user→role→
// permission edge reads + resource chain reads) + `infrastructure/cedar::decide`.
// Policy 2 (read-open) permits any principal, so a member authorize on reads
// returns Allow today; the gate cost is identical to the B1 deny path.
// Acceptance question: "what does one more authorize cost on the hot read
// paths, and does it justify B1's coarse `Virtual::"read"` desk for collection
// reads (1× principal assembly) instead of per-item single-resource assembly?"

use std::time::{Duration, Instant};

use crate::repository::authorization::{Resource, assemble_principal};
use crate::repository::role::{PERMISSION_ARTICLE_READ, ROLE_MEMBER};

const ITERATIONS: usize = 100;
const WARMUP: usize = 10;

async fn mean_duration<F, Fut>(mut run: F) -> Duration
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    for _ in 0..WARMUP {
        run().await;
    }
    let mut total = Duration::ZERO;
    for _ in 0..ITERATIONS {
        let start = Instant::now();
        run().await;
        total += start.elapsed();
    }
    let denominator = u32::try_from(ITERATIONS).unwrap_or(1);
    total / denominator
}

fn report(metric: &str, mean: Duration) {
    eprintln!("probe_001 {metric}: mean {mean:?}");
}

#[tokio::test]
async fn probe_001_read_gate_assembly_baseline() {
    let (state, _) = super::context::build_state(&super::context::test_config(), 0)
        .await
        .expect("state");

    let hash = nail_common::hash::email("user-zero@example.com");
    let user_zero = crate::repository::user::read_user_by_email_address_hash(&state.graph, &hash)
        .await
        .expect("lookup")
        .expect("user zero");
    let alice = crate::repository::user::create_user(
        &state.graph,
        &nail_common::hash::email("alice@example.com"),
    )
    .await
    .expect("user");
    crate::repository::role::hold_role(&state.graph, &alice, ROLE_MEMBER)
        .await
        .expect("hold");

    let author = crate::repository::user::create_user(
        &state.graph,
        &nail_common::hash::email("author@example.com"),
    )
    .await
    .expect("author");
    let article_id = uuid::Uuid::now_v7().to_string();
    let version_id = uuid::Uuid::now_v7().to_string();
    crate::repository::article::create_article(
        &state.graph,
        &crate::repository::article::ArticleDraft {
            article_id: article_id.clone(),
            author_id: author.clone(),
            title: "benchmark article".to_string(),
            summary: "baseline probe".to_string(),
            tags: vec![],
            first_version: crate::repository::version::VersionDraft {
                version_id: version_id.clone(),
                version_number: "1.0.0".to_string(),
                content_hash: format!("{:032x}", 1),
                note: "baseline".to_string(),
            },
        },
    )
    .await
    .expect("article");
    let comment_id =
        crate::logic::comment::create_comment(&state, &user_zero, &version_id, "baseline comment")
            .await
            .expect("comment");

    let admin_principal = mean_duration(|| {
        let graph = state.graph.clone();
        let user = user_zero.clone();
        async move {
            let _ = assemble_principal(&graph, &user)
                .await
                .expect("assemble admin");
        }
    })
    .await;
    report("assemble_principal admin (27 grants)", admin_principal);

    let member_principal = mean_duration(|| {
        let graph = state.graph.clone();
        let user = alice.clone();
        async move {
            let _ = assemble_principal(&graph, &user)
                .await
                .expect("assemble member");
        }
    })
    .await;
    report("assemble_principal member (2 grants)", member_principal);

    let article_gate = mean_duration(|| {
        let state = state.clone();
        let actor = alice.clone();
        let id = article_id.clone();
        async move {
            crate::logic::authorize::authorize(
                &state,
                &actor,
                PERMISSION_ARTICLE_READ,
                &Resource::Article(id.clone()),
            )
            .await
            .expect("article read gate");
        }
    })
    .await;
    report("authorize Article::Read single-resource", article_gate);

    let version_gate = mean_duration(|| {
        let state = state.clone();
        let actor = alice.clone();
        let id = version_id.clone();
        async move {
            crate::logic::authorize::authorize(
                &state,
                &actor,
                PERMISSION_ARTICLE_READ,
                &Resource::Version(id.clone()),
            )
            .await
            .expect("version read gate");
        }
    })
    .await;
    report("authorize Article::Read on Version (chain)", version_gate);

    let comment_gate = mean_duration(|| {
        let state = state.clone();
        let actor = alice.clone();
        let id = comment_id.clone();
        async move {
            crate::logic::authorize::authorize(
                &state,
                &actor,
                PERMISSION_ARTICLE_READ,
                &Resource::Comment(id.clone()),
            )
            .await
            .expect("comment read gate");
        }
    })
    .await;
    report("authorize Article::Read on Comment (chain)", comment_gate);

    let coarse_desk = mean_duration(|| {
        let state = state.clone();
        let actor = alice.clone();
        async move {
            crate::logic::authorize::authorize(
                &state,
                &actor,
                PERMISSION_ARTICLE_READ,
                &Resource::Virtual("read".to_string()),
            )
            .await
            .expect("coarse desk gate");
        }
    })
    .await;
    report("authorize Article::Read on Virtual desk", coarse_desk);

    let hot_read = mean_duration(|| {
        let state = state.clone();
        let id = article_id.clone();
        async move {
            let _ = crate::logic::article::read_article(&state, &id)
                .await
                .expect("read");
        }
    })
    .await;
    report("logic read_article (session-only body)", hot_read);

    let single_per_page = article_gate.saturating_mul(8);
    eprintln!(
        "probe_001 collection page (8 items): per-item gate {single_per_page:?} vs coarse desk {coarse_desk:?} ({:?}x)",
        single_per_page.as_secs_f64() / coarse_desk.as_secs_f64()
    );

    assert!(
        article_gate < Duration::from_millis(10),
        "single-resource gate must stay sub-10ms: {article_gate:?}"
    );
    assert!(
        coarse_desk < Duration::from_millis(5),
        "coarse desk gate must stay sub-5ms: {coarse_desk:?}"
    );
}
