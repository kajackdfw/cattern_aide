use ratatui::{style::{Color, Style}, text::Span};
use crate::state::agent::AgentState;

pub fn led_span(state: &AgentState) -> Span<'static> {
    let color = match state {
        AgentState::Idle    => Color::DarkGray,
        AgentState::Running => Color::Blue,
        AgentState::Waiting => Color::Yellow,
        AgentState::Error   => Color::Red,
    };
    Span::styled("●", Style::default().fg(color))
}
