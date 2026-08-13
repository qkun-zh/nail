
#[path = "concurrency/util.rs"]
mod util;

use std::sync::Arc;

use tokio::task::JoinSet;
use uuid::Uuid;

use crate::logic::article::handle_create_article;
use crate::logic::authenticate::handle_token_exchange;
use crate::logic::download::handle_consume_download;
use crate::logic::email::handle_email_update_confirm;
use crate::logic::user::{handle_deregister_confirm, handle_read_name, handle_update_name};
use crate::unit_tests::context;

use util::{err_count, is_bad_request, join_all, ok_count};


#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_login_same_email_creates_one_user() {
    let state = Arc::new(context::state().await);
    let email_hash = common::hash::email("dup@qq.com");
    let t1 = Uuid::now_v7().to_string();
    let t2 = Uuid::now_v7().to_string();
    crate::repo::token::authenticate::create_authenticate_token(
        &state.cache,
        &t1,
        &email_hash,
        &Uuid::now_v7().to_string(),
    );
    crate::repo::token::authenticate::create_authenticate_token(
        &state.cache,
        &t2,
        &email_hash,
        &Uuid::now_v7().to_string(),
    );

    let difficulty = state.config.server.pow_difficulty_iterations;
    let p1 = Arc::new(context::proof_of_work_for_issued(&state, &t1, difficulty));
    let p2 = Arc::new(context::proof_of_work_for_issued(&state, &t2, difficulty));

    let mut futures = Vec::new();
    for pow in [p1, p2] {
        let state = state.clone();
        futures.push(Box::pin(async move {
            handle_token_exchange(&state, &pow).await
        }));
    }
    let results = join_all(futures).await;
    assert_eq!(
        ok_count(&results),
        2,
        "both logins must succeed: {results:?}"
    );

    let sessions: Vec<String> = results.into_iter().map(|r| r.unwrap()).collect();
    let user_id = crate::repo::user::find_user_by_email_address_hash(&state.db, &email_hash)
        .await
        .unwrap()
        .unwrap();
    let mut live = 0;
    for s in &sessions {
        if let Some(owner) =
            crate::repo::token::session::find_user_id_by_session_token(&state.cache, s)
        {
            assert_eq!(owner, user_id, "every live session maps to the one account");
            live += 1;
        }
    }
    assert!(live >= 1, "at least one session must survive");
    close_search(&state).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_set_name_unique_conflict() {
    let state = Arc::new(context::state().await);
    let u1 = Uuid::now_v7().to_string();
    let u2 = Uuid::now_v7().to_string();
    crate::repo::user::create_user(&state.db, &u1, "h1")
        .await
        .unwrap();
    crate::repo::user::create_user(&state.db, &u2, "h2")
        .await
        .unwrap();
    let s1 = Uuid::now_v7().to_string();
    let s2 = Uuid::now_v7().to_string();
    crate::repo::token::session::create_session_token(&state.cache, &s1, &u1);
    crate::repo::token::session::create_session_token(&state.cache, &s2, &u2);

    let difficulty = state.config.server.pow_difficulty_iterations;
    let p1 = Arc::new(context::proof_of_work_for_issued(
        &state, "alice", difficulty,
    ));
    let p2 = Arc::new(context::proof_of_work_for_issued(
        &state, "alice", difficulty,
    ));
    let s1_check = s1.clone();

    let mut futures = Vec::new();
    for (session, pow) in [(s1, p1), (s2, p2)] {
        let state = state.clone();
        futures.push(Box::pin(async move {
            let res = handle_update_name(&state, &pow, &session).await;
            (session, res)
        }));
    }
    let results = join_all(futures).await;

    let mut winners = 0;
    let mut loser_session = None;
    for (session, res) in results {
        match res {
            Ok(name) => {
                assert_eq!(name, "alice");
                winners += 1;
            }
            Err(_) => loser_session = Some(session),
        }
    }
    assert_eq!(
        winners, 1,
        "the unique name index must have exactly one winner"
    );
    let loser_session = loser_session.expect("one attempt must lose");
    let loser_default = if loser_session == s1_check {
        u1.replace('-', "")
    } else {
        u2.replace('-', "")
    };
    assert_eq!(
        handle_read_name(&state, &loser_session)
            .await
            .unwrap()
            .as_str(),
        loser_default.as_str(),
        "the loser's account keeps its default name (hyphen-less uuidv7)"
    );
    close_search(&state).await;
}


