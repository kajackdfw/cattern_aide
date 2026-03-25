use super::agent::{AgentKind, AgentState, Provider};
use super::container::TextContainer;
use super::filetree::FileTreeState;
use super::git::GitStatus;

#[derive(Clone, Debug)]
pub struct ConversationMessage {
    pub role:    String,   // "user" or "assistant"
    pub content: String,
}

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
    pub name:             String,
    pub provider:         Provider,
    pub path:             String,
    pub tabs:             Vec<HorizontalTab>,
    pub active_tab:       usize,
    pub prompt_input:     String,
    pub prompt_cursor:    usize,
    pub file_tree:        FileTreeState,
    pub git_status:       GitStatus,
    /// Full conversation sent to the AI on each turn.
    pub conversation:     Vec<ConversationMessage>,
    /// Accumulates the streaming assistant reply; finalised on Idle/Error.
    pub pending_response: String,
}

impl Project {
    pub fn new(name: String, provider: Provider, path: String) -> Self {
        let tabs = vec![
            HorizontalTab::new(AgentKind::AiPrompt,  "AI Prompt"),
            HorizontalTab::new(AgentKind::TargetCode, "Target Code"),
            HorizontalTab::new(AgentKind::Git,        "Git"),
        ];
        let file_tree  = FileTreeState::new(&path);
        let git_status = GitStatus::new(&path);
        Self {
            name, provider, path, tabs, active_tab: 0,
            prompt_input: String::new(), prompt_cursor: 0,
            file_tree, git_status,
            conversation: Vec::new(),
            pending_response: String::new(),
        }
    }
}
