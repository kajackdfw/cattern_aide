use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use crate::{
    app::{AppModal, FolderBrowserState, HelpPage, ModalField, NewProjectForm},
    config::ProjectConfig,
    state::agent::Provider,
};

pub fn draw_modal(frame: &mut Frame, modal: &AppModal, recently_closed: &[ProjectConfig]) {
    match modal {
        AppModal::NewProject(form)                        => draw_project_form(frame, form, " New Project "),
        AppModal::EditProject { form, .. }                 => draw_project_form(frame, form, " Edit Project "),
        AppModal::DeleteConfirm { name, yes_focused, .. } => draw_delete_confirm(frame, name, *yes_focused),
        AppModal::GitCommit { message, cursor, .. }        => draw_git_commit(frame, message, *cursor),
        AppModal::Help(page)                              => draw_help(frame, page),
        AppModal::RecentProjects { selected }             => draw_recent_projects(frame, recently_closed, *selected),
    }
}

fn draw_help(frame: &mut Frame, page: &HelpPage) {
    match page {
        HelpPage::Main    => draw_help_main(frame),
        HelpPage::Files   => draw_help_files(frame),
        HelpPage::Git     => draw_help_git(frame),
        HelpPage::Code    => draw_help_code(frame),
        HelpPage::Process => draw_help_process(frame),
    }
}


fn render_help_modal(frame: &mut Frame, title: &str, w: u16, lines: Vec<Line>) {
    let h = (lines.len() + 2) as u16;
    let screen = frame.size();
    let modal_rect = centered_rect(w, h, screen);
    frame.render_widget(Clear, modal_rect);
    let block = Block::default()
        .title(format!(" {} ", title))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(Color::Rgb(12, 12, 30)).fg(Color::White));
    frame.render_widget(block, modal_rect);
    let inner = Rect::new(
        modal_rect.x + 1,
        modal_rect.y + 1,
        modal_rect.width.saturating_sub(2),
        modal_rect.height.saturating_sub(2),
    );
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_help_main(frame: &mut Frame) {
    let k  = |s: &'static str| Span::styled(s, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    let d  = |s: &'static str| Span::styled(s, Style::default().fg(Color::Rgb(200, 200, 220)));
    let sp = || Span::raw("  ");
    let h  = |s: &'static str| Line::from(Span::styled(s, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let b  = || Line::from("");
    let dim = |s: &'static str| Span::styled(s, Style::default().fg(Color::Rgb(100, 100, 130)));
    let cat = |s: &'static str| Span::styled(s, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));

    let lines: Vec<Line> = vec![
        h("Navigation"),
        Line::from(vec![k("j / k  ↑↓"), sp(), d("Switch project")]),
        Line::from(vec![k("l / → / Tab"), sp(), d("Next tab")]),
        Line::from(vec![k("h / ←"),      sp(), d("Previous tab")]),
        Line::from(vec![k("PgDn / PgUp"),sp(), d("Scroll current tab")]),
        b(),
        h("Projects"),
        Line::from(vec![k("Enter"),  sp(), d("Focus AI prompt / start process")]),
        Line::from(vec![k("e"),      sp(), d("Edit selected project")]),
        Line::from(vec![k("d / Del"),sp(), d("Delete selected project")]),
        b(),
        h("General"),
        Line::from(vec![k("Ctrl-K"), sp(), d("Kill running agent on current tab")]),
        Line::from(vec![k("?"),      sp(), d("Help")]),
        Line::from(vec![k("q / Ctrl-C"), sp(), d("Quit")]),
        b(),
        Line::from(vec![
            Span::styled("─── More help ", Style::default().fg(Color::Rgb(60, 60, 90))),
            cat("f"), dim(" Files  "),
            cat("g"), dim(" Git  "),
            cat("c"), dim(" Code  "),
            cat("p"), dim(" Process"),
        ]),
        b(),
        Line::from(dim("Press a category key or any other key to close")),
    ];

    render_help_modal(frame, "Help", 46, lines);
}

fn draw_help_files(frame: &mut Frame) {
    let k  = |s: &'static str| Span::styled(s, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    let d  = |s: &'static str| Span::styled(s, Style::default().fg(Color::Rgb(200, 200, 220)));
    let sp = || Span::raw("  ");
    let h  = |s: &'static str| Line::from(Span::styled(s, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let b  = || Line::from("");
    let dim = |s: &'static str| Span::styled(s, Style::default().fg(Color::Rgb(100, 100, 130)));

    let lines: Vec<Line> = vec![
        h("File Tree  (press f to focus)"),
        b(),
        Line::from(vec![k("j / k"),    sp(), d("Navigate up / down")]),
        Line::from(vec![k("Enter"),    sp(), d("Open file in Target Code panel")]),
        Line::from(vec![k("l / Space"),sp(), d("Expand / collapse directory")]),
        Line::from(vec![k("h"),        sp(), d("Collapse dir or jump to parent")]),
        Line::from(vec![k("g / G"),    sp(), d("Jump to top / bottom")]),
        Line::from(vec![k("r"),        sp(), d("Reload tree from disk")]),
        Line::from(vec![k("Esc"),      sp(), d("Blur panel")]),
        b(),
        Line::from(dim("b / Backspace / Esc → back to main help")),
    ];

    render_help_modal(frame, "Help — Files", 44, lines);
}

