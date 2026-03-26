use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use crate::{state::git::{FileStatusCode, GitStatus}, ui::theme::Theme};

pub fn draw_git_panel(frame: &mut Frame, area: Rect, status: &GitStatus, focused: bool, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(5)])
        .split(area);

    draw_status_list(frame, chunks[0], status, focused, theme);
    draw_action_menu(frame, chunks[1], focused, theme);
}

fn draw_status_list(frame: &mut Frame, area: Rect, status: &GitStatus, focused: bool, theme: &Theme) {
    let border_style = if focused {
        Style::default().fg(theme.border_focused)
    } else {
        Style::default().fg(theme.border)
    };

    let branch_str = if status.is_git_repo && !status.branch.is_empty() {
        format!("  {} ", status.branch)
    } else {
        String::new()
    };
    let title = format!(" Git{}", branch_str);
    let hint  = if focused { " Esc exit " } else { " g to focus " };

    let block = Block::default()
        .title(title)
        .title_bottom(hint)
        .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP)
        .border_style(border_style)
        .style(Style::default().bg(theme.bg_alt));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if !status.is_git_repo {
        frame.render_widget(
            Paragraph::new("  (not a git repo)")
                .style(Style::default().fg(theme.text_muted)),
            inner,
        );
        return;
    }

    if status.files.is_empty() {
        frame.render_widget(
            Paragraph::new("  working tree clean")
                .style(Style::default().fg(Color::Rgb(80, 160, 80))),
            inner,
        );
        return;
    }

    let n       = status.files.len();
    let visible = inner.height as usize;
    let offset  = scroll_offset(status.selected, visible, n);

    let max_path = (area.width as usize).saturating_sub(5);

    let lines: Vec<Line> = status.files.iter()
        .skip(offset)
        .take(visible)
        .enumerate()
        .map(|(i, entry)| {
            let abs = offset + i;
            let (code, fg) = status_display(&entry.status);
            let path: String = entry.path.chars().take(max_path).collect();
            let text = format!(" {} {}", code, path);

            if abs == status.selected && focused {
                Line::from(Span::styled(
                    text,
                    Style::default().fg(theme.sel_fg).bg(theme.sel_bg_focused).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(text, Style::default().fg(fg)))
            }
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), inner);

    if n > visible {
        let mut sb = ScrollbarState::new(n.saturating_sub(visible)).position(offset);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼")),
            area,
            &mut sb,
        );
    }
}

fn draw_action_menu(frame: &mut Frame, area: Rect, focused: bool, theme: &Theme) {
    let border_style = if focused {
        Style::default().fg(theme.border_focused)
    } else {
        Style::default().fg(theme.border)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(theme.bg_alt));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let key = |k: &'static str| Span::styled(k, if focused {
        Style::default().fg(theme.border_focused).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.text_muted)
    });
    let act = |a: &'static str| Span::styled(a, if focused {
        Style::default().fg(theme.text)
    } else {
        Style::default().fg(theme.text_dim)
    });
    let dot = || Span::styled(" · ", Style::default().fg(theme.border));

    let lines = vec![
        Line::from(vec![key("s"), act(" stage"), dot(), key("u"), act(" unstage"), dot(), key("c"), act(" commit")]),
        Line::from(vec![key("p"), act(" pull"),  dot(), key("P"), act(" push"),    dot(), key("f"), act(" fetch")]),
        Line::from(vec![key("x"), act(" discard"), dot(), key("r"), act(" reload"), dot(), key("↵"), act(" diff")]),
    ];

    frame.render_widget(Paragraph::new(lines), inner);
}

fn status_display(status: &FileStatusCode) -> (&'static str, Color) {
    match status {
        FileStatusCode::Staged         => ("M ", Color::Green),
        FileStatusCode::Modified       => (" M", Color::Red),
        FileStatusCode::StagedModified => ("MM", Color::Yellow),
        FileStatusCode::Added          => ("A ", Color::Green),
        FileStatusCode::Deleted        => ("D ", Color::Red),
        FileStatusCode::Renamed        => ("R ", Color::Cyan),
        FileStatusCode::Untracked      => ("??", Color::Rgb(160, 160, 60)),
        FileStatusCode::Other(_)       => ("  ", Color::Rgb(100, 100, 100)),
    }
}

fn scroll_offset(selected: usize, visible: usize, total: usize) -> usize {
    if total <= visible { return 0; }
    selected.saturating_sub(visible / 2).min(total - visible)
}
