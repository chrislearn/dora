use std::sync::Arc;

use salvo::prelude::*;
use tokio::sync::mpsc;

use crate::models::*;
use crate::session::ChatSession;
use crate::{AppResult, ServerEvent};


pub fn root(server_events_tx: mpsc::Sender<ServerEvent>, chat_session: Arc<ChatSession>) -> Router {
    Router::with_hoop(affix_state::inject(server_events_tx).inject(chat_session))
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
        .push(Router::with_path("{**path}").get(index))
}

#[handler]
async fn todo(res: &mut Response) {
    res.render(Text::Plain("TODO"));
}
#[handler]
async fn index(res: &mut Response) {
    res.render(Text::Plain("Hello"));
}

#[handler]
async fn chat_completions(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    tracing::info!("Handling the coming chat completion request.");
    let request_tx = depot
        .obtain::<mpsc::Sender<ServerEvent>>()
        .expect("request_tx must be exists");
    let chat_session = depot
        .obtain::<Arc<ChatSession>>()
        .expect("chat session must be exists");

    tracing::info!("Prepare the chat completion request.");

    let mut chat_request = req.parse_json::<CompletionRequest>().await?;

    // check if the user id is provided
    if chat_request.user.is_none() {
        chat_request.user = Some(crate::utils::gen_chat_id())
    };
    let id = chat_request.user.clone().unwrap();

    // log user id
    tracing::info!("user: {}", chat_request.user.clone().unwrap());
    let stream = chat_request.stream;

    // let (tx, rx) = oneshot::channel();
    // request_tx
    //     .send(ServerEvent::CompletionRequest {
    //         request: chat_request,
    //         reply: tx,
    //     })
    //     .await?;

    // if let Some(true) = stream {
    //     // let result = async {
    //     //     let chat_completion_object = rx.await?;
    //     //     Ok::<_, AppError>(serde_json::to_string(&chat_completion_object)?)
    //     // };
    //     let result = chat_session.chat(chat_request).await?;
    //     let stream = futures::stream::once(result);

    //     let _ = res.add_header("Content-Type", "text/event-stream", true);
    //     let _ = res.add_header("Cache-Control", "no-cache", true);
    //     let _ = res.add_header("Connection", "keep-alive", true);
    //     let _ = res.add_header("user", id, true);
    //     res.stream(stream);
    // } else {
        let response = chat_session.chat(chat_request).await.unwrap();
        let _ = res.add_header("user", id, true);
        res.render(Json(response));
    // };
    tracing::info!("Send the chat completion response.");
    Ok(())
}
