use axum::{Router};
use std::net::{SocketAddr, IpAddr};
use axum::extract::{OriginalUri, Path, Query};
use axum::response::IntoResponse;
use axum::routing::get;
use tokio::net::TcpListener;
use tower_http::services::{ServeDir, ServeFile};
use tracing::{error};
use crate::config::{BackEndServerConfig};

pub fn create_router(config: &BackEndServerConfig) -> Router {
    Router::new()
        .fallback_service(
            get(handler)
        )
}

async fn handler(OriginalUri(uri): OriginalUri) ->impl IntoResponse {
    return "hello, qian kun zhang!".to_string();
}

pub async fn create_listener(config: &BackEndServerConfig) -> TcpListener {
    let ip: IpAddr = config.ip_address.parse()
        .unwrap_or_else(|e|{
            error!("wrong ip address: {}", e);
            panic!();
        });
    let socket_addr = SocketAddr::from((ip, config.port_number));

    TcpListener::bind(socket_addr).await
        .unwrap_or_else(|e| {
            error!("wrong port number: {}", e);
            panic!();
        })
}

pub async fn start_server_of_back_end(config: &BackEndServerConfig) {
    let listener = create_listener(config).await;
    let router = create_router(config);
    axum::serve(listener, router).await
        .unwrap_or_else(|e| {
            error!("failed to start server of back end: {}", e);
            panic!();
        });

}