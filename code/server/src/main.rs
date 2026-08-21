mod infrastructure;
mod interface;
mod logic;
mod repository;

#[cfg(test)]
#[path = "../../../test/unit/server/harness.rs"]
mod server_tests;

#[tokio::main]
async fn main() {
    let code = match run().await {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("server fatal error: {error:#}");
            1
        }
    };
    std::process::exit(code);
}

async fn seed_samples(
    config: &infrastructure::config::AppConfig,
    count: usize,
) -> anyhow::Result<()> {
    let graph = infrastructure::server::open_database(config.db_path())?;
    repository::seed::init_graph(&graph, config.user_zero_email())?;
    let search =
        infrastructure::search::Searcher::open_or_create(config.search_index_path()).await?;
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
        let _guard = infrastructure::logging::init(&config.logging)?;
        seed_samples(&config, count).await?;
        return Ok(());
    }
    let _guard = infrastructure::logging::init(&config.logging)?;

    infrastructure::server::run_server(config).await?;
    Ok(())
}
