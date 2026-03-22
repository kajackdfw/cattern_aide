#[allow(dead_code)]
pub struct OpenAiApiAgent;
// Phase 2: POST to {api_base}/chat/completions with stream:true,
// parse choices[0].delta.content SSE events (OpenAI-compat for opencode).
