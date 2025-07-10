use serde::{Deserialize, Serialize};
use serde_json::Value;
use rmcp::model::{Request, Tool, ServerInfo, JsonObject};

#[derive(Deserialize, Debug, Clone)]
pub struct McpServer {
    tools: Vec<Tool>,
    info: ServerInfo,
}

impl McpServer {
    pub fn new(tools: Vec<Tool>, info: ServerInfo) -> Self {
        Self { tools, info }
    }

    pub fn tools(&self) -> &[Tool] {
        &self.tools
    }

    pub fn info(&self) -> &ServerInfo {
        &self.info
    }

    pub fn handle_ping(&self) {}
    pub fn handle_tools_list(&self) {}
    pub fn handle_tool_call(&self, params: JsonObject) {}

    pub fn handle_request(&self, request: Request) {
        let Request {
            method,
            params,
            extensions,
        } = request;
        match method.as_str() {
            "ping" => self.handle_ping(),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tool_call(params),
            method => {
                tracing::error!("unexpected method: {:#?}", method)
            }
        }
    }
}
