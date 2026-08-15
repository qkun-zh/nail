use std::sync::Arc;

use crate::other::conf::AppConfig;
use crate::other::email::EmailService;
use crate::repo::TokenCaches;
use crate::repo::db::DbHandle;
use crate::repo::search::SearchIndexHandle;

#[derive(Clone)]
pub struct AppState {
    pub db: DbHandle,
    pub search: SearchIndexHandle,
    pub cache: TokenCaches,
    pub config: Arc<AppConfig>,
    pub email: EmailService,
}
