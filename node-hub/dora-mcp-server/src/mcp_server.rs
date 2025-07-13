use std::collections::HashMap;
use std::sync::Mutex;

use dora_node_api::arrow::array::{AsArray, StringArray};
use dora_node_api::{
    dora_core::config::DataId, ArrowData, DoraNode, Metadata, MetadataParameters, Parameter,
};
use rmcp::model::{
    CallToolResult, EmptyResult, Implementation, InitializeResult, JsonObject, ListToolsResult,
    ProtocolVersion, Request, ServerCapabilities, ServerInfo, ServerResult, Tool,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;
use futures::channel::oneshot;

use crate::ServerEvent;

#[derive(Debug)]
pub struct McpServer {
    tools: Vec<Tool>,
    server_info: Implementation,
}

impl McpServer {
    pub fn new(tools: Vec<Tool>, server_info: Implementation) -> Self {
        Self { tools, server_info }
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
        request_tx: &mpsc::Sender<ServerEvent>,
    ) -> eyre::Result<CallToolResult> {
        let (tx, rx) = oneshot::channel();

        request_tx
            .send(ServerEvent::CallNode {
                node_id: "node_id".into(), // TODO
                data: serde_json::to_string(&params).unwrap(),
                reply: tx,
            })
            .await?;

        let data: String = rx.await?;
        Ok(serde_json::from_str(&data)
            .map_err(|e| eyre::eyre!("Failed to parse call tool result: {}", e))?)
    }

    pub async fn handle_request(
        &self,
        request: Request,
        server_events_tx: &mpsc::Sender<ServerEvent>,
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
                .handle_tools_call(params, server_events_tx)
                .await
                .map(|result| ServerResult::CallToolResult(result)),
            method => Err(eyre::eyre!("unexpected method: {:#?}", method)),
        }
    }

}

pub(crate) fn gen_call_id() -> String {
    format!("call-{}", uuid::Uuid::new_v4())
}
