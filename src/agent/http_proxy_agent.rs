#[allow(dead_code)]
pub struct HttpProxyAgent;
// Phase 2: axum listener on port, forward requests to target via reqwest,
// log request/response as AgentMessage::Chunk. Ctrl-K cancels via oneshot.