fn draw_help_git(frame: &mut Frame) {
    let k  = |s: &'static str| Span::styled(s, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    let d  = |s: &'static str| Span::styled(s, Style::default().fg(Color::Rgb(200, 200, 220)));
    let sp = || Span::raw("  ");
    let h  = |s: &'static str| Line::from(Span::styled(s, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let b  = || Line::from("");
    let dim = |s: &'static str| Span::styled(s, Style::default().fg(Color::Rgb(100, 100, 130)));

    let lines: Vec<Line> = vec![
        h("Git Panel  (press g to focus)"),
        b(),
        Line::from(vec![k("j / k"),  sp(), d("Navigate changed files")]),
        Line::from(vec![k("Enter"),  sp(), d("Diff selected file (→ Git tab)")]),
        Line::from(vec![k("s"),      sp(), d("Stage selected file")]),
        Line::from(vec![k("u"),      sp(), d("Unstage selected file")]),
        Line::from(vec![k("x"),      sp(), d("Discard changes to selected file")]),
        Line::from(vec![k("c"),      sp(), d("Commit  (opens message modal)")]),
        Line::from(vec![k("p"),      sp(), d("Pull")]),
        Line::from(vec![k("P"),      sp(), d("Push")]),
        Line::from(vec![k("f"),      sp(), d("Fetch")]),
        Line::from(vec![k("r"),      sp(), d("Reload git status")]),
        Line::from(vec![k("Esc"),    sp(), d("Blur panel")]),
        b(),
        Line::from(dim("b / Backspace / Esc → back to main help")),
    ];

    render_help_modal(frame, "Help — Git", 44, lines);
}

fn draw_help_code(frame: &mut Frame) {
    let k  = |s: &'static str| Span::styled(s, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    let d  = |s: &'static str| Span::styled(s, Style::default().fg(Color::Rgb(200, 200, 220)));
    let sp = || Span::raw("  ");
    let h  = |s: &'static str| Line::from(Span::styled(s, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let b  = || Line::from("");
    let dim = |s: &'static str| Span::styled(s, Style::default().fg(Color::Rgb(100, 100, 130)));

    let lines: Vec<Line> = vec![
        h("AI Prompt  (press Enter to focus)"),
        b(),
        Line::from(vec![k("Enter"),  sp(), d("Send prompt to AI")]),
        Line::from(vec![k("Ctrl-K"), sp(), d("Cancel in-progress generation")]),
        Line::from(vec![k("Esc"),    sp(), d("Blur prompt input")]),
        b(),
        h("Target Code  (press t to focus)"),
        b(),
        Line::from(vec![k("PgDn / PgUp"), sp(), d("Scroll content")]),
        Line::from(vec![k("g / G"),       sp(), d("Jump to top / bottom")]),
        Line::from(vec![k("Esc"),         sp(), d("Blur panel")]),
        b(),
        Line::from(dim("Open a file from the File Tree to populate this panel.")),
        b(),
        Line::from(dim("b / Backspace / Esc → back to main help")),
    ];

    render_help_modal(frame, "Help — Code", 48, lines);
}

fn draw_help_process(frame: &mut Frame) {
    let k  = |s: &'static str| Span::styled(s, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    let d  = |s: &'static str| Span::styled(s, Style::default().fg(Color::Rgb(200, 200, 220)));
    let sp = || Span::raw("  ");
    let h  = |s: &'static str| Line::from(Span::styled(s, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let b  = || Line::from("");
    let dim = |s: &'static str| Span::styled(s, Style::default().fg(Color::Rgb(100, 100, 130)));
    let code = |s: &'static str| Span::styled(s, Style::default().fg(Color::Rgb(160, 200, 130)));

    let lines: Vec<Line> = vec![
        h("Process Tabs"),
        b(),
        Line::from(vec![k("Enter"),  sp(), d("Start the process (when idle / errored)")]),
        Line::from(vec![k("Ctrl-K"), sp(), d("Stop running process")]),
        b(),
        h("HTTP Proxy Tabs"),
        b(),
        Line::from(vec![k("Ctrl-K"), sp(), d("Stop proxy listener")]),
        Line::from(d("Proxy auto-starts on launch and logs all")),
        Line::from(d("requests/responses to this tab.")),
        b(),
        h("config.toml examples"),
        b(),
        Line::from(code("tabs = [")),
        Line::from(code("  {kind=\"process\", name=\"Dev\",")),
        Line::from(code("   command=\"npm run dev\"},")),
        Line::from(code("  {kind=\"http_proxy\", name=\"API\",")),
        Line::from(code("   port=8080, target=\"http://localhost:3000\"},")),
        Line::from(code("]")),
        b(),
        Line::from(dim("b / Backspace / Esc → back to main help")),
    ];

    render_help_modal(frame, "Help — Process & Proxy", 52, lines);
}

