pub(crate) fn gen_call_id() -> String {
    format!("call-{}", uuid::Uuid::new_v4())
}
pub(crate) fn gen_chat_id() -> String {
    format!("chatcmpl-{}", uuid::Uuid::new_v4())
}
