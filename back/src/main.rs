mod back_end_server;
mod banner;
mod config;
mod log;
use crate::back_end_server::start_server_of_back_end;
use crate::banner::display_banner;
use crate::config::TheConfig;
use crate::log::init_log_recorder;
use config::load_config_of_all_servers;
use tracing::info;

#[tokio::main]
async fn main() {
    display_banner();

    init_log_recorder();
    info!("[init log recorder]");

    info!("[load config of all servers]");
    let config: TheConfig = load_config_of_all_servers();

    info!("[start back end server]");
    start_server_of_back_end(&config.back_end_server).await;
}
