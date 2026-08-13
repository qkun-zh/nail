#![allow(dead_code)]

mod infrastructure;
mod interface;
mod logic;
mod repository;

#[tokio::main]
async fn main() {
    let code = match run().await {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("nail_back fatal error: {error:#}");
            1
        }
    };
    std::process::exit(code);
}

async fn run() -> anyhow::Result<()> {
    Ok(())
}
