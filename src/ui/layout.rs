use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

pub struct AppLayout {
    pub sidebar: Rect,
    pub main:    Rect,
}

pub fn compute(frame: &Frame) -> AppLayout {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(6), Constraint::Min(0)])
        .split(frame.size());
    AppLayout { sidebar: chunks[0], main: chunks[1] }
}
