use std::collections::HashMap;
use std::sync::Arc;

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
use salvo::cors::*;
use salvo::prelude::*;
use tokio::sync::mpsc;

mod mcp_server;
use mcp_server::McpServer;
mod error;
mod routing;
use error::AppError;

pub type AppResult<T> = Result<T, crate::AppError>;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let (mut node, events) = DoraNode::init_from_env()?;

    let events = futures::executor::block_on_stream(events);

    let mcp_server = Arc::new(McpServer::new(vec![], Default::default()));

    let acceptor = TcpListener::new("0.0.0.0:8008").bind().await;
    tokio::spawn(async move {
        let service = Service::new(routing::root(mcp_server.clone()))
            .hoop(
                Cors::new()
                    .allow_origin(AllowOrigin::any())
                    .allow_methods(AllowMethods::any())
                    .allow_headers(AllowHeaders::any())
                    .into_handler(),
            );
        Server::new(acceptor).serve(service).await;
    });

    for event in events {
        match event {
            Event::Input {
                id,
                data,
                metadata,
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

                        if let Ok(result) = mcp_server.handle_request(request, metadata).await {
                            node.send_output(DataId::from("response".to_owned()), metadata.parameters, result)
                                .context("failed to send dora output")?;
                        }
                    }
                    _ => {
                        mcp_server.handle_event(id, data, metadata)?;
                        // node.send_output(DataId::from("response".to_owned()), metadata, data)
                        //     .context("failed to send dora output")?;
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
