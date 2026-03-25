use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs, Wrap},
    Frame,
};
use crate::{
    app::App,
    state::agent::AgentKind,
    ui::{file_tree::draw_file_tree, git_panel::draw_git_panel, led::led_span, tab_content::TabContentWidget},
};

pub fn draw_main(frame: &mut Frame, area: Rect, app: &App) {
    let Some(idx)     = app.active_project_index() else { return };
    let Some(project) = app.projects.get(idx)       else { return };

    // ── Top: tabs bar (3 rows) ──────────────────────────────────────────────
    let top_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let titles: Vec<Line> = project.tabs.iter().map(|tab| {
        Line::from(vec![
            led_span(&tab.state),
            Span::raw(" "),
            Span::raw(tab.label.as_str()),
        ])
    }).collect();

    frame.render_widget(
        Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title(project.name.as_str()))
            .select(project.active_tab)
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        top_split[0],
    );

    let content_area = top_split[1];

    // ── Horizontal split: left panel 20% | right panel 80% ─────────────────
    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(content_area);

    // ── Left panel: top 50% = file tree, bottom 50% = reserved ─────────────
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(horiz[0]);

    draw_file_tree(frame, left[0], &project.file_tree, app.file_tree_focused);
    draw_git_panel(frame, left[1], &project.git_status, app.git_focused);

    // ── Right panel: tab content ────────────────────────────────────────────
    let Some(tab) = project.tabs.get(project.active_tab) else { return };

    if tab.kind == AgentKind::AiPrompt {
        let right_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(horiz[1]);

        frame.render_widget(TabContentWidget { tab }, right_split[0]);
        draw_prompt_input(frame, right_split[1], project, app.prompt_focused);
    } else {
        frame.render_widget(TabContentWidget { tab }, horiz[1]);
    }
}

fn draw_prompt_input(
    frame:   &mut Frame,
    area:    Rect,
    project: &crate::state::project::Project,
    focused: bool,
) {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Rgb(80, 80, 120))
    };
    let title = if focused {
        " Prompt  Enter send · Esc blur · Ctrl-K cancel "
    } else {
        " Prompt  Enter to type "
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(Color::Black));

    let input  = &project.prompt_input;
    let cursor = project.prompt_cursor;

    let line = if focused {
        let (before, cur_ch, after) = split_at_cursor(input, cursor);
        Line::from(vec![
            Span::raw(before),
            Span::styled(cur_ch, Style::default().add_modifier(Modifier::REVERSED)),
            Span::raw(after),
        ])
    } else {
        Line::from(Span::styled(input.as_str(), Style::default().fg(Color::Rgb(180, 180, 180))))
    };

    frame.render_widget(
        Paragraph::new(line)
            .block(block)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: false }),
        area,
    );

    if focused {
        let col     = char_display_width(input, cursor) as u16;
        let max_col = area.width.saturating_sub(2);
        frame.set_cursor(area.x + 1 + col.min(max_col), area.y + 1);
    }
}

fn split_at_cursor(s: &str, cursor: usize) -> (String, String, String) {
    let mut chars = s.chars();
    let before: String = chars.by_ref().take(cursor).collect();
    let cur_ch: String = chars.next().map(|c| c.to_string()).unwrap_or_else(|| " ".to_string());
    let after:  String = chars.collect();
    (before, cur_ch, after)
}

fn char_display_width(s: &str, cursor: usize) -> usize {
    s.chars().take(cursor).count()
}
