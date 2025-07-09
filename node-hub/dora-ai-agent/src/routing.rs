use eyre::Context;
use futures::channel::oneshot;
use futures::TryStreamExt;
use rust_embed::RustEmbed;
use salvo::prelude::*;
use salvo::serve_static::static_embed;
use tokio::sync::mpsc;

use crate::models::*;
use crate::{AppResult, ServerEvent};

#[derive(RustEmbed)]
#[folder = "static"]
struct Assets;

pub fn root(server_events_tx: mpsc::Sender<ServerEvent>) -> Router {
    Router::with_hoop(affix_state::inject(server_events_tx))
        .push(
            Router::with_path("v1")
                .push(Router::with_path("chat/completions").post(chat_completions))
                .push(Router::with_path("models").get(todo))
                .push(Router::with_path("embeddings").get(todo))
                .push(Router::with_path("files").get(todo))
                .push(Router::with_path("chunks").get(todo))
                .push(Router::with_path("info").get(todo))
                .push(Router::with_path("realtime").get(todo)),
        )
        .push(Router::with_path("{**path}").get(static_embed::<Assets>().defaults("index.html")))
}

#[handler]
async fn todo(res: &mut Response) {
    res.render(Text::Plain("TODO"));
}

#[handler]
async fn chat_completions(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    tracing::info!(target: "stdout", "Handling the coming chat completion request.");
    let request_tx = depot
        .obtain::<mpsc::Sender<ServerEvent>>()
        .expect("request_tx must be exists");

    tracing::info!(target: "stdout", "Prepare the chat completion request.");

    let mut chat_request = req.parse_json::<ChatCompletionRequest>().await?;

    // check if the user id is provided
    if chat_request.user.is_none() {
        chat_request.user = Some(crate::utils::gen_chat_id())
    };
    let id = chat_request.user.clone().unwrap();

    // log user id
    tracing::info!(target: "stdout", "user: {}", chat_request.user.clone().unwrap());
    let stream = chat_request.stream;

    let (tx, rx) = oneshot::channel();
    request_tx
        .send(ServerEvent::ChatCompletionRequest {
            request: chat_request,
            reply: tx,
        })
        .await?;

    if let Some(true) = stream {
        let result = async {
            let chat_completion_object = rx.await?;
            serde_json::to_string(&chat_completion_object).context("failed to serialize response")
        };
        let stream = futures::stream::once(result).map_err(|e| e.to_string());

        let _ = res.add_header("Content-Type", "text/event-stream", true);
        let _ = res.add_header("Cache-Control", "no-cache", true);
        let _ = res.add_header("Connection", "keep-alive", true);
        let _ = res.add_header("user", id, true);
        res.stream(stream);
    } else {
        let chat_completion_object = rx.await?;
        let _ = res.add_header("user", id, true);
        res.render(Json(chat_completion_object));
    };
    tracing::info!(target: "stdout", "Send the chat completion response.");
    Ok(())
}
