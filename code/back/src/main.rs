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

    let prune_directory = std::path::PathBuf::from(config.server.log_dir.clone());
    let prune_task = tokio::spawn(infrastructure::logging::prune_loop(
        prune_directory,
        config.server.log_retention_days,
        config.server.log_max_file_count,
        config.server.log_prune_interval_secs,
    ));

    infrastructure::server::run_server(config).await?;
    prune_task.abort();
    Ok(())
}
