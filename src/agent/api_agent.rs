#[allow(dead_code)]
pub struct AnthropicApiAgent;
// Phase 2: POST to api.anthropic.com/v1/messages with stream:true,
// parse content_block_delta SSE events, send AgentMessage::Chunk per token.
