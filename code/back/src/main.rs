#![allow(dead_code)]

mod infrastructure;
mod interface;
mod logic;
mod repository;

#[cfg(test)]
#[path = "../../../test/unit/back/harness.rs"]
mod back_tests;

#[tokio::main]
async fn main() {
    let code = match run().await {
        Ok(()) => 0,
        Err(error) => {
            infrastructure::logging::record_startup_failure(&error);
            eprintln!("nail_back fatal error: {error:#}");
            1
        }
    };
    std::process::exit(code);
}

async fn run() -> anyhow::Result<()> {
    let config = infrastructure::config::AppConfig::load()?;
    infrastructure::logging::init(&config.server)?;

    let prune_task = tokio::spawn(infrastructure::logging::prune_loop(
        std::path::PathBuf::from(&config.server.log_dir),
        config.server.log_retention_days,
        config.server.log_max_file_count,
        config.server.log_prune_interval_secs,
    ));

    if config.smtp.password.is_empty() {
        tracing::warn!("SMTP password is empty: fill it in configuration/smtp.toml or email sending will fail");
    }
    if !config.smtp.starttls {
        tracing::warn!("smtp.starttls is false: emails are sent in plaintext; only use this for a local test SMTP sink");
    }

    let result = infrastructure::server::run_server(config).await;
    prune_task.abort();
    result
}
