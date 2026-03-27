use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Tabs, Wrap},
    Frame,
};
use crate::{
    app::App,
    state::{
        agent::{AgentKind, AgentState},
        project::Project,
    },
    ui::{
        file_tree::draw_file_tree,
        git_panel::draw_git_panel,
        led::led_span,
        tab_content::TabContentWidget,
        theme::Theme,
    },
};

pub fn draw_main(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let Some(idx)     = app.active_project_index() else { return };
    let Some(project) = app.projects.get(idx)       else { return };

    // ── Three columns: 20% left | 40% middle | 40% right ───────────────────
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(40),
            Constraint::Percentage(40),
        ])
        .split(area);

    // ── Left column: file tree (top 50%) + git panel (bottom 50%) ───────────
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(cols[0]);

    draw_file_tree(frame, left[0], &project.file_tree, app.file_tree_focused, theme);
    draw_git_panel(frame, left[1], &project.git_status, app.git_focused, theme);

    // ── PTY active: full-screen (entire main area) ──────────────────────────
    let is_pty_active = project.tabs.get(project.active_tab)
        .map(|t| matches!(t.kind, AgentKind::PtyProcess(_)))
        .unwrap_or(false);

    if is_pty_active {
        if let Some(tab) = project.tabs.get(project.active_tab) {
            frame.render_widget(TabContentWidget { tab, theme, pty_focused: app.prompt_focused }, area);
        }
        return;
    }

    // ── Normal layout: middle column tabs bar + active tab content ───────────
    let mid = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(cols[1]);

    let titles: Vec<Line> = project.tabs.iter().map(|tab| {
        Line::from(vec![
            led_span(&tab.state),
            Span::raw(" "),
            Span::raw(tab.label.as_str()),
        ])
    }).collect();

    frame.render_widget(
        Tabs::new(titles)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg))
                .title(project.name.as_str()))
            .select(project.active_tab)
            .highlight_style(Style::default().fg(theme.border_focused).add_modifier(Modifier::BOLD)),
        mid[0],
    );

    if let Some(tab) = project.tabs.get(project.active_tab) {
        let is_ai      = tab.kind == AgentKind::AiPrompt;
        let is_running_process = matches!(tab.kind, AgentKind::Process(_))
            && tab.state == AgentState::Running;

        if is_ai || is_running_process {
            let content_split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(5), Constraint::Min(0)])
                .split(mid[1]);

            draw_prompt_input(frame, content_split[0], project, app.prompt_focused, theme, is_running_process);
            frame.render_widget(TabContentWidget { tab, theme, pty_focused: false }, content_split[1]);
        } else if matches!(tab.kind, AgentKind::Process(_))
            && matches!(tab.state, AgentState::Idle | AgentState::Error)
            && tab.content.lines.is_empty()
        {
            draw_process_hint(frame, mid[1], tab, theme);
        } else {
            frame.render_widget(TabContentWidget { tab, theme, pty_focused: false }, mid[1]);
        }
    }

    // ── Right column: Target Code panel (always visible) ────────────────────
    draw_target_code_panel(frame, cols[2], project, app.target_code_focused, theme);
}

fn draw_target_code_panel(frame: &mut Frame, area: Rect, project: &Project, focused: bool, theme: &Theme) {
    let border_color = if focused { theme.border_focused } else { theme.border };

    // Outer border block (no title — tabs act as the header)
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg));
    let outer_inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    if project.target_tabs.is_empty() {
        let hint = if focused { " Esc to blur " } else { " t to focus " };
        frame.render_widget(
            Paragraph::new(format!(
                "Open a file from the tree\nor press Enter on a git diff\n\n{}",
                hint
            ))
            .style(Style::default().fg(theme.text_muted))
            .alignment(Alignment::Center),
            outer_inner,
        );
        return;
    }

    // Split into tab bar (3 rows) + content
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(outer_inner);

    let tab_bar_area = split[0];
    let content_area = split[1];

    // Tab bar with pagination
    let hint = if focused {
        " ←/→ tabs · PgUp/Dn · g/G · x close · Esc blur "
    } else {
        " t to focus "
    };
    let bar_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg))
        .title(" Files ")
        .title_bottom(hint);
    let bar_inner = bar_block.inner(tab_bar_area);
    frame.render_widget(bar_block, tab_bar_area);
    draw_target_tab_bar(frame, bar_inner, project, theme);

    // Content
    let Some(tab) = project.target_tabs.get(project.active_target_tab) else { return };
    let visible = content_area.height as usize;
    let n       = tab.content.lines.len();
    let offset  = tab.content.clamped_offset(visible);

    let lines: Vec<Line> = tab.content.lines
        .iter()
        .skip(offset)
        .take(visible)
        .map(|l| {
            if tab.is_diff {
                crate::ui::tab_content::diff_line(l, theme)
            } else {
                Line::from(Span::styled(l.as_str(), Style::default().fg(theme.text)))
            }
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(theme.content_bg))
            .wrap(Wrap { trim: false }),
        content_area,
    );

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

