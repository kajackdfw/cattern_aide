use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use crate::{
    app::{AppModal, ModalField, NewProjectForm},
    state::agent::Provider,
};

pub fn draw_modal(frame: &mut Frame, modal: &AppModal) {
    match modal {
        AppModal::NewProject(form) => draw_new_project(frame, form),
        AppModal::Help             => draw_help(frame),
    }
}

fn draw_help(frame: &mut Frame) {
    let screen = frame.size();
    let modal_rect = centered_rect(52, 20, screen);

    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Help ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(Color::Rgb(15, 15, 35)).fg(Color::White));
    frame.render_widget(block, modal_rect);

    let inner = Rect::new(
        modal_rect.x + 1,
        modal_rect.y + 1,
        modal_rect.width.saturating_sub(2),
        modal_rect.height.saturating_sub(2),
    );

    let key = |k: &'static str| Span::styled(k, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    let sep = || Span::styled("  ", Style::default());
    let desc = |d: &'static str| Span::styled(d, Style::default().fg(Color::Rgb(200, 200, 220)));
    let head = |h: &'static str| Line::from(Span::styled(h, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let blank = || Line::from("");

    let lines: Vec<Line> = vec![
        head("Navigation"),
        Line::from(vec![key("j / ↓"),          sep(), desc("Next project")]),
        Line::from(vec![key("k / ↑"),          sep(), desc("Previous project")]),
        Line::from(vec![key("l / → / Tab"),    sep(), desc("Next tab")]),
        Line::from(vec![key("h / ←"),          sep(), desc("Previous tab")]),
        Line::from(vec![key("PgDn / PgUp"),    sep(), desc("Scroll content")]),
        blank(),
        head("Add Project"),
        Line::from(vec![key("j / ↓"),          sep(), desc("Navigate down to  ─── +")]),
        Line::from(vec![key("Enter / Space"),  sep(), desc("Open add-project form")]),
        Line::from(vec![key("Tab / ↑↓"),       sep(), desc("Move between form fields")]),
        Line::from(vec![key("← / →"),          sep(), desc("Toggle ClaudeCode / OpenCode")]),
        Line::from(vec![key("Esc"),            sep(), desc("Cancel form")]),
        blank(),
        head("General"),
        Line::from(vec![key("?"),              sep(), desc("Toggle this help")]),
        Line::from(vec![key("q / Ctrl-C"),     sep(), desc("Quit")]),
        blank(),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(Color::Rgb(80, 80, 100)).add_modifier(Modifier::ITALIC),
        )),
    ];

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(Color::White)),
        inner,
    );
}

fn draw_new_project(frame: &mut Frame, form: &NewProjectForm) {
    let screen = frame.size();
    let modal_rect = centered_rect(62, 14, screen);

    // Clear whatever is underneath
    frame.render_widget(Clear, modal_rect);

    // Outer block
    let block = Block::default()
        .title(" New Project ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(Color::Rgb(15, 15, 35)).fg(Color::White));
    frame.render_widget(block, modal_rect);

    // Inner area (inside border)
    let inner = Rect::new(
        modal_rect.x + 1,
        modal_rect.y + 1,
        modal_rect.width.saturating_sub(2),
        modal_rect.height.saturating_sub(2),
    );

    // Vertical layout: name(3) + path(3) + provider(2) + confirm(2) + hint(1) + padding
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Name
            Constraint::Length(3),  // Path
            Constraint::Length(2),  // Provider toggle
            Constraint::Length(2),  // Confirm button
            Constraint::Length(1),  // Hint
            Constraint::Min(0),     // padding
        ])
        .split(inner);

    render_text_field(frame, rows[0], "Name", &form.name,
        form.focused_field == ModalField::Name, form.name_cursor);
    render_text_field(frame, rows[1], "Path", &form.path,
        form.focused_field == ModalField::Path, form.path_cursor);
    render_provider_toggle(frame, rows[2], &form.provider,
        form.focused_field == ModalField::Provider);
    render_confirm_button(frame, rows[3], form.focused_field == ModalField::Confirm);
    render_hint(frame, rows[4]);

    // Position terminal cursor inside focused text fields
    if form.focused_field == ModalField::Name {
        let inner_x = rows[0].x + 1;
        let inner_y = rows[0].y + 1;
        let col     = char_display_width(&form.name, form.name_cursor) as u16;
        let max_col = rows[0].width.saturating_sub(2);
        frame.set_cursor(inner_x + col.min(max_col), inner_y);
    } else if form.focused_field == ModalField::Path {
        let inner_x = rows[1].x + 1;
        let inner_y = rows[1].y + 1;
        let col     = char_display_width(&form.path, form.path_cursor) as u16;
        let max_col = rows[1].width.saturating_sub(2);
        frame.set_cursor(inner_x + col.min(max_col), inner_y);
    }
}

