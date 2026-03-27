use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use crate::{
    state::{agent::AgentKind, project::HorizontalTab, pty_screen::{PtyCell, PtyColor}},
    ui::theme::Theme,
};

pub struct TabContentWidget<'a> {
    pub tab:          &'a HorizontalTab,
    pub theme:        &'a Theme,
    pub pty_focused:  bool,
}

impl<'a> Widget for TabContentWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let is_pty = matches!(self.tab.kind, AgentKind::PtyProcess(_));

        if is_pty {
            let (hint, border_color) = if self.pty_focused {
                (" keys → pty · Ctrl+] detach ", self.theme.border_focused)
            } else {
                (" Enter to interact ", self.theme.border)
            };
            let block = Block::default().borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(self.theme.content_bg))
                .title_bottom(hint);
            let inner = block.inner(area);
            block.render(area, buf);

            let avail_rows = inner.height as usize;
            let avail_cols = inner.width as usize;
            let screen = &self.tab.pty_screen;

            for (row_idx, row) in screen.iter().take(avail_rows).enumerate() {
                let y = inner.y + row_idx as u16;
                for (col_idx, cell) in row.iter().take(avail_cols).enumerate() {
                    let x = inner.x + col_idx as u16;
                    let style = pty_cell_style(cell, self.theme);
                    let ch = if cell.ch.is_empty() { " " } else { cell.ch.as_str() };
                    buf.get_mut(x, y).set_symbol(ch).set_style(style);
                }
            }
            return;
        }

        let visible = area.height.saturating_sub(2) as usize;
        let offset  = self.tab.content.clamped_offset(visible);
        let is_git  = self.tab.kind == AgentKind::Git;

        let lines: Vec<Line> = self.tab.content.lines
            .iter()
            .skip(offset)
            .take(visible)
            .map(|s| {
                if is_git { diff_line(s, self.theme) } else { ai_line(s, self.theme) }
            })
            .collect();

        let block = Block::default().borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border))
            .style(Style::default().bg(self.theme.content_bg));

        Paragraph::new(Text::from(lines))
            .block(block)
            .style(Style::default().fg(self.theme.text).bg(self.theme.content_bg))
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}

fn pty_cell_style(cell: &PtyCell, theme: &Theme) -> Style {
    let mut fg = pty_color(&cell.fg, theme.text);
    let mut bg = pty_color(&cell.bg, theme.content_bg);
    if cell.reversed { std::mem::swap(&mut fg, &mut bg); }
    let mut style = Style::default().fg(fg).bg(bg);
    if cell.bold      { style = style.add_modifier(Modifier::BOLD); }
    if cell.italic    { style = style.add_modifier(Modifier::ITALIC); }
    if cell.underline { style = style.add_modifier(Modifier::UNDERLINED); }
    style
}

fn pty_color(c: &PtyColor, default: Color) -> Color {
    match c {
        PtyColor::Default      => default,
        PtyColor::Rgb(r, g, b) => Color::Rgb(*r, *g, *b),
        PtyColor::Indexed(n)   => Color::Indexed(*n),
    }
}

/// Render a single AI output line, highlighting ``` fence openers with a copy hint.
pub fn ai_line<'a>(line: &'a str, theme: &Theme) -> Line<'a> {
    let trimmed = line.trim();
    if trimmed.starts_with("```") {
        // Opening or closing fence — show ⎘ hint on opening fences
        Line::from(vec![
            Span::styled(line, Style::default().fg(theme.text_accent)),
            Span::styled("  ⎘ click to copy", Style::default().fg(theme.text_muted)),
        ])
    } else {
        Line::from(Span::styled(line, Style::default().fg(theme.text)))
    }
}

/// Colorize a single line of unified diff output.
pub fn diff_line<'a>(line: &'a str, theme: &Theme) -> Line<'a> {
    let style = if line.starts_with("+++") || line.starts_with("---") {
        // File path header lines — dim
        Style::default().fg(theme.text_dim)
    } else if line.starts_with('+') {
        Style::default().fg(Color::Rgb(80, 210, 80))
    } else if line.starts_with('-') {
        Style::default().fg(Color::Rgb(210, 70, 70))
    } else if line.starts_with("@@") {
        // Hunk header — cyan accent
        Style::default().fg(Color::Rgb(80, 190, 210))
    } else if line.starts_with("diff ")
        || line.starts_with("index ")
        || line.starts_with("new file")
        || line.starts_with("deleted file")
        || line.starts_with("rename ")
        || line.starts_with("Binary ")
    {
        Style::default().fg(theme.text_dim)
    } else {
        Style::default().fg(theme.text)
    };
    Line::from(Span::styled(line, style))
}
