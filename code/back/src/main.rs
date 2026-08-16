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

async fn seed_samples(
    config: &infrastructure::config::AppConfig,
    count: usize,
) -> anyhow::Result<()> {
    let graph = repository::graph::open(&config.server.db_path)?;
    repository::seed::init_graph(&graph, &config.server.user_zero_email).await?;
    let search =
        repository::search::SearchIndex::open_or_create(&config.server.search_index_path).await?;
    repository::seed_demo::seed_sample_articles(&graph, &search, count).await?;
    search.close().await;
    tracing::info!("sample seeding finished");
    Ok(())
}

async fn run() -> anyhow::Result<()> {
    let config = infrastructure::config::AppConfig::load()?;
    if std::env::args().any(|arg| arg == "seed-samples") {
        let count = std::env::args()
            .find_map(|arg| arg.parse::<usize>().ok())
            .unwrap_or(300);
        infrastructure::logging::init(&config.server)?;
        seed_samples(&config, count).await?;
        return Ok(());
    }
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
