use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Tabs},
    Frame,
};
use crate::{
    app::App,
    ui::{led::led_span, tab_content::TabContentWidget},
};

pub fn draw_main(frame: &mut Frame, area: Rect, app: &App) {
    let Some(idx)     = app.active_project_index()  else { return };
    let Some(project) = app.projects.get(idx)        else { return };

    // Build tab titles: ● label
    let titles: Vec<Line> = project.tabs.iter().map(|tab| {
        Line::from(vec![
            led_span(&tab.state),
            Span::raw(" "),
            Span::raw(tab.label.as_str()),
        ])
    }).collect();

    // Split vertically: tabs bar (3 rows) + content
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(project.name.as_str()))
        .select(project.active_tab)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(tabs, chunks[0]);

    if let Some(tab) = project.tabs.get(project.active_tab) {
        frame.render_widget(TabContentWidget { tab }, chunks[1]);
    }
}
