use std::collections::VecDeque;

use dora_node_api::{
    self,
    arrow::array::{AsArray, StringArray},
    dora_core::config::DataId,
    merged::{MergeExternalSend, MergedEvent},
    DoraNode, Event,
};

use eyre::{Context, ContextCompat};
use futures::channel::oneshot;
use rmcp::model::Request;
use tokio::sync::mpsc;

mod mcp_server;
use mcp_server::McpServer;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let (mut node, events) = DoraNode::init_from_env()?;

    let events = futures::executor::block_on_stream(events);

    let server = McpServer::new(vec![], Default::default());

    for event in events {
        match event {
            Event::Input {
                id,
                data,
                metadata: _,
            } => {
                match id.as_str() {
                    "request" => {
                        let data =
                            data.as_string::<i32>()
                                .iter()
                                .fold("".to_string(), |mut acc, s| {
                                    if let Some(s) = s {
                                        acc.push('\n');
                                        acc.push_str(s);
                                    }
                                    acc
                                });

                        let request = serde_json::from_str::<Request>(&data)
                            .context("failed to parse call tool from string")?;

                        server.handle_request(request);
                    }
                    _ => {
                        node.send_output(DataId::from("response".to_owned()), metadata, data.0)
                            .context("failed to send dora output")?;
                    }
                };
            }
            Event::Stop(_) => {
                break;
            }
            Event::InputClosed { id, .. } => {
                tracing::info!("Input channel closed for id: {}", id);
            }
            event => {
                eyre::bail!("unexpected event: {:#?}", event)
            }
        }
    }

    Ok(())
}