/// Render the paginated tab bar inside `area` (already inside the block border).
fn draw_target_tab_bar(frame: &mut Frame, area: Rect, project: &Project, theme: &Theme) {
    let tabs      = &project.target_tabs;
    let active    = project.active_target_tab;
    let n         = tabs.len();
    let avail     = area.width as usize;
    if avail == 0 || n == 0 { return; }

    const LEFT_ARROW:  &str = "<<";
    const RIGHT_ARROW: &str = ">>";
    const SEP:         &str = " │ ";
    let arrow_w = LEFT_ARROW.len() + SEP.len();  // "<<" + " │ "

    // Determine render offset: start from stored offset, but scroll right
    // if the active tab is past the visible window.
    let mut offset = project.target_tab_offset.min(n.saturating_sub(1));

    // Advance offset until active tab is within the visible window.
    loop {
        let left_w  = if offset > 0 { arrow_w } else { 0 };
        let right_w = arrow_w; // reserve space for >> speculatively
        let mut budget = avail.saturating_sub(left_w + right_w);
        let mut last_visible = offset;
        for i in offset..n {
            let tab_w = tabs[i].label.len() + if i == offset { 0 } else { SEP.len() };
            if tab_w <= budget {
                budget -= tab_w;
                last_visible = i;
            } else {
                break;
            }
        }
        if active <= last_visible { break; }
        if offset + 1 >= n { break; }
        offset += 1;
    }

    // Now compute which tabs actually fit from `offset`.
    let has_left  = offset > 0;
    let left_w    = if has_left { arrow_w } else { 0 };
    // Speculatively reserve >> space; we'll know if we need it after fitting tabs.
    let mut budget = avail.saturating_sub(left_w + arrow_w);
    let mut last_visible = offset;
    for i in offset..n {
        let tab_w = tabs[i].label.len() + if i == offset { 0 } else { SEP.len() };
        if tab_w <= budget {
            budget -= tab_w;
            last_visible = i;
        } else {
            break;
        }
    }
    // If all remaining tabs fit, we don't need the >> reservation.
    let has_right = last_visible < n - 1;
    if !has_right {
        // Re-fit without reserving >> space.
        budget = avail.saturating_sub(left_w);
        last_visible = offset;
        for i in offset..n {
            let tab_w = tabs[i].label.len() + if i == offset { 0 } else { SEP.len() };
            if tab_w <= budget {
                budget -= tab_w;
                last_visible = i;
            } else {
                break;
            }
        }
    }

    // Build spans
    let arrow_style       = Style::default().fg(theme.border_focused).add_modifier(Modifier::BOLD);
    let dim_arrow         = Style::default().fg(theme.text_muted);
    let sep_style         = Style::default().fg(theme.border);
    let active_style      = Style::default().fg(theme.border_focused).add_modifier(Modifier::BOLD);
    let active_diff_style = Style::default().fg(ratatui::style::Color::Rgb(230, 80, 80)).add_modifier(Modifier::BOLD);
    let normal_style      = Style::default().fg(theme.text_dim);
    let diff_style        = Style::default().fg(ratatui::style::Color::Rgb(180, 60, 60));

    let mut spans: Vec<Span> = Vec::new();

    if has_left {
        spans.push(Span::styled(LEFT_ARROW, arrow_style));
        spans.push(Span::styled(SEP, sep_style));
    }
    for i in offset..=last_visible {
        if i > offset {
            spans.push(Span::styled(SEP, sep_style));
        }
        let style = match (i == active, tabs[i].is_diff) {
            (true,  true)  => active_diff_style,
            (true,  false) => active_style,
            (false, true)  => diff_style,
            (false, false) => normal_style,
        };
        spans.push(Span::styled(tabs[i].label.as_str(), style));
    }
    if has_right {
        spans.push(Span::styled(SEP, sep_style));
        spans.push(Span::styled(RIGHT_ARROW, if active > last_visible { arrow_style } else { dim_arrow }));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_prompt_input(
    frame:    &mut Frame,
    area:     Rect,
    project:  &Project,
    focused:  bool,
    theme:    &Theme,
    is_stdin: bool,
) {
    let border_style = if focused {
        Style::default().fg(theme.border_focused)
    } else {
        Style::default().fg(theme.border)
    };
    let title = if is_stdin {
        if focused { " stdin  Enter send · Esc blur " } else { " stdin  Enter to type " }
    } else if focused {
        " Prompt  Enter send · Esc blur · Ctrl-K cancel "
    } else {
        " Prompt  Enter to type "
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(theme.content_bg));

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
        Line::from(Span::styled(input.as_str(), Style::default().fg(theme.text_dim)))
    };

    frame.render_widget(
        Paragraph::new(line)
            .block(block)
            .style(Style::default().fg(theme.text))
            .wrap(Wrap { trim: false }),
        area,
    );

    if focused {
        let inner_w = area.width.saturating_sub(2).max(1) as usize;
        let col     = char_display_width(input, cursor);
        let crow    = (col / inner_w) as u16;
        let ccol    = (col % inner_w) as u16;
        let max_row = area.height.saturating_sub(2);
        frame.set_cursor(area.x + 1 + ccol, (area.y + 1 + crow).min(area.y + max_row));
    }
}

fn draw_process_hint(frame: &mut Frame, area: Rect, tab: &crate::state::project::HorizontalTab, theme: &Theme) {
    let cmd_text = tab.command.as_deref().unwrap_or("(no command configured)");
    let hint_text = if tab.command.is_some() {
        "Press Enter to start · Ctrl-K to stop"
    } else {
        "No command configured — edit config.toml to add a command"
    };
    let lines = vec![
        Line::from(Span::styled(cmd_text, Style::default().fg(theme.border_focused).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled(hint_text, Style::default().fg(theme.text_dim))),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.content_bg)))
            .alignment(Alignment::Center),
        area,
    );
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

