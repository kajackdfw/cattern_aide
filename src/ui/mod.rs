pub mod file_tree;
pub mod git_panel;
pub mod layout;
pub mod led;
pub mod main_area;
pub mod modal;
pub mod sidebar;
pub mod tab_content;
pub mod theme;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use crate::app::App;
use sidebar::SidebarWidget;

pub fn draw(frame: &mut Frame, app: &App) {
    let layout = layout::compute(frame);

    let t = active_theme(app);

    frame.render_widget(
        SidebarWidget { projects: &app.projects, sidebar_sel: &app.sidebar_sel, theme: &t },
        layout.sidebar,
    );

    main_area::draw_main(frame, layout.main, app, &t);

    // Modal overlay — rendered last so it appears on top of everything
    if let Some(m) = &app.modal {
        modal::draw_modal(frame, m, &app.config.recently_closed);
    }

    // "Copied!" flash notification
    if app.copy_flash.is_some() {
        let size = frame.size();
        let w = 14u16;
        let h = 3u16;
        let x = size.width.saturating_sub(w + 1);
        let y = size.height.saturating_sub(h + 1);
        let area = Rect::new(x, y, w, h);
        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new(Span::styled(" ⎘ Copied! ", Style::default().fg(t.text_accent).add_modifier(Modifier::BOLD)))
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(t.border_focused)))
                .style(Style::default().bg(t.content_bg)),
            area,
        );
    }
}

fn active_theme(app: &App) -> theme::Theme {
    app.active_project_index()
        .and_then(|i| app.projects.get(i))
        .map(|p| theme::by_name(&p.theme))
        .unwrap_or_else(theme::dawn)
}
