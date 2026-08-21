use common::pow::{Challenge, Pow};

use crate::request::error::{RequestError, RequestResult};
use crate::request::http;

pub async fn prove_pow() -> RequestResult<Pow> {
    let challenge: Challenge =
        http::post_json("/challenges", &serde_json::json!({}), false, None).await?;
    crate::infrastructure::pow::prove(&challenge).map_err(RequestError::network)
}
