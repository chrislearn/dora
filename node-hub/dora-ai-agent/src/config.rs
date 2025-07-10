use std::collections::HashMap;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::{LazyLock, OnceLock};

use figment::providers::{Env, Format, Json, Toml, Yaml};
use figment::Figment;
use rmcp::model::{ServerInfo, Tool};
use serde::Deserialize;

pub static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn init() {
    let config_file = Env::var("CONFIG").unwrap_or("config.toml".into());
    let config_path = PathBuf::from(config_file);
    if !config_path.exists() {
        eprintln!("Config file not found at: {}", config_path.display());
        std::process::exit(1);
    }

    let raw_config = match config_path
        .extension()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
    {
        "yaml" | "yml" => Figment::new().merge(Yaml::file(config_path)),
        "json" => Figment::new().merge(Json::file(config_path)),
        "toml" => Figment::new().merge(Toml::file(config_path)),
        ext => {
            eprintln!("unsupport config file format: {ext:?}");
            std::process::exit(1);
        }
    };

    let conf = match raw_config.extract::<Config>() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("It looks like your config is invalid. The following error occurred: {e}");
            std::process::exit(1);
        }
    };

    CONFIG.set(conf).expect("config should be set");
}
pub fn get() -> &'static Config {
    CONFIG.get().unwrap()
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,

    pub mcp_servers: Vec<McpServerConfig>,
}
fn default_listen_addr() -> String {
    "0.0.0.0:8008".to_owned()
}

#[derive(Clone, Debug, Deserialize)]
pub struct McpServerConfig {
    pub tools: HashMap<String, Tool>,
}
