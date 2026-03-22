use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use crate::state::project::HorizontalTab;

pub struct TabContentWidget<'a> {
    pub tab: &'a HorizontalTab,
}

impl<'a> Widget for TabContentWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Visible lines = height minus 2 border rows
        let visible = area.height.saturating_sub(2) as usize;
        let offset  = self.tab.content.clamped_offset(visible);

        let lines: Vec<Line> = self.tab.content.lines
            .iter()
            .skip(offset)
            .take(visible)
            .map(|s| Line::from(s.as_str()))
            .collect();

        Paragraph::new(Text::from(lines))
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}