fn draw_project_form(frame: &mut Frame, form: &NewProjectForm, title: &str) {
    let screen = frame.size();
    let modal_rect = centered_rect(62, 16, screen);

    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(title)
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

    // Vertical layout: name(3) + path(3) + provider(2) + theme(2) + confirm(2) + hint(1) + padding
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Name
            Constraint::Length(3),  // Path
            Constraint::Length(2),  // Provider toggle
            Constraint::Length(2),  // Theme toggle
            Constraint::Length(2),  // Confirm button
            Constraint::Length(1),  // Hint
            Constraint::Min(0),     // padding
        ])
        .split(inner);

    render_text_field(frame, rows[0], "Name", &form.name,
        form.focused_field == ModalField::Name, form.name_cursor);
    render_text_field(frame, rows[1], "Path  (Ctrl-F to browse)", &form.path,
        form.focused_field == ModalField::Path, form.path_cursor);
    render_provider_toggle(frame, rows[2], &form.provider,
        form.focused_field == ModalField::Provider);
    render_theme_toggle(frame, rows[3], &form.theme,
        form.focused_field == ModalField::Theme);
    render_confirm_button(frame, rows[4], form.focused_field == ModalField::Confirm);
    render_hint(frame, rows[5]);

    // Folder browser overlay
    if let Some(ref browser) = form.browser {
        draw_folder_browser(frame, browser);
    }

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

fn render_theme_toggle(frame: &mut Frame, area: Rect, theme: &str, focused: bool) {
    let label_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Rgb(140, 140, 180))
    };

    let themes = crate::ui::theme::NAMES;
    let mut spans = vec![Span::styled("Theme:    ", label_style)];
    for name in themes {
        let active = *name == theme;
        let style = if active {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(80, 80, 100))
        };
        spans.push(Span::styled(format!("[ {} ]", name), style));
        spans.push(Span::raw("  "));
    }
    spans.push(Span::styled("(←/→)", Style::default().fg(Color::Rgb(80, 80, 100))));

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
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
        Paragraph::new("Tab/↑↓ navigate   ←/→ toggle   Ctrl-F browse path   Enter confirm   Esc cancel")
            .style(Style::default().fg(Color::Rgb(80, 80, 100)))
            .alignment(Alignment::Center),
        area,
    );
}

fn draw_folder_browser(frame: &mut Frame, browser: &FolderBrowserState) {
    let screen = frame.size();
    let modal_rect = centered_rect(70, 22, screen);

    frame.render_widget(Clear, modal_rect);

    let current_path = browser.current_dir.to_string_lossy();
    let title = format!("  {}  ", current_path);
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(Color::Rgb(10, 10, 30)).fg(Color::Cyan));
    frame.render_widget(block, modal_rect);

    let inner = Rect::new(
        modal_rect.x + 1,
        modal_rect.y + 1,
        modal_rect.width.saturating_sub(2),
        modal_rect.height.saturating_sub(2),
    );

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let list_height = rows[0].height as usize;
    let n = browser.entries.len();

    if n == 0 {
        frame.render_widget(
            Paragraph::new("  (no subdirectories — press Space to select this directory)")
                .style(Style::default().fg(Color::Rgb(100, 100, 120))),
            rows[0],
        );
    } else {
        let start = if browser.selected >= list_height {
            browser.selected - list_height + 1
        } else {
            0
        };
        let end = (start + list_height).min(n);

        let lines: Vec<Line> = browser.entries[start..end]
            .iter()
            .enumerate()
            .map(|(i, path)| {
                let abs_idx = start + i;
                let name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());
                let display = format!("  {}  ", name);
                if abs_idx == browser.selected {
                    Line::from(Span::styled(
                        display,
                        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ))
                } else {
                    Line::from(Span::styled(display, Style::default().fg(Color::White)))
                }
            })
            .collect();

        frame.render_widget(Paragraph::new(lines), rows[0]);
    }

    frame.render_widget(
        Paragraph::new("↑↓ navigate   Enter enter dir   ← go up   Space select this dir   Esc cancel")
            .style(Style::default().fg(Color::Rgb(80, 80, 100)))
            .alignment(Alignment::Center),
        rows[1],
    );
}

