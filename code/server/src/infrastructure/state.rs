use std::sync::Arc;

use database::Database;

use crate::infrastructure::authorizer::Authorizer;
use crate::infrastructure::config::AppConfig;
use crate::infrastructure::search::Searcher;

#[derive(Clone)]
pub struct AppState {
    pub authorizer: Authorizer,
    pub database: Database,
    pub searcher: Searcher,
    pub cache: cache::Cache,
    pub config: Arc<AppConfig>,
    pub emailer: emailer::Emailer,
}
