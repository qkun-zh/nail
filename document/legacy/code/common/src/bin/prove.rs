
use anyhow::{Context, Result, bail};
use common::pow::{Challenge, ProveInput, prove};
use uuid::Uuid;

const MAX_DIFFICULTY: u64 = 10_000;

fn main() -> Result<()> {
    use std::io::Write;

    let usage = "usage: prove <challenge-id> <difficulty> <payload>";
    let mut args = std::env::args_os().skip(1);
    let id = args
        .next()
        .context(usage)?
        .into_string()
        .map_err(|_| anyhow::anyhow!("challenge id is not valid UTF-8"))?;
    let difficulty: u64 = args
        .next()
        .context(usage)?
        .to_str()
        .context("difficulty is not valid UTF-8")?
        .parse()?;
    if difficulty == 0 || difficulty > MAX_DIFFICULTY {
        bail!("difficulty {difficulty} out of range (1..={MAX_DIFFICULTY})");
    }
    let payload = args
        .next()
        .context(usage)?
        .into_string()
        .map_err(|_| anyhow::anyhow!("payload is not valid UTF-8"))?;
    if args.next().is_some() {
        bail!(usage);
    }

    let pow = prove(ProveInput {
        challenge: Challenge {
            id: Uuid::parse_str(&id).context("invalid challenge id")?,
            difficulty,
        },
        payload,
    })?;
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{}", serde_json::to_string(&pow)?)?;
    Ok(())
}
