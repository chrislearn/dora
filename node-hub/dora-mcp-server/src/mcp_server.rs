use std::collections::HashMap;
use std::sync::Mutex;

use dora_node_api::{dora_core::config::DataId, ArrowData, Metadata, Parameter};
use rmcp::model::{
    CallToolResult, EmptyResult, Implementation, InitializeResult, JsonObject, ListToolsResult,
    ProtocolVersion, Request, ServerCapabilities, ServerInfo, ServerResult, Tool,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::oneshot;

#[derive(Debug)]
pub struct McpServer {
    tools: Vec<Tool>,
    server_info: Implementation,
    reply_channels: Mutex<HashMap<String, oneshot::Sender<ArrowData>>>,
}

impl McpServer {
    pub fn new(tools: Vec<Tool>, server_info: Implementation) -> Self {
        Self {
            tools,
            server_info,
            reply_channels: Default::default(),
        }
    }

    pub fn tools(&self) -> &[Tool] {
        &self.tools
    }

    pub fn server_info(&self) -> &Implementation {
        &self.server_info
    }

    pub async fn handle_ping(&self) -> eyre::Result<EmptyResult> {
        Ok(EmptyResult {})
    }
    pub async fn handle_initialize(&self) -> eyre::Result<InitializeResult> {
        Ok(InitializeResult {
            protocol_version: ProtocolVersion::V_2025_03_26,
            server_info: self.server_info.clone(),
            capabilities: ServerCapabilities::default(),
            instructions: None,
        })
    }
    pub async fn handle_tools_list(&self) -> eyre::Result<ListToolsResult> {
        Ok(ListToolsResult {
            tools: self.tools.clone(),
            next_cursor: None,
        })
    }
    pub async fn handle_tools_call(
        &self,
        params: JsonObject,
        mut metadata: Metadata,
    ) -> eyre::Result<CallToolResult> {
        let (tx, rx) = oneshot::channel();
        let call_id = gen_call_id();
        metadata
            .parameters
            .insert("__dora_call_id".to_string(), Parameter::String(call_id));
        let reply_channels = self
            .reply_channels
            .lock()
            .map_err(|_| eyre::eyre!("Failed to lock reply channels"))?;
        reply_channels.insert(call_id, tx);
        Ok(rx.await)
    }

    pub async fn handle_request(
        &self,
        request: Request,
        metata: Metadata,
    ) -> eyre::Result<ServerResult> {
        let Request {
            method,
            params,
            extensions,
        } = request;
        match method.as_str() {
            "ping" => self
                .handle_ping()
                .await
                .map(|result| ServerResult::EmptyResult(result)),
            "initialize" => self
                .handle_initialize()
                .await
                .map(|result| ServerResult::InitializeResult(result)),
            "tools/list" => self
                .handle_tools_list()
                .await
                .map(|result| ServerResult::ListToolsResult(result)),
            "tools/call" => self
                .handle_tools_call(params, metata)
                .await
                .map(|result| ServerResult::CallToolResult(result)),
            method => Err(eyre::eyre!("unexpected method: {:#?}", method)),
        }
    }

    pub fn handle_event(
        &self,
        id: DataId,
        data: ArrowData,
        metadata: Metadata,
    ) -> eyre::Result<()> {
        let Some(Parameter::String(call_id)) = metadata.parameters.get("__dora_call_id") else {
            return Ok(());
        };
        let reply_channels = self
            .reply_channels
            .lock()
            .map_err(|_| eyre::eyre!("Failed to lock reply channels"))?;
        let Some(sender) = reply_channels.remove(call_id) else {
            return Ok(());
        };
        sender.send(data)?;
        Ok(())
    }
}

pub(crate) fn gen_call_id() -> String {
    format!("call-{}", uuid::Uuid::new_v4())
}
