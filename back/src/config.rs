use serde::Deserialize;
use std::{env, fs};
use std::path::PathBuf;
use clap::Parser;
use toml;
use tracing::{error};

const DEFAULT_CONFIG_FILE_NAME: &str = "nail_conf.toml";

#[derive(Parser, Debug)]
struct Cli {
    #[arg(
        short = 'c',
        long = "config",
        help = "Follow this path to find the configuration file"
    )]
    config_path: Option<String>,
}

#[derive(Debug, Deserialize,Clone)]
pub struct TheConfig {
    pub back_end_server: BackEndServerConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BackEndServerConfig {
    pub ip_address: String,
    pub port_number: u16,
}

fn get_default_config_path() -> PathBuf {
    let exe_path = env::current_exe()
        .unwrap_or_else(|err| {
            error!("Failed to get path of executable file: {}", err);
            panic!();
        });

    let exe_dir = exe_path.parent()
        .unwrap_or_else(|| {
            error!("Failed to get the directory where the executable file is located");
            panic!();
        });

    exe_dir.join(DEFAULT_CONFIG_FILE_NAME)
}

pub fn load_config_of_all_servers() -> TheConfig {
    let cli = Cli::parse();
    let config_path = match cli.config_path {
        Some(path) => PathBuf::from(path),
        None => get_default_config_path(),
    };



    let config_text = fs::read_to_string(&config_path)
        .unwrap_or_else(|err| {
            error!("failed to read config file: {}",err);
            panic!("failed to read config file: {}",err);
        });

    toml::from_str(&config_text)
        .unwrap_or_else(|err| {
            error!("failed to parse configuration file: {}", err);
            panic!("failed to parse configuration file: {}", err);
        })
}