#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_get_or_create_tag_deduplicates() {
    let state = Arc::new(context::state().await);
    let mut futures = Vec::new();
    for _ in 0..10 {
        let state = state.clone();
        futures.push(Box::pin(async move {
            context::get_or_create_tag(&state.db, "concurrency").await
        }));
    }
    let results = join_all(futures).await;
    for r in &results {
        assert!(r.is_ok(), "no tag creation attempt may fail: {r:?}");
    }
    let ids: Vec<String> = results.into_iter().map(|r| r.unwrap().id).collect();
    assert!(
        ids.iter().all(|id| id == &ids[0]),
        "all refs share one tag id: {ids:?}"
    );
    let found = context::find_tag_id_by_name(&state.db, "concurrency")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found, ids[0]);
    close_search(&state).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_article_creation_all_persist() {
    let state = Arc::new(context::state().await);
    let user_id = Uuid::now_v7().to_string();
    crate::repo::user::create_user(&state.db, &user_id, "h")
        .await
        .unwrap();
    let session = Uuid::now_v7().to_string();
    crate::repo::token::session::create_session_token(&state.cache, &session, &user_id);

    let mut futures = Vec::new();
    for i in 0..3 {
        let state = state.clone();
        let session = session.clone();
        futures.push(Box::pin(async move {
            handle_create_article(
                &state,
                &session,
                &format!("title {i}"),
                &format!("article {i}"),
                "#tag",
                "1.0.0",
                "initial",
                context::stage_pdf(&context::test_pdf_variant(&format!("create {i}"))),
            )
            .await
        }));
    }
    let results = join_all(futures).await;
    let mut ids = Vec::new();
    for r in results {
        let (article_id, _version_id) = r.expect("every create must succeed");
        ids.push(article_id);
    }
    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 3);
    for id in &ids {
        assert!(
            crate::repo::article::read_article(&state.db, id)
                .await
                .unwrap()
                .is_some(),
            "created article must exist: {id}"
        );
    }
    close_search(&state).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn create_article_racing_deregister_leaves_no_orphans() {
    for round in 0..3 {
        let mut state = context::state().await;
        let storage = std::env::temp_dir().join(format!("nail_dereg_{round}_{}", Uuid::now_v7()));
        std::fs::create_dir_all(&storage).unwrap();
        std::sync::Arc::make_mut(&mut state.config)
            .server
            .pdf_storage_path = storage.to_str().unwrap().to_string();
        let state = Arc::new(state);

        let user_id = Uuid::now_v7().to_string();
        let email_hash = common::hash::email("race@qq.com");
        crate::repo::user::create_user(&state.db, &user_id, &email_hash)
            .await
            .unwrap();
        let session = Uuid::now_v7().to_string();
        crate::repo::token::session::create_session_token(&state.cache, &session, &user_id);
        let dtoken = Uuid::now_v7().to_string();
        crate::repo::token::deregister::create_deregister_token(
            &state.cache,
            &dtoken,
            &user_id,
            &email_hash,
        );

        let difficulty = state.config.server.pow_difficulty_iterations;
        let pow = Arc::new(context::proof_of_work_for_issued(
            &state, &dtoken, difficulty,
        ));

        let mut set = JoinSet::new();
        {
            let state = state.clone();
            let session = session.clone();
            set.spawn(async move {
                handle_create_article(
                    &state,
                    &session,
                    "racing article",
                    "racing article",
                    "#race_tag",
                    "1.0.0",
                    "initial",
                    context::stage_pdf(&context::test_pdf_variant("race")),
                )
                .await
                .map(|(_article_id, _version_id)| "created".to_string())
            });
        }
        {
            let state = state.clone();
            let pow = pow.clone();
            let session = session.clone();
            set.spawn(async move {
                handle_deregister_confirm(&state, &pow, &session)
                    .await
                    .map(|()| "deregistered".to_string())
            });
        }
        let mut results = Vec::new();
        while let Some(res) = set.join_next().await {
            results.push(res.expect("concurrent task panicked"));
        }
        eprintln!("round {round}: create/deregister race results = {results:?}");

        let db = state.db.read().await;
        let edges = db
            .exec(
                agdb::QueryBuilder::search()
                    .elements()
                    .where_()
                    .edge()
                    .and()
                    .key(crate::repo::types::KEY_TYPE)
                    .value(crate::repo::types::EDGE_USER_TO_ARTICLE)
                    .query(),
            )
            .unwrap();
        let dangling = edges
            .elements
            .iter()
            .filter(|el| {
                db.exec(agdb::QueryBuilder::select().ids([el.from]).query())
                    .unwrap()
                    .elements
                    .first()
                    .and_then(|n| {
                        n.values.iter().find(|kv| {
                            kv.key
                                == agdb::DbValue::String(crate::repo::types::KEY_TYPE.to_string())
                        })
                    })
                    .and_then(|kv| match &kv.value {
                        agdb::DbValue::String(v) => Some(v.clone()),
                        _ => None,
                    })
                    .as_deref()
                    != Some(crate::repo::types::ENTITY_TYPE_USER)
            })
            .count() as u64;
        assert_eq!(dangling, 0, "round {round}: dangling user_to_article edge");

        let orphans = db
            .exec(
                agdb::QueryBuilder::search()
                    .elements()
                    .where_()
                    .key(crate::repo::types::KEY_TYPE)
                    .value(crate::repo::types::ENTITY_TYPE_ARTICLE)
                    .and()
                    .edge_count_to(agdb::CountComparison::Equal(0))
                    .query(),
            )
            .unwrap();
        assert_eq!(
            orphans.elements.len() as u64,
            0,
            "round {round}: orphaned article"
        );
        drop(db);

        std::fs::remove_dir_all(&storage).ok();
        close_search(&state).await;
    }
}


