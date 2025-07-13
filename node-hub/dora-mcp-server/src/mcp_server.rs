use std::{collections::HashMap, fs::Metadata};

use dora_node_api::{dora_core::config::DataId, ArrowData};
use rmcp::model::{JsonObject, Request, ServerInfo, Tool};
use serde::{Deserialize, Serialize};
use serde_json::Value;use tokio::sync::oneshot;

#[derive(Deserialize, Debug, Clone)]
pub struct McpServer {
    tools: Vec<Tool>,
    info: ServerInfo,
    channels: HashMap,
}

impl McpServer {
    pub fn new(tools: Vec<Tool>, info: ServerInfo) -> Self {
        Self {
            tools,
            info,
            channels: Default::default(),
        }
    }

    pub fn tools(&self) -> &[Tool] {
        &self.tools
    }

    pub fn info(&self) -> &ServerInfo {
        &self.info
    }

    pub async fn handle_ping(&self) {}
    pub async fn handle_tools_list(&self) -> eyre::Result<Vec<Tool>> {}
    pub async fn handle_tools_call(&self, params: JsonObject, mut metadata: Metadata) {
        let (tx, rx) = oneshot::channel();
        metadata.insert("__dora_call_id".to_string(), Value::String(gen_call_id()));
        self.channels.insert(params.call_id.clone(), tx);
        Ok(rx.await)
    }

    pub async fn handle_request(&self, request: Request, metata: Metadata) -> eyre::Result<Option<Response<Value>>> {
        let Request {
            method,
            params,
            extensions,
        } = request;
        match method.as_str() {
            "ping" => self.handle_ping(),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tools_call(params, metata),
            method => {
                tracing::error!("unexpected method: {:#?}", method)
            }
        }
    }

    pub fn handle_event(
        &self,
        id: DataId,
        data: ArrowData,
        metadata: &Metadata,
    ) -> eyre::Result<()> {
        let Some(call_id) = metadata.get("__dora_call_id").and_then(Value::as_str) else {
            return Ok(());
        };
        let Some(sender) = reply_channels.remove(call_id) else {
            return Ok(());
        };
        match event {
            ServerEvent::Result(result) => result,
            ServerEvent::CompletionRequest { request, metadata } => {
                let call_id = metadata.get("call_id").and_then(Value::as_str);
                if let Some(call_id) = call_id {
                    // Handle the completion request
                    tracing::info!("Handling completion request for call ID: {}", call_id);
                } else {
                    tracing::warn!("No call ID found in metadata");
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn gen_call_id() -> String {
    format!("call-{}", uuid::Uuid::new_v4())
}
