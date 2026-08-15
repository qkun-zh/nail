mod api;
mod authorization;
mod logic;
mod other;
mod repo;

use std::path::PathBuf;

use other::server;

#[cfg(test)]
#[path = "../../../test/unit/back/harness.rs"]
mod unit_tests;

#[cfg(all(test, feature = "end_to_end"))]
#[path = "../../../test/end_to_end/mod.rs"]
mod end_to_end_tests;

#[tokio::main]
async fn main() {
    let code = match run().await {
        Ok(()) => 0,
        Err(e) => {
            other::log::record_startup_failure(&e);
            eprintln!("nail_back fatal error: {e:#}");
            1
        }
    };
    std::process::exit(code);
}

async fn run() -> anyhow::Result<()> {
    let config = other::conf::AppConfig::load()?;

    other::log::init(&config.server)?;
    let prune_task = tokio::spawn(other::log::prune_loop(
        PathBuf::from(&config.server.log_dir),
        config.server.log_retention_days,
        config.server.log_max_file_count,
        config.server.log_prune_interval_secs,
    ));

    if config.smtp.password.is_empty() {
        tracing::warn!(
            "SMTP password is empty: fill it in conf/back/smtp.toml \
             (copy smtp.toml.example if missing) or email sending will fail"
        );
    }
    if !config.smtp.starttls {
        tracing::warn!(
            "smtp.starttls is false: emails (incl. one-time verification tokens) \
             are sent in PLAINTEXT — only use this for local test SMTP sinks"
        );
    }

    let result = server::run_server(config).await;
    prune_task.abort();
    result
}
