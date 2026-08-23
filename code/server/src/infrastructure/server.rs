use std::path::Path;
use std::time::Duration;

use crate::infrastructure::authorizer::Authorizer;
use crate::infrastructure::config::AppConfig;
use crate::infrastructure::state::AppState;
use crate::interface;
use crate::repository;
use crate::repository::schema::INDEX_KEYS;

use axum::http::Uri;
use tower_http::trace::TraceLayer;

const REDACTED_DOWNLOAD_TOKEN: &str = "<REDACTED>";

pub(crate) fn open_database(address: &str) -> anyhow::Result<database::Database> {
    let indexes: Vec<String> = INDEX_KEYS.iter().map(|key| (*key).to_string()).collect();
    let database = match address.trim().to_ascii_lowercase().as_str() {
        "memory" | "mem" | ":memory:" | "in-memory" => {
            database::Database::open_memory("nail_memory", &indexes)?
        }
        path => {
            if path.is_empty() || path == "/" {
                anyhow::bail!("invalid db_path: {path:?} (use a file path or a memory indicator)");
            }
            database::Database::open_mapped(Path::new(path), &indexes)?
        }
    };
    Ok(database)
}

pub async fn run_server(config: AppConfig) -> anyhow::Result<()> {
    let database = open_database(config.db_path())?;
    repository::seed::init_graph(&database, config.user_zero_email())?;
    let searcher =
        crate::infrastructure::search::Searcher::open_or_create(config.search_index_path()).await?;
    if searcher.was_recreated() {
        tracing::info!("rebuilt search index; synchronizing from graph");
        searcher.sync_all(&database).await?;
    }
    crate::infrastructure::pdf::prepare_pdf_storage(config.pdf_storage_path()).await?;

    let cache = cache::Cache::new(&config.cache);

    let email_sender = emailer::Emailer::new(&config.emailer)?;

    let authorizer = Authorizer::new(database.clone())?;

    let state = AppState {
        authorizer,
        database,
        searcher,
        cache,
        emailer: email_sender,
        config: std::sync::Arc::new(config),
    };

    let listener = tokio::net::TcpListener::bind(state.config.server.listen_addr.as_str()).await?;
    tracing::info!(address = %state.config.server.listen_addr.as_str(), "listening");
    let router = interface::router::build_router(state.clone()).layer(
        TraceLayer::new_for_http()
            .make_span_with(|request: &axum::extract::Request| {
                tracing::info_span!(
                    "request",
                    method = %request.method(),
                    uri = %redacted_uri(request.uri()),
                )
            })
            .on_request(|_request: &axum::extract::Request, _span: &tracing::Span| {})
            .on_response(
                |response: &axum::response::Response, latency: Duration, _span: &tracing::Span| {
                    let status = response.status().as_u16();
                    let latency_ms = latency.as_secs() * 1000 + u64::from(latency.subsec_millis());
                    if status < 400 {
                        tracing::info!(
                            target: "tower_http::trace::on_response",
                            status,
                            latency_ms,
                        );
                    } else {
                        tracing::warn!(
                            target: "tower_http::trace::on_response",
                            status,
                            latency_ms,
                        );
                    }
                },
            ),
    );

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    state.searcher.close().await;
    Ok(())
}

pub(crate) fn redacted_uri(uri: &Uri) -> String {
    match uri.query() {
        None => uri.path().to_string(),
        Some(query) => format!("{}?{}", uri.path(), redact_token_query(query)),
    }
}

pub(crate) fn redact_token_query(query: &str) -> String {
    query
        .split('&')
        .map(|pair| {
            let key = pair.split_once('=').map_or(pair, |(key, _)| key);
            if key == "token" {
                format!("token={REDACTED_DOWNLOAD_TOKEN}")
            } else {
                pair.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("&")
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
