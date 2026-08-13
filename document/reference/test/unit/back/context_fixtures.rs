
use common::pow::{Challenge, Pow, ProveInput, prove};
use uuid::Uuid;

use crate::repo::db::DbHandle;
use crate::repo::types::KEY_TAG_NAME;

pub const TEST_POW_DIFFICULTY: u64 = 16;

pub async fn state() -> crate::other::AppState {
    sweep_stale_search_dirs();
    let search_dir = std::env::temp_dir().join(format!("nail_search_index_{}", Uuid::now_v7()));
    state_in_dir(&search_dir).await
}

pub async fn state_in_dir(search_dir: &std::path::Path) -> crate::other::AppState {
    let mut config = crate::other::conf::AppConfig::load().expect("embedded config must load");
    config.server.pow_difficulty_iterations = TEST_POW_DIFFICULTY;

    let db = crate::repo::new("memory").await.expect("graph db");

    crate::repo::schema::init_graph(&db, &config.server.user_zero_email)
        .await
        .expect("schema");

    let search = crate::repo::search::open_or_create_index(search_dir.to_str().unwrap())
        .await
        .expect("search index");
    crate::repo::search::rebuild_index(&search, &db)
        .await
        .expect("search index rebuild");

    let cache = crate::repo::TokenCaches::new(
        std::time::Duration::from_secs(config.server.token_ttl_seconds),
        std::time::Duration::from_secs(config.server.session_ttl_seconds),
        std::time::Duration::from_secs(config.server.download_token_ttl_seconds),
        std::time::Duration::from_secs(config.server.challenge_ttl_seconds),
        config.server.token_cache_capacity,
    );

    crate::other::AppState {
        db,
        search,
        cache,
        email: crate::other::email::EmailService::new(
            config.smtp.clone(),
            config.server.email_cooldown_seconds,
        ),
        config: std::sync::Arc::new(config),
    }
}

fn sweep_stale_search_dirs() {
    static SWEPT: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    SWEPT.get_or_init(|| {
        let Some(cutoff) =
            std::time::SystemTime::now().checked_sub(std::time::Duration::from_secs(3600))
        else {
            return;
        };
        let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if !name.starts_with("nail_search_index_") {
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let Ok(modified) = meta.modified() else {
                continue;
            };
            if modified < cutoff {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    });
}

pub fn stage_pdf(data: &[u8]) -> crate::other::pdf::PdfUpload {
    let tmp_dir = std::env::temp_dir().join(format!("nail_stage_pdf_{}", Uuid::now_v7()));
    std::fs::create_dir_all(&tmp_dir).expect("create stage pdf dir");
    let path = tmp_dir.join("stage.pdf");
    std::fs::write(&path, data).expect("write stage pdf");
    let hash = common::hash::pdf(data);
    crate::other::pdf::PdfUpload::received(hash, crate::other::pdf::TempPdf::new(path))
}

pub fn test_pdf() -> Vec<u8> {
    let header = b"%PDF-1.4\n";
    let obj1 = b"1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n";
    let obj2 = b"2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1\n>>\nendobj\n";
    let obj3 = b"3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n>>\nendobj\n";
    let off1 = header.len();
    let off2 = off1 + obj1.len();
    let off3 = off2 + obj2.len();
    let xref_start = off3 + obj3.len();
    let mut pdf = Vec::from(header);
    pdf.extend_from_slice(obj1);
    pdf.extend_from_slice(obj2);
    pdf.extend_from_slice(obj3);
    pdf.extend_from_slice(
        format!(
            "xref\n0 4\n0000000000 65535 f\n{off1:010} 00000 n\n{off2:010} 00000 n\n{off3:010} 00000 n\n\
             trailer\n<<\n/Size 4\n/Root 1 0 R\n>>\nstartxref\n{xref_start}\n%%EOF"
        )
        .as_bytes(),
    );
    pdf
}

pub fn content_hash_for(version_id: &str) -> String {
    common::hash::pdf(version_id.as_bytes())
}

pub fn proof_of_work_for(payload: &str, difficulty: u64) -> Pow {
    let challenge = Challenge {
        id: Uuid::now_v7(),
        difficulty,
    };
    prove(ProveInput {
        challenge,
        payload: payload.to_string(),
    })
    .expect("proof generation must succeed")
}

pub fn proof_of_work_for_issued(
    state: &crate::other::AppState,
    payload: &str,
    difficulty: u64,
) -> Pow {
    let pow = proof_of_work_for(payload, difficulty);
    crate::repo::token::challenge::create_challenge(&state.cache, &pow.challenge.id.to_string());
    pow
}

pub async fn delete_user(
    db: &DbHandle,
    user_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    crate::repo::article::transfer_account_assets(db, user_id).await?;
    Ok(())
}

pub async fn create_article_with_initial_version(
    db: &DbHandle,
    article_id: &str,
    author_id: &str,
    title: &str,
    summary: &str,
    tags: &[String],
    version_id: &str,
    version_string: &str,
    content_hash: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    crate::repo::article::create_article(
        db,
        article_id,
        author_id,
        title,
        summary,
        tags,
        version_id,
        version_string,
        content_hash,
        "",
    )
    .await?;
    Ok(())
}

pub async fn get_or_create_tag(
    db: &DbHandle,
    name: &str,
) -> Result<common::tag::TagRef, agdb::DbError> {
    let mut db_guard = db.write().await;
    db_guard.transaction_mut(|txn| crate::repo::tag::get_or_create_tag_in_txn(txn, name))
}

pub async fn find_tag_id_by_name(
    db: &DbHandle,
    name: &str,
) -> Result<Option<String>, agdb::DbError> {
    let db = db.read().await;
    let ids = crate::repo::db::find_by_index_sync(&db, KEY_TAG_NAME, name)?;
    let Some(id) = ids.first() else {
        return Ok(None);
    };
    Ok(crate::repo::db::read_node_sync::<crate::repo::types::IdRow>(&db, *id)?.map(|row| row.id))
}

pub fn test_pdf_variant(variant: &str) -> Vec<u8> {
    let header = b"%PDF-1.4\n";
    let obj1 = b"1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n";
    let obj2 = b"2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1\n>>\nendobj\n";
    let obj3 = b"3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n>>\nendobj\n";
    let variant_comment = format!("% {variant}\n");
    let off1 = header.len();
    let off2 = off1 + obj1.len();
    let off3 = off2 + obj2.len();
    let xref_start = off3 + obj3.len() + variant_comment.len();
    let mut pdf = Vec::from(header);
    pdf.extend_from_slice(obj1);
    pdf.extend_from_slice(obj2);
    pdf.extend_from_slice(obj3);
    pdf.extend_from_slice(variant_comment.as_bytes());
    pdf.extend_from_slice(
        format!(
            "xref\n0 4\n0000000000 65535 f\n{off1:010} 00000 n\n{off2:010} 00000 n\n{off3:010} 00000 n\n\
             trailer\n<<\n/Size 4\n/Root 1 0 R\n>>\nstartxref\n{xref_start}\n%%EOF"
        )
        .as_bytes(),
    );
    pdf
}
