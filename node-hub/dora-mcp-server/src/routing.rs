use std::sync::Arc;

use eyre::{Context, ContextCompat};
use futures::channel::oneshot;
use futures::TryStreamExt;
use salvo::prelude::*;
use salvo::serve_static::static_embed;
use tokio::sync::mpsc;

use crate::{AppError, AppResult, ServerEvent, McpServer};

pub fn root(server_events_tx: mpsc::Sender<ServerEvent>, mcp_server: Arc<McpServer>) -> Router {
    Router::with_hoop(affix_state::inject(server_events_tx).inject(mcp_server)).push(
        Router::with_path("mcp")
            .post(handle_post)
            .delete(handle_delete),
    )
}

#[handler]
async fn handle_delete(res: &mut Response) {
    res.render(Text::Plain("TODO"));
}

#[handler]
async fn handle_post(req: &mut Request, depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    tracing::info!("Handling the coming chat completion request.");
    let request_tx = depot
        .obtain::<mpsc::Sender<ServerEvent>>()
        .expect("request_tx must be exists");
    let mcp_server = depot
        .obtain::<Arc<McpServer>>()
        .expect("mcp server must be exists");

    tracing::info!("Prepare the chat completion request.");

    let mut chat_request = req.parse_json::<CompletionRequest>().await?;

    let response = mcp_server.handle_request(chat_request).await.unwrap();
    let _ = res.add_header("user", id, true);
    res.render(Json(response));
    tracing::info!("Send the chat completion response.");
    Ok(())
}
