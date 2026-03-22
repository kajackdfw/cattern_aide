use super::agent::{AgentKind, AgentState, Provider};
use super::container::TextContainer;

pub struct HorizontalTab {
    pub kind:    AgentKind,
    pub state:   AgentState,
    pub content: TextContainer,
    pub label:   String,
}

impl HorizontalTab {
    pub fn new(kind: AgentKind, label: impl Into<String>) -> Self {
        Self {
            kind,
            state:   AgentState::Idle,
            content: TextContainer::new(),
            label:   label.into(),
        }
    }
}

pub struct Project {
    pub name:         String,
    pub provider:     Provider,
    pub path:         String,
    pub tabs:         Vec<HorizontalTab>,
    pub active_tab:   usize,
    pub prompt_input: String,
}

impl Project {
    pub fn new(name: String, provider: Provider, path: String) -> Self {
        let tabs = vec![
            HorizontalTab::new(AgentKind::AiPrompt,   "AI Prompt"),
            HorizontalTab::new(AgentKind::TargetCode,  "Target Code"),
        ];
        Self { name, provider, path, tabs, active_tab: 0, prompt_input: String::new() }
    }
}
