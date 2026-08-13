use std::sync::Arc;

use crate::infrastructure::config::AppConfig;
use crate::infrastructure::email::RateLimitedSender;
use crate::repository::cache::TokenCaches;
use crate::repository::graph::DbHandle;
use crate::repository::search::SearchIndex;

#[derive(Clone)]
pub struct AppState {
    pub graph: DbHandle,
    pub search: SearchIndex,
    pub caches: TokenCaches,
    pub config: Arc<AppConfig>,
    pub email: RateLimitedSender,
}
