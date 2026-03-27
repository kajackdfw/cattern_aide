use super::agent::{AgentKind, AgentState, Provider};
use super::container::TextContainer;
use super::filetree::FileTreeState;
use super::git::GitStatus;
use super::pty_screen::PtyScreen;

/// A single tab in the right-column viewer.
pub struct TargetTab {
    pub label:   String,
    pub content: TextContainer,
    pub is_diff: bool,
}

#[derive(Clone, Debug)]
pub struct ConversationMessage {
    pub role:    String,   // "user" or "assistant"
    pub content: String,
}

pub struct HorizontalTab {
    pub kind:       AgentKind,
    pub state:      AgentState,
    pub content:    TextContainer,
    pub label:      String,
    pub command:    Option<String>,  // for Process tabs
    pub pty_screen: PtyScreen,      // live screen grid for PTY tabs
}

impl HorizontalTab {
    pub fn new(kind: AgentKind, label: impl Into<String>) -> Self {
        Self {
            kind,
            state:      AgentState::Idle,
            content:    TextContainer::new(),
            label:      label.into(),
            command:    None,
            pty_screen: Vec::new(),
        }
    }

    pub fn with_command(mut self, cmd: impl Into<String>) -> Self {
        self.command = Some(cmd.into());
        self
    }
}

pub struct Project {
    pub name:              String,
    pub provider:          Provider,
    pub path:              String,
    pub tabs:              Vec<HorizontalTab>,
    pub active_tab:        usize,
    pub prompt_input:      String,
    pub prompt_cursor:     usize,
    pub file_tree:         FileTreeState,
    pub git_status:        GitStatus,
    /// Full conversation sent to the AI on each turn.
    pub conversation:      Vec<ConversationMessage>,
    /// Accumulates the streaming assistant reply; finalised on Idle/Error.
    pub pending_response:  String,
    /// Right-panel multi-tab viewer. Newest tab is always at index 0.
    pub target_tabs:       Vec<TargetTab>,
    pub active_target_tab: usize,
    /// Index of the first tab shown in the tab bar (for horizontal scrolling).
    pub target_tab_offset: usize,
    /// Color scheme name ("dawn", "matrix").
    pub theme:             String,
}

impl Project {
    pub fn new(name: String, provider: Provider, path: String) -> Self {
        let tabs = vec![
            HorizontalTab::new(AgentKind::AiPrompt, "AI Prompt"),
            HorizontalTab::new(AgentKind::Git,       "Git"),
        ];
        let file_tree  = FileTreeState::new(&path);
        let git_status = GitStatus::new(&path);
        Self {
            name, provider, path, tabs, active_tab: 0,
            prompt_input: String::new(), prompt_cursor: 0,
            file_tree, git_status,
            conversation: Vec::new(),
            pending_response: String::new(),
            target_tabs:       Vec::new(),
            active_target_tab: 0,
            target_tab_offset: 0,
            theme: "dawn".into(),
        }
    }
}
