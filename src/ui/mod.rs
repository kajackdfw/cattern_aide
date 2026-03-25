pub mod file_tree;
pub mod git_panel;
pub mod layout;
pub mod led;
pub mod main_area;
pub mod modal;
pub mod sidebar;
pub mod tab_content;

use ratatui::Frame;
use crate::app::App;
use sidebar::SidebarWidget;

pub fn draw(frame: &mut Frame, app: &App) {
    let layout = layout::compute(frame);

    frame.render_widget(
        SidebarWidget { projects: &app.projects, sidebar_sel: &app.sidebar_sel },
        layout.sidebar,
    );

    main_area::draw_main(frame, layout.main, app);

    // Modal overlay — rendered last so it appears on top of everything
    if let Some(m) = &app.modal {
        modal::draw_modal(frame, m);
    }
}