fn draw_git_commit(frame: &mut Frame, message: &str, cursor: usize) {
    let screen = frame.size();
    let modal_rect = centered_rect(62, 7, screen);

    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Git Commit ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(Color::Rgb(10, 20, 15)).fg(Color::White));
    frame.render_widget(block, modal_rect);

    let inner = Rect::new(
        modal_rect.x + 1,
        modal_rect.y + 1,
        modal_rect.width.saturating_sub(2),
        modal_rect.height.saturating_sub(2),
    );

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // input field
            Constraint::Length(1), // hint
            Constraint::Min(0),
        ])
        .split(inner);

    render_text_field(frame, rows[0], "Message", message, true, cursor);

    frame.render_widget(
        Paragraph::new("Enter commit · Esc cancel")
            .style(Style::default().fg(Color::Rgb(80, 80, 100)))
            .alignment(Alignment::Center),
        rows[1],
    );

    // Position terminal cursor
    let col     = char_display_width(message, cursor) as u16;
    let max_col = rows[0].width.saturating_sub(2);
    frame.set_cursor(rows[0].x + 1 + col.min(max_col), rows[0].y + 1);
}

fn draw_delete_confirm(frame: &mut Frame, name: &str, yes_focused: bool) {
    let screen = frame.size();
    let modal_rect = centered_rect(52, 9, screen);

    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Delete Project ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(Color::Rgb(35, 10, 10)).fg(Color::White));
    frame.render_widget(block, modal_rect);

    let inner = Rect::new(
        modal_rect.x + 1,
        modal_rect.y + 1,
        modal_rect.width.saturating_sub(2),
        modal_rect.height.saturating_sub(2),
    );

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // padding
            Constraint::Length(1), // message line 1
            Constraint::Length(1), // message line 2
            Constraint::Length(1), // padding
            Constraint::Length(2), // buttons
            Constraint::Min(0),
        ])
        .split(inner);

    // Truncate long names
    let display_name: String = name.chars().take(36).collect();
    let ellipsis = if name.chars().count() > 36 { "…" } else { "" };

    frame.render_widget(
        Paragraph::new(format!("Delete \"{}{}\"?", display_name, ellipsis))
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center),
        rows[1],
    );
    frame.render_widget(
        Paragraph::new("This cannot be undone.")
            .style(Style::default().fg(Color::Rgb(160, 100, 100)))
            .alignment(Alignment::Center),
        rows[2],
    );

    // Buttons
    let btn_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(10),
            Constraint::Length(4),
            Constraint::Length(10),
            Constraint::Min(0),
        ])
        .split(rows[4]);

    let yes_style = if yes_focused {
        Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(160, 80, 80)).bg(Color::Rgb(50, 20, 20))
    };
    let no_style = if !yes_focused {
        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(100, 140, 160)).bg(Color::Rgb(20, 35, 45))
    };

    frame.render_widget(
        Paragraph::new("  Yes  ").style(yes_style).alignment(Alignment::Center),
        btn_chunks[1],
    );
    frame.render_widget(
        Paragraph::new("  No  ").style(no_style).alignment(Alignment::Center),
        btn_chunks[3],
    );
}

fn draw_recent_projects(frame: &mut Frame, items: &[ProjectConfig], selected: usize) {
    if items.is_empty() { return; }

    let list_h  = (items.len().min(12) + 2) as u16; // entries + header + hint
    let height  = list_h + 4;  // border top/bottom + padding
    let screen  = frame.size();
    let modal_rect = centered_rect(66, height, screen);

    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .title(" Recent Projects ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(Color::Rgb(12, 12, 30)).fg(Color::White));
    frame.render_widget(block, modal_rect);

    let inner = Rect::new(
        modal_rect.x + 1,
        modal_rect.y + 1,
        modal_rect.width.saturating_sub(2),
        modal_rect.height.saturating_sub(2),
    );

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    // Scrolling: keep selected in view
    let visible = rows[0].height as usize;
    let offset = if selected >= visible { selected - visible + 1 } else { 0 };

    let lines: Vec<Line> = items.iter()
        .enumerate()
        .skip(offset)
        .take(visible)
        .map(|(i, pc)| {
            let path_preview: String = pc.path.chars().take(36).collect();
            let ellipsis = if pc.path.chars().count() > 36 { "…" } else { "" };
            let name_col: String = pc.name.chars().take(22).collect();
            let text = format!("  {:<22}  {}{}", name_col, path_preview, ellipsis);
            if i == selected {
                Line::from(Span::styled(
                    text,
                    Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(text, Style::default().fg(Color::Rgb(200, 200, 220))))
            }
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), rows[0]);

    frame.render_widget(
        Paragraph::new("↑↓ navigate   Enter reopen   d/Del remove from list   Esc close")
            .style(Style::default().fg(Color::Rgb(80, 80, 110)))
            .alignment(Alignment::Center),
        rows[1],
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
