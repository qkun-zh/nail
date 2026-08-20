use std::time::Duration;

use crate::infrastructure::config::AppConfig;
use crate::infrastructure::state::{AppState, Configurator};
use crate::interface;
use crate::repository;

use tower_http::trace::TraceLayer;

pub async fn run_server(config: AppConfig) -> anyhow::Result<()> {
    let database = repository::graph::open(config.db_path())?;
    repository::seed::init_graph(&database, config.user_zero_email()).await?;
    let searcher =
        repository::search::SearchIndex::open_or_create(config.search_index_path()).await?;
    if searcher.was_recreated() {
        tracing::info!("rebuilt search index; synchronizing from graph");
        searcher.sync_all(&database).await?;
    }
    crate::infrastructure::pdf::prepare_pdf_storage(config.pdf_storage_path()).await?;

    let cache = repository::cache::TokenCaches::new(
        Duration::from_secs(config.token_ttl_seconds()),
        Duration::from_secs(config.session_ttl_seconds()),
        Duration::from_secs(config.challenge_ttl_seconds()),
        Duration::from_secs(config.download_token_ttl_seconds()),
        config.token_cache_capacity(),
    );

    let email_sender = emailer::Emailer::new(&config.emailer);

    let state = AppState {
        database,
        searcher,
        cache,
        emailer: email_sender,
        configurator: Configurator::new(config),
    };

    let listener = tokio::net::TcpListener::bind(state.configurator.listen_addr()).await?;
    tracing::info!(address = %state.configurator.listen_addr(), "listening");
    let router = interface::router::build_router(state.clone()).layer(TraceLayer::new_for_http());

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    state.searcher.close().await;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::warn!(error = %error, "failed to install Ctrl-C handler");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::warn!(error = %error, "failed to install SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutdown signal received, draining in-flight requests");
}
