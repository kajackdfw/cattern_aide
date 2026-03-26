use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

pub struct AppLayout {
    pub sidebar: Rect,
    pub main:    Rect,
}

pub struct PanelRects {
    pub sidebar:     Rect,
    pub file_tree:   Rect,
    pub git_panel:   Rect,
    pub middle:      Rect,
    pub target_code: Rect,
}

pub fn compute(frame: &Frame) -> AppLayout {
    compute_from_rect(frame.size())
}

/// Recompute panel rects from a terminal size without needing a Frame.
/// Used by mouse hit-testing in app.rs.
pub fn panel_rects(terminal_size: Rect) -> PanelRects {
    let app = compute_from_rect(terminal_size);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(40),
            Constraint::Percentage(40),
        ])
        .split(app.main);

    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(cols[0]);

    PanelRects {
        sidebar:     app.sidebar,
        file_tree:   left[0],
        git_panel:   left[1],
        middle:      cols[1],
        target_code: cols[2],
    }
}

fn compute_from_rect(r: Rect) -> AppLayout {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(6), Constraint::Min(0)])
        .split(r);
    AppLayout { sidebar: chunks[0], main: chunks[1] }
}