#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn authenticate_token_is_single_use_under_race() {
    let state = Arc::new(context::state().await);
    let email_hash = common::hash::email("race@qq.com");
    let token = Uuid::now_v7().to_string();
    crate::repo::token::authenticate::create_authenticate_token(
        &state.cache,
        &token,
        &email_hash,
        &Uuid::now_v7().to_string(),
    );

    let difficulty = state.config.server.pow_difficulty_iterations;
    let pows: Vec<common::pow::Pow> = (0..10)
        .map(|_| context::proof_of_work_for_issued(&state, &token, difficulty))
        .collect();

    let mut futures = Vec::new();
    for pow in pows {
        let state = state.clone();
        futures.push(Box::pin(async move {
            handle_token_exchange(&state, &Arc::new(pow)).await
        }));
    }
    let results = join_all(futures).await;

    assert_eq!(
        ok_count(&results),
        1,
        "exactly one authenticate must succeed: {results:?}"
    );
    assert_eq!(
        err_count(&results, is_bad_request),
        9,
        "losers must be BadRequest: {results:?}"
    );

    let session = results.into_iter().find_map(Result::ok).unwrap();
    let owner = crate::repo::token::session::find_user_id_by_session_token(&state.cache, &session)
        .expect("session must exist");
    let user_id = crate::repo::user::find_user_by_email_address_hash(&state.db, &email_hash)
        .await
        .unwrap()
        .expect("user must be created");
    assert_eq!(
        owner, user_id,
        "the single session must belong to the account"
    );
    close_search(&state).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deregister_confirm_is_single_use_under_race() {
    let state = Arc::new(context::state().await);
    let user_id = Uuid::now_v7().to_string();
    let email_hash = common::hash::email("race2@qq.com");
    crate::repo::user::create_user(&state.db, &user_id, &email_hash)
        .await
        .unwrap();
    let session = Uuid::now_v7().to_string();
    crate::repo::token::session::create_session_token(&state.cache, &session, &user_id);
    let dtoken = Uuid::now_v7().to_string();
    crate::repo::token::deregister::create_deregister_token(
        &state.cache,
        &dtoken,
        &user_id,
        &email_hash,
    );

    let difficulty = state.config.server.pow_difficulty_iterations;
    let pow1 = Arc::new(context::proof_of_work_for_issued(
        &state, &dtoken, difficulty,
    ));
    let pow2 = Arc::new(context::proof_of_work_for_issued(
        &state, &dtoken, difficulty,
    ));

    let mut futures = Vec::new();
    for pow in [pow1, pow2] {
        let state = state.clone();
        let session = session.clone();
        futures.push(Box::pin(async move {
            handle_deregister_confirm(&state, &pow, &session).await
        }));
    }
    let results = join_all(futures).await;

    assert_eq!(
        ok_count(&results),
        2,
        "transfer+deletion is idempotent: {results:?}"
    );
    assert!(
        results.iter().all(Result::is_ok),
        "no request may hard-fail: {results:?}"
    );

    assert!(
        crate::repo::user::read_user(&state.db, &user_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        crate::repo::token::session::find_user_id_by_session_token(&state.cache, &session)
            .is_none()
    );
    assert!(
        crate::repo::token::deregister::find_user_id_by_deregister_token(&state.cache, &dtoken)
            .is_none(),
        "deregister token must be consumed"
    );
    close_search(&state).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn download_token_is_single_use_under_race() {
    let state = Arc::new(context::state().await);
    let user_id = Uuid::now_v7().to_string();
    crate::repo::user::create_user(&state.db, &user_id, "h")
        .await
        .unwrap();
    let session = Uuid::now_v7().to_string();
    crate::repo::token::session::create_session_token(&state.cache, &session, &user_id);
    let article_id = Uuid::now_v7().to_string();
    let version_id = Uuid::now_v7().to_string();
    let content_hash = context::content_hash_for(&version_id);
    context::create_article_with_initial_version(
        &state.db,
        &article_id,
        &user_id,
        "title",
        "desc",
        &["#t".to_string()],
        &version_id,
        "1.0.0",
        &content_hash,
    )
    .await
    .unwrap();

    let download_token = Uuid::now_v7().to_string();
    crate::repo::token::download::create_download_token(
        &state.cache,
        &download_token,
        &version_id,
        &user_id,
    );

    let mut futures = Vec::new();
    for _ in 0..10 {
        let state = state.clone();
        let session = session.clone();
        let token = download_token.clone();
        futures.push(Box::pin(async move {
            handle_consume_download(&state, &session, &token).await
        }));
    }
    let results = join_all(futures).await;

    assert_eq!(
        ok_count(&results),
        1,
        "exactly one download must succeed: {results:?}"
    );
    assert_eq!(
        err_count(&results, is_bad_request),
        9,
        "losers must be BadRequest: {results:?}"
    );

    assert!(
        crate::repo::token::download::find_download_token(&state.cache, &download_token).is_none(),
        "download token must be consumed"
    );
    close_search(&state).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn email_update_token_is_single_use_under_race() {
    let state = Arc::new(context::state().await);
    let user_id = Uuid::now_v7().to_string();
    let old_hash = common::hash::email("old@qq.com");
    let new_hash = common::hash::email("new@qq.com");
    crate::repo::user::create_user(&state.db, &user_id, &old_hash)
        .await
        .unwrap();
    let session = Uuid::now_v7().to_string();
    crate::repo::token::session::create_session_token(&state.cache, &session, &user_id);

    let old_token = Uuid::now_v7().to_string();
    let new_token = Uuid::now_v7().to_string();
    crate::repo::token::email_update::create_email_update_token(
        &state.cache,
        &user_id,
        &old_hash,
        &new_hash,
        &common::hash::token(&old_token),
        &common::hash::token(&new_token),
    );

    let payload = format!("{}\n{}", old_token, new_token);
    let difficulty = state.config.server.pow_difficulty_iterations;
    let pow1 = Arc::new(context::proof_of_work_for_issued(
        &state, &payload, difficulty,
    ));
    let pow2 = Arc::new(context::proof_of_work_for_issued(
        &state, &payload, difficulty,
    ));

    let mut futures = Vec::new();
    for pow in [pow1, pow2] {
        let state = state.clone();
        let session = session.clone();
        let old_token = old_token.clone();
        let new_token = new_token.clone();
        futures.push(Box::pin(async move {
            handle_email_update_confirm(&state, &pow, &old_token, &new_token, &session).await
        }));
    }
    let results = join_all(futures).await;

    assert_eq!(
        ok_count(&results),
        1,
        "exactly one email_update must succeed: {results:?}"
    );
    assert_eq!(
        err_count(&results, is_bad_request),
        1,
        "loser must be BadRequest: {results:?}"
    );

    let user = crate::repo::user::read_user(&state.db, &user_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user.email_address_hash, new_hash);
    close_search(&state).await;
}

async fn close_search(state: &crate::other::AppState) {
    seekstorm::index::Close::close(&state.search).await;
}
