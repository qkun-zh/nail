use std::time::Duration;

use anyhow::Context;

use crate::api;
use crate::other::AppState;
use crate::other::conf::AppConfig;
use crate::repo;
use seekstorm::index::Close;

pub async fn run_server(config: AppConfig) -> anyhow::Result<()> {
    let db = repo::new(&config.server.db_path).await?;
    repo::schema::init_graph(&db, &config.server.user_zero_email).await?;
    let search = repo::search::open_or_create_index(&config.server.search_index_path).await?;
    let rebuilt = repo::search::rebuild_index(&search, &db).await?;
    tracing::info!(articles = rebuilt, "search index rebuilt at startup");

    let cache = repo::TokenCaches::new(
        Duration::from_secs(config.server.token_ttl_seconds),
        Duration::from_secs(config.server.session_ttl_seconds),
        Duration::from_secs(config.server.download_token_ttl_seconds),
        Duration::from_secs(config.server.challenge_ttl_seconds),
        config.server.token_cache_capacity,
    );

    let tmp_dir = std::path::Path::new(&config.server.pdf_storage_path).join(".tmp");
    tokio::fs::create_dir_all(&tmp_dir)
        .await
        .with_context(|| format!("create pdf tmp dir {}", tmp_dir.display()))?;
    let mut tmp_entries = tokio::fs::read_dir(&tmp_dir).await?;
    while let Some(entry) = tmp_entries.next_entry().await? {
        let path = entry.path();
        if let Err(e) = tokio::fs::remove_file(&path).await {
            tracing::warn!(error = %e, path = %path.display(), "failed to clean leftover pdf temp file");
        }
    }

    let app_state = AppState {
        db,
        search,
        cache,
        email: crate::other::email::EmailService::new(
            config.smtp.clone(),
            config.server.email_cooldown_seconds,
        ),
        config: std::sync::Arc::new(config),
    };

    let listener = tokio::net::TcpListener::bind(&app_state.config.server.listen_addr).await?;
    tracing::info!(addr = %app_state.config.server.listen_addr, "listening");
    let app = api::router(app_state.clone())?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    app_state.search.close().await;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received, draining in-flight requests");
}
