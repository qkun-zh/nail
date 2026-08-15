
use anyhow::Result;
use common::pow::{Pow, ProveInput};

pub async fn prove(input: ProveInput) -> Result<Pow> {
    common::pow::prove(input)
}