fn render_text_field(
    frame: &mut Frame,
    area:  Rect,
    label: &str,
    value: &str,
    focused: bool,
    cursor:  usize,
) {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Rgb(80, 80, 120))
    };
    let block = Block::default()
        .title(format!(" {label} "))
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(Color::Rgb(15, 15, 35)));

    // Build spans to show block cursor at the cursor position
    let line = if focused {
        let (before, cur_ch, after) = split_at_cursor(value, cursor);
        Line::from(vec![
            Span::raw(before),
            Span::styled(cur_ch, Style::default().add_modifier(Modifier::REVERSED)),
            Span::raw(after),
        ])
    } else {
        Line::from(Span::raw(value))
    };

    frame.render_widget(
        Paragraph::new(line)
            .block(block)
            .style(Style::default().fg(Color::White)),
        area,
    );
}

fn render_provider_toggle(frame: &mut Frame, area: Rect, provider: &Provider, focused: bool) {
    let (claude_style, opencode_style) = match provider {
        Provider::Anthropic => (
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            Style::default().fg(Color::Rgb(80, 80, 100)),
        ),
        Provider::OpenCode => (
            Style::default().fg(Color::Rgb(80, 80, 100)),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
    };
    let label_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Rgb(140, 140, 180))
    };

    let line = Line::from(vec![
        Span::styled("Provider: ", label_style),
        Span::styled("[ ClaudeCode ]", claude_style),
        Span::raw("  "),
        Span::styled("[ OpenCode ]", opencode_style),
        Span::raw("  "),
        Span::styled("(←/→)", Style::default().fg(Color::Rgb(80, 80, 100))),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

fn render_confirm_button(frame: &mut Frame, area: Rect, focused: bool) {
    let style = if focused {
        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(140, 140, 200)).bg(Color::Rgb(30, 30, 60))
    };
    frame.render_widget(
        Paragraph::new("  Add Project  ")
            .style(style)
            .alignment(Alignment::Center),
        area,
    );
}

fn render_hint(frame: &mut Frame, area: Rect) {
    frame.render_widget(
        Paragraph::new("Tab/↑↓ navigate   ←/→ toggle   Enter confirm   Esc cancel")
            .style(Style::default().fg(Color::Rgb(80, 80, 100)))
            .alignment(Alignment::Center),
        area,
    );
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Split `s` at `cursor` (char index) into (before, cursor_char_or_space, after).
fn split_at_cursor(s: &str, cursor: usize) -> (String, String, String) {
    let mut chars = s.chars();
    let before:  String = chars.by_ref().take(cursor).collect();
    let cur_ch:  String = chars.next().map(|c| c.to_string()).unwrap_or_else(|| " ".to_string());
    let after:   String = chars.collect();
    (before, cur_ch, after)
}

/// Count display columns up to `cursor` char positions (ASCII-safe; 1 col/char).
fn char_display_width(s: &str, cursor: usize) -> usize {
    s.chars().take(cursor).count()
}

/// Return a centered Rect with the given width and height inside `area`.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(
        x.max(area.x),
        y.max(area.y),
        width.min(area.width),
        height.min(area.height),
    )
}
