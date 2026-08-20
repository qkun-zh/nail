use anyhow::Context;

use crate::repository::graph::DbHandle;
use crate::repository::search::SearchIndex;

const SAMPLE_TAG_POOL: &[&str] = &[
    "rust",
    "database",
    "web",
    "search",
    "graph",
    "testing",
    "api",
    "security",
    "performance",
    "algorithm",
    "backend",
    "frontend",
    "wasm",
    "async",
    "config",
    "observability",
    "logging",
    "cache",
    "storage",
    "network",
    "protocol",
    "crypto",
    "auth",
    "deployment",
    "docker",
    "linux",
    "cli",
    "ui",
    "data",
    "ml",
];

const SAMPLE_TOPIC_WORDS: &[&str] = &[
    "solar",
    "lunar",
    "quantum",
    "neural",
    "distributed",
    "concurrent",
    "persistent",
    "elastic",
    "atomic",
    "semantic",
    "vector",
    "streaming",
    "incremental",
    "reactive",
    "declarative",
    "recursive",
    "asynchronous",
    "eventual",
    "graph",
    "search",
];

const SAMPLE_OBJECT_WORDS: &[&str] = &[
    "index",
    "cache",
    "pipeline",
    "store",
    "queue",
    "router",
    "schema",
    "parser",
    "compiler",
    "tracer",
    "replica",
    "snapshot",
    "journal",
    "filter",
    "scheduler",
    "gateway",
    "broker",
    "session",
    "transaction",
    "partition",
];

struct SampleRng {
    state: u64,
}

impl SampleRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1),
        }
    }

    fn next(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state >> 11
    }

    fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            return 0;
        }
        usize::try_from(self.next() % bound as u64).unwrap_or(usize::MAX)
    }
}

fn unique_sample_title(rng: &mut SampleRng, index: usize) -> String {
    let topic = SAMPLE_TOPIC_WORDS[rng.below(SAMPLE_TOPIC_WORDS.len())];
    let object = SAMPLE_OBJECT_WORDS[rng.below(SAMPLE_OBJECT_WORDS.len())];
    format!("{topic} {object} {index}")
}

fn unique_sample_summary(rng: &mut SampleRng, index: usize) -> String {
    let topic = SAMPLE_TOPIC_WORDS[rng.below(SAMPLE_TOPIC_WORDS.len())];
    let object = SAMPLE_OBJECT_WORDS[rng.below(SAMPLE_OBJECT_WORDS.len())];
    format!(
        "Sample {index}: a {topic} {object} design note covering tradeoffs, benchmarks and follow-ups."
    )
}

fn sample_tags(rng: &mut SampleRng) -> Vec<String> {
    let count = 1 + rng.below(4);
    let mut tags = Vec::with_capacity(count);
    for _ in 0..count {
        let tag = SAMPLE_TAG_POOL[rng.below(SAMPLE_TAG_POOL.len())].to_string();
        if !tags.iter().any(|existing| existing == &tag) {
            tags.push(tag);
        }
    }
    tags
}

fn sample_version_number(rng: &mut SampleRng) -> String {
    format!("{}.{}.{}", 1 + rng.below(4), rng.below(10), rng.below(20))
}

fn sample_note(rng: &mut SampleRng, index: usize) -> String {
    let topic = SAMPLE_TOPIC_WORDS[rng.below(SAMPLE_TOPIC_WORDS.len())];
    let object = SAMPLE_OBJECT_WORDS[rng.below(SAMPLE_OBJECT_WORDS.len())];
    format!("version note {index}: {topic} {object} adjustments")
}

pub async fn seed_sample_articles(
    db: &DbHandle,
    search: &SearchIndex,
    count: usize,
) -> anyhow::Result<()> {
    let author_count = count.clamp(2, 12);
    let mut author_ids = Vec::with_capacity(author_count);
    for author_index in 0..author_count {
        let email = format!("sample-author-{author_index}@example.com");
        let user_id =
            crate::repository::user::create_user(db, &nail_common::hash::hash(email.as_bytes())?)
                .await
                .with_context(|| format!("create sample author {author_index}"))?;
        crate::repository::user::update_user_name(
            db,
            &user_id,
            &format!("sample-author-{author_index:02}"),
        )
        .await
        .map_err(|error| anyhow::anyhow!("name sample author {author_index}: {error}"))?;
        author_ids.push(user_id);
    }

    let now = nail_common::time::now_ms()?;
    let mut rng = SampleRng::new(now);
    for index in 0..count {
        let author = &author_ids[rng.below(author_ids.len())];
        let back_ms = (index as u64).saturating_mul(3_600_000);
        let sample_ms = now.saturating_sub(back_ms);
        let article_id = nail_common::time::uuidv7_min_for_ms(sample_ms);
        let version_id = nail_common::time::uuidv7_max_for_ms(sample_ms);
        let content_hash = format!("{:032x}", now.saturating_add(index as u64));
        let draft = crate::repository::article::ArticleDraft {
            article_id,
            author_id: author.clone(),
            title: unique_sample_title(&mut rng, index),
            summary: unique_sample_summary(&mut rng, index),
            tags: sample_tags(&mut rng),
            first_version: crate::repository::version::VersionDraft {
                version_id,
                version_number: sample_version_number(&mut rng),
                content_hash,
                note: sample_note(&mut rng, index),
            },
        };
        crate::repository::article::create_article(db, &draft)
            .await
            .with_context(|| format!("create sample article {index}"))?;
    }

    let synced = search.sync_all(db).await?;
    tracing::info!(count, synced, "seeded sample articles");
    Ok(())
}
