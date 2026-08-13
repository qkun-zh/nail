use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArticleSearchParams {
    pub q: Option<String>,
    pub ranges: Option<String>,
    pub sort: Option<String>,
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub limit: Option<u64>,
    pub page: Option<u64>,
}

#[cfg(test)]
#[path = "../../../test/unit/common/search/tests.rs"]
mod tests;