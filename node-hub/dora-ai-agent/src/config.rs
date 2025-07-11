use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::{LazyLock, OnceLock};
use std::{collections::HashMap, path::Path, process::Stdio};

use figment::providers::{Env, Format, Json, Toml, Yaml};
use figment::Figment;
use rmcp::model::{ServerInfo, Tool};
use rmcp::{service::RunningService, transport::ConfigureCommandExt, RoleClient, ServiceExt};
use serde::{Deserialize, Serialize};

use crate::client::GeminiClient;
use crate::{ChatSession, ToolSet};

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

    pub gemini: Option<GeminiConfig>,

    pub mcp: Option<McpConfig>,
    #[serde(default = "default_false")]
    pub support_tool: bool,
}
fn default_listen_addr() -> String {
    "0.0.0.0:8008".to_owned()
}
fn default_false() -> bool {
    false
}
fn default_gemini_api_url() -> String {
    "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent"
        .to_owned()
}

#[derive(Clone, Debug, Deserialize)]
pub struct GeminiConfig {
    pub api_key: String,
    #[serde(default = "default_gemini_api_url")]
    pub api_url: String,
    #[serde(default = "default_false")]
    pub proxy: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct McpConfig {
    pub server: Vec<McpServerConfig>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpServerConfig {
    pub name: String,
    #[serde(flatten)]
    pub transport: McpServerTransportConfig,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "protocol", rename_all = "lowercase")]
pub enum McpServerTransportConfig {
    Streamable {
        url: String,
    },
    Sse {
        url: String,
    },
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        envs: HashMap<String, String>,
    },
}

impl McpServerTransportConfig {
    pub async fn start(&self) -> eyre::Result<RunningService<RoleClient, ()>> {
        let client = match self {
            McpServerTransportConfig::Streamable { url } => {
                let transport =
                    rmcp::transport::StreamableHttpClientTransport::from_uri(url.to_string());
                ().serve(transport).await?
            }
            McpServerTransportConfig::Sse { url } => {
                let transport =
                    rmcp::transport::sse_client::SseClientTransport::start(url.to_owned()).await?;
                ().serve(transport).await?
            }
            McpServerTransportConfig::Stdio {
                command,
                args,
                envs,
            } => {
                let transport = rmcp::transport::TokioChildProcess::new(
                    tokio::process::Command::new(command).configure(|cmd| {
                        cmd.args(args)
                            .envs(envs)
                            .stderr(Stdio::inherit())
                            .stdout(Stdio::inherit());
                    }),
                )?;
                ().serve(transport).await?
            }
        };
        Ok(client)
    }
}

impl Config {
    pub async fn create_mcp_clients(
        &self,
    ) -> eyre::Result<HashMap<String, RunningService<RoleClient, ()>>> {
        let mut clients = HashMap::new();

        if let Some(mcp_config) = &self.mcp {
            for server in &mcp_config.server {
                let client = server.transport.start().await?;
                clients.insert(server.name.clone(), client);
            }
        }

        Ok(clients)
    }

    pub async fn create_session(&self) -> eyre::Result<ChatSession> {
        let mut tool_set = ToolSet::default();

        if self.mcp.is_some() {
            let mcp_clients = self.create_mcp_clients().await?;

            for (name, client) in mcp_clients {
                println!("load MCP tool: {}", name);
                let server = client.peer().clone();
                let tools = crate::get_mcp_tools(server).await?;

                for tool in tools {
                    tool_set.add_tool(tool);
                }
            }
        }

        let gemini_config = self.gemini.as_ref().ok_or_else(|| {
            eyre::eyre!("Gemini configuration is missing. Please check your config file.")
        })?;
        let gemini_client = Arc::new(GeminiClient::new(gemini_config));
        Ok(ChatSession::new(
            gemini_client,
            tool_set,
            Some("gpt-4o-mini".to_string()),
        ))
    }
}
