pub mod api_agent;
pub mod openai_api_agent;
pub mod subprocess_agent;
pub mod http_proxy_agent;

use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use crate::{
    config::Config,
    state::{
        agent::{AgentKind, AgentState, Provider},
        project::{ConversationMessage, Project},
    },
};

pub enum AgentMessage {
    Chunk {
        project_name: String,
        tab_kind:     AgentKind,
        text:         String,
        is_newline:   bool,
    },
    StateChange {
        project_name: String,
        tab_kind:     AgentKind,
        new_state:    AgentState,
    },
}

pub struct AgentManager {
    tx:         mpsc::UnboundedSender<AgentMessage>,
    rx:         mpsc::UnboundedReceiver<AgentMessage>,
    cancellers: HashMap<(String, String), oneshot::Sender<()>>,
}

impl AgentManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { tx, rx, cancellers: HashMap::new() }
    }

    pub fn spawn_ai_prompt(
        &mut self,
        project_name: &str,
        provider:     &Provider,
        project_path: &str,
        messages:     Vec<ConversationMessage>,
        config:       &Config,
    ) {
        let key = (project_name.to_string(), "AI Prompt".to_string());

        // Cancel any existing run on this slot
        if let Some(cancel) = self.cancellers.remove(&key) {
            let _ = cancel.send(());
        }

        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
        self.cancellers.insert(key, cancel_tx);

        let tx    = self.tx.clone();
        let pname = project_name.to_string();
        let cwd   = project_path.to_string();

        match provider {
            Provider::Anthropic => {
                let mode = config.provider.anthropic.as_ref()
                    .map(|c| c.mode.as_str())
                    .unwrap_or("subprocess")
                    .to_string();

                if mode == "api" {
                    let api_key = {
                        let key = config.provider.anthropic.as_ref()
                            .map(|c| c.api_key.clone())
                            .unwrap_or_default();
                        if key.is_empty() {
                            std::env::var("ANTHROPIC_API_KEY").unwrap_or_default()
                        } else {
                            key
                        }
                    };
                    let model = config.provider.anthropic.as_ref()
                        .map(|c| c.model.clone())
                        .unwrap_or_else(|| "claude-opus-4-5".to_string());
                    tokio::spawn(api_agent::run(
                        pname, AgentKind::AiPrompt, messages, api_key, model, tx, cancel_rx,
                    ));
                } else {
                    tokio::spawn(subprocess_agent::run(
                        "claude".to_string(),
                        vec!["-p".to_string()],
                        pname, AgentKind::AiPrompt, cwd, Some(messages), tx, cancel_rx,
                    ));
                }
            }

            Provider::OpenCode => {
                let mode = config.provider.opencode.as_ref()
                    .map(|c| c.mode.as_str())
                    .unwrap_or("subprocess")
                    .to_string();

                if mode == "api" {
                    let api_base = config.provider.opencode.as_ref()
                        .map(|c| c.api_base.clone())
                        .unwrap_or_else(|| "http://localhost:4096/v1".to_string());
                    let api_key = config.provider.opencode.as_ref()
                        .map(|c| c.api_key.clone())
                        .unwrap_or_default();
                    let model = config.provider.opencode.as_ref()
                        .map(|c| c.model.clone())
                        .unwrap_or_else(|| "anthropic/claude-sonnet-4-5".to_string());
                    tokio::spawn(openai_api_agent::run(
                        pname, AgentKind::AiPrompt, messages,
                        api_base, api_key, model, tx, cancel_rx,
                    ));
                } else {
                    tokio::spawn(subprocess_agent::run(
                        "opencode".to_string(),
                        vec!["run".to_string(), "--print".to_string()],
                        pname, AgentKind::AiPrompt, cwd, Some(messages), tx, cancel_rx,
                    ));
                }
            }
        }
    }

    pub fn spawn_git_command(
        &mut self,
        project_name: &str,
        project_path: &str,
        git_args:     Vec<String>,
    ) {
        let key = (project_name.to_string(), "Git".to_string());
        if let Some(cancel) = self.cancellers.remove(&key) {
            let _ = cancel.send(());
        }
        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
        self.cancellers.insert(key, cancel_tx);

        let tx    = self.tx.clone();
        let pname = project_name.to_string();
        let cwd   = project_path.to_string();
        tokio::spawn(subprocess_agent::run(
            "git".to_string(),
            git_args,
            pname, AgentKind::Git, cwd, None, tx, cancel_rx,
        ));
    }

    pub fn cancel_ai_prompt(&mut self, project_name: &str) {
        let key = (project_name.to_string(), "AI Prompt".to_string());
        if let Some(cancel) = self.cancellers.remove(&key) {
            let _ = cancel.send(());
        }
    }

    pub fn drain_into(&mut self, projects: &mut Vec<Project>) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                AgentMessage::Chunk { project_name, tab_kind, text, is_newline } => {
                    if let Some(p) = projects.iter_mut().find(|p| p.name == project_name) {
                        if let Some(tab) = p.tabs.iter_mut().find(|t| t.kind == tab_kind) {
                            if is_newline {
                                tab.content.push_line(text.clone());
                            } else {
                                tab.content.push_partial(&text);
                            }
                            // Accumulate for conversation history
                            if tab.kind == AgentKind::AiPrompt {
                                p.pending_response.push_str(&text);
                                if is_newline { p.pending_response.push('\n'); }
                            }
                        }
                    }
                }
                AgentMessage::StateChange { project_name, tab_kind, new_state } => {
                    if let Some(p) = projects.iter_mut().find(|p| p.name == project_name) {
                        let is_ai_prompt = tab_kind == AgentKind::AiPrompt;
                        let is_git       = tab_kind == AgentKind::Git;
                        let project_path = p.path.clone();

                        if let Some(tab) = p.tabs.iter_mut().find(|t| t.kind == tab_kind) {
                            match &new_state {
                                AgentState::Running => {
                                    tab.content.scroll_offset = usize::MAX;
                                    if is_ai_prompt { p.pending_response.clear(); }
                                }
                                AgentState::Idle | AgentState::Error => {
                                    if is_ai_prompt && !p.pending_response.is_empty() {
                                        let content = std::mem::take(&mut p.pending_response);
                                        p.conversation.push(ConversationMessage {
                                            role:    "assistant".to_string(),
                                            content: content.trim_end_matches('\n').to_string(),
                                        });
                                    }
                                    if is_git {
                                        p.git_status.refresh(&project_path);
                                    }
                                }
                                _ => {}
                            }
                            tab.state = new_state;
                        }
                    }
                }
            }
        }
    }
}
