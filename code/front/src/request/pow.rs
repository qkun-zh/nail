use nail_common::pow::{Challenge, Pow, ProveInput};

use crate::request::error::{RequestError, RequestResult};
use crate::request::http;

pub async fn prove_pow(payload: String) -> RequestResult<Pow> {
    let challenge: Challenge = http::get_json("/challenge/read", false).await?;
    crate::infrastructure::pow::prove(ProveInput { challenge, payload })
        .map_err(RequestError::network)
}
