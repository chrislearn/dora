use std::sync::Arc;

use dora_node_api::ArrowData;
use eyre::{Context, ContextCompat};
use futures::channel::oneshot;
use futures::TryStreamExt;
use salvo::prelude::*;
use salvo::serve_static::static_embed;
use tokio::sync::mpsc;

use crate::{AppError, AppResult, McpServer};

pub fn root(mcp_server: Arc<McpServer>) -> Router {
    Router::with_hoop(affix_state::inject(mcp_server)).push(
        Router::with_path("mcp")
            .post(handle_post)
            .delete(handle_delete),
    )
}

#[handler]
async fn handle_delete(res: &mut Response) {
    res.render(Text::Plain("DELETE method is not supported"));
}

#[handler]
async fn handle_post(req: &mut Request, depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    tracing::info!("Handling the coming chat completion request.");
    let mcp_server = depot
        .obtain::<Arc<McpServer>>()
        .expect("mcp server must be exists");

    tracing::info!("Prepare the chat completion request.");

    let rcp_request = serde_json::from_slice::<rmcp::model::Request>(&req.payload().await?)
        .context("failed to parse request body")?;
    let response = mcp_server.handle_request(rcp_request).await.unwrap();
    res.render(Json(response));
    tracing::info!("Send the chat completion response.");
    Ok(())
}
