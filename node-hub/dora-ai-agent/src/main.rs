use std::collections::VecDeque;

use dora_node_api::{
    arrow::array::{AsArray, StringArray},
    dora_core::config::DataId,
    merged::{MergeExternalSend, MergedEvent},
    DoraNode, Event,
};
use eyre::{Context, ContextCompat};

use futures::channel::oneshot;
use salvo::cors::*;
use salvo::prelude::*;
use tokio::sync::mpsc;

mod client;
mod models;
mod routing;
mod utils;
use models::*;
mod error;
use error::AppError;
mod config;
mod session;
use session::ChatSession;
mod tool;
use tool::{get_mcp_tools, Tool, ToolSet};

pub type AppResult<T> = Result<T, crate::AppError>;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    config::init();

    let (server_events_tx, server_events_rx) = mpsc::channel(3);
    let server_events = tokio_stream::wrappers::ReceiverStream::new(server_events_rx);

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    tokio::spawn(async move {
        let service = Service::new(routing::root(server_events_tx.clone())).hoop(
            Cors::new()
                .allow_origin(AllowOrigin::any())
                .allow_methods(AllowMethods::any())
                .allow_headers(AllowHeaders::any())
                .into_handler(),
        );
        Server::new(acceptor).serve(service).await;
        if let Err(err) = server_events_tx.send(ServerEvent::Result(Ok(()))).await {
            tracing::warn!("server result channel closed: {err}");
        }
    });

    let mut tool_set = ToolSet::default();

    let config = config::get();
    // load MCP
    if config.mcp.is_some() {
        let mcp_clients = config.create_mcp_clients().await?;

        for (name, client) in mcp_clients {
            println!("load MCP tool: {}", name);
            let server = client.peer().clone();
            let tools = get_mcp_tools(server).await?;

            for tool in tools {
                println!("add tool: {}", tool.name());
                tool_set.add_tool(tool);
            }
        }
    }

    let (mut node, events) = DoraNode::init_from_env()?;

    let merged = events.merge_external_send(server_events);
    let events = futures::executor::block_on_stream(merged);

    let output_id = DataId::from("text".to_owned());
    let mut reply_channels = VecDeque::new();

    for event in events {
        match event {
            MergedEvent::External(event) => match event {
                ServerEvent::Result(server_result) => {
                    server_result.context("server failed")?;
                    break;
                }
                ServerEvent::CompletionRequest { request, reply } => {
                    let texts = request.to_texts();
                    node.send_output(
                        output_id.clone(),
                        Default::default(),
                        StringArray::from(texts),
                    )
                    .context("failed to send dora output")?;

                    reply_channels.push_back((reply, 0 as u64, request.model));
                }
            },
            MergedEvent::Dora(event) => match event {
                Event::Input {
                    id,
                    data,
                    metadata: _,
                } => {
                    match id.as_str() {
                        "text" => {
                            let (reply_channel, prompt_tokens, model) =
                                reply_channels.pop_front().context("no reply channel")?;
                            let data = data.as_string::<i32>();
                            let string = data.iter().fold("".to_string(), |mut acc, s| {
                                if let Some(s) = s {
                                    acc.push('\n');
                                    acc.push_str(s);
                                }
                                acc
                            });

                            let data = ChatCompletionObject {
                                id: format!("completion-{}", uuid::Uuid::new_v4()),
                                object: "chat.completion".to_string(),
                                created: chrono::Utc::now().timestamp() as u64,
                                model: model.unwrap_or_default(),
                                choices: vec![ChatCompletionObjectChoice {
                                    index: 0,
                                    message: ChatCompletionObjectMessage {
                                        role: ChatCompletionRole::Assistant,
                                        content: Some(string.to_string()),
                                        tool_calls: Vec::new(),
                                        function_call: None,
                                    },
                                    finish_reason: FinishReason::stop,
                                    logprobs: None,
                                }],
                                usage: Usage {
                                    prompt_tokens,
                                    completion_tokens: string.len() as u64,
                                    total_tokens: prompt_tokens + string.len() as u64,
                                },
                            };

                            if reply_channel.send(data).is_err() {
                                tracing::warn!("failed to send chat completion reply because channel closed early");
                            }
                        }
                        _ => eyre::bail!("unexpected input id: {}", id),
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
            },
        }
    }

    Ok(())
}

enum ServerEvent {
    Result(eyre::Result<()>),
    CompletionRequest {
        request: CompletionRequest,
        reply: oneshot::Sender<ChatCompletionObject>,
    },
}
