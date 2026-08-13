
use std::future::Future;
use std::pin::Pin;

use tokio::task::JoinSet;

use crate::logic::error::LogicError;

pub(crate) async fn join_all<F, T>(futures: Vec<Pin<Box<F>>>) -> Vec<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let mut set = JoinSet::new();
    for fut in futures {
        set.spawn(fut);
    }
    let mut out = Vec::with_capacity(set.len());
    while let Some(res) = set.join_next().await {
        out.push(res.expect("concurrent task panicked"));
    }
    out
}

pub(crate) fn ok_count<T>(results: &[Result<T, LogicError>]) -> usize {
    results.iter().filter(|r| r.is_ok()).count()
}

pub(crate) fn err_count<T>(
    results: &[Result<T, LogicError>],
    pred: fn(&LogicError) -> bool,
) -> usize {
    results
        .iter()
        .filter(|r| matches!(r, Err(e) if pred(e)))
        .count()
}

pub(crate) fn is_bad_request(e: &LogicError) -> bool {
    matches!(e, LogicError::BadRequest(_))
}
