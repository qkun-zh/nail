use std::sync::Arc;

use crate::infrastructure::config::AppConfig;
use crate::infrastructure::email::RateLimitedSender;
use crate::repository::cache::TokenCaches;
use crate::repository::graph::DbHandle;

#[derive(Clone)]
pub struct AppState {
    pub graph: DbHandle,
    pub caches: TokenCaches,
    pub config: Arc<AppConfig>,
    pub email: RateLimitedSender,
}
