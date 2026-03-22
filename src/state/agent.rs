#[derive(Debug, Clone, PartialEq)]
pub enum AgentState {
    Idle,
    Running,
    Waiting,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentKind {
    AiPrompt,
    TargetCode,
    Named(String),
    Process(String),
    HttpProxy { port: u16, target: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Provider {
    Anthropic,
    OpenCode,
}

impl Default for Provider {
    fn default() -> Self {
        Provider::Anthropic
    }
}
