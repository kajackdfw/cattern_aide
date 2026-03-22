pub mod api_agent;
pub mod openai_api_agent;
pub mod subprocess_agent;
pub mod http_proxy_agent;

use crate::state::{
    agent::{AgentKind, AgentState},
    project::Project,
};

#[allow(dead_code)]
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
    // Phase 2: mpsc channels, task handles, cancellation senders
}

impl AgentManager {
    pub fn new_stub() -> Self {
        Self {}
    }

    /// Drain pending AgentMessages into project state. No-op in phase 1.
    pub fn drain_into(&mut self, _projects: &mut Vec<Project>) {}
}
