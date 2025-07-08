use salvo::prelude::*;
use tokio::sync::mpsc;

#[derive(RustEmbed)]
#[folder = "static"]
struct Assets;

pub fn root(server_events_tx: mpsc::Sender<ServerEvent>) -> Router {
    Router::with_hoop(affix_state::inject(config).inject(server_events_tx))
        .hoop(
            Cors::new()
                .allow_origin(AllowOrigin::any())
                .allow_methods(AllowMethods::any())
                .allow_headers(AllowHeaders::any())
                .into_handler(),
        )
        .get(index)
        .push(
            Router::with_path("v1")
                .get(index)
                .push(Router::with_path("chat/completions").get(index))
                .push(Router::with_path("models").get(todo))
                .push(Router::with_path("embeddings").get(todo))
                .push(Router::with_path("files").get(todo))
                .push(Router::with_path("chunks").get(todo))
                .push(Router::with_path("info").get(todo))
                .push(Router::with_path("realtime").get(todo)),
        )
        .push(Router::with_path("{**path}").get(static_embed::<Assets>().defaults("index.html")))
}

#[handle]
async fn todo() {
    res.render(Text::Plain("TODO"));
}

#[handle]
async fn chat_completions(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
    body: JsonBody<ChatCompletionRequest>,
) {
    info!(target: "stdout", "Handling the coming chat completion request.");
    let request_tx = depot
        .obtain::<mpsc::Sender<ServerEvent>>()
        .expect("request_tx must be exists");

    info!(target: "stdout", "Prepare the chat completion request.");

    let mut chat_request = body.into_inner();

    // check if the user id is provided
    if chat_request.user.is_none() {
        chat_request.user = Some(utils::gen_chat_id())
    };
    let id = chat_request.user.clone().unwrap();

    // log user id
    info!(target: "stdout", "user: {}", chat_request.user.clone().unwrap());
    let stream = chat_request.stream;

    let (tx, rx) = oneshot::channel();
    if let Err(err) = request_tx
        .send(ServerEvent::ChatCompletionRequest {
            request: chat_request,
            reply: tx,
        })
        .await
        .context("failed to send request")
    {
        return error::internal_server_error(format!("{err:?}"));
    }

    let res = if let Some(true) = stream {
        let result = async {
            let chat_completion_object = rx
                .await
                .unwrap_or_else(|Canceled| Err(eyre::eyre!("result channel closed early")))?;
            serde_json::to_string(&chat_completion_object).context("failed to serialize response")
        };
        let stream = futures::stream::once(result).map_err(|e| e.to_string());

        res.header("Content-Type", "text/event-stream");
        res.header("Cache-Control", "no-cache");
        res.header("Connection", "keep-alive");
        res.header("user", id);
        res.stream(stream);
    } else {
        match rx
            .await
            .unwrap_or_else(|Canceled| Err(eyre::eyre!("result channel closed early")))
        {
            Ok(chat_completion_object) => {
                res.header("user", id);
                res.sender(Json(chat_completion_object));
            }
            Err(e) => {
                let err_msg = format!("Failed to get chat completions. Reason: {}", e);
                error!(target: "stdout", "{}", &err_msg);
                error::internal_server_error(err_msg)
            }
        }
    };
    info!(target: "stdout", "Send the chat completion response.");
}
