use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use crate::{state::filetree::FileTreeState, ui::theme::Theme};

pub fn draw_file_tree(frame: &mut Frame, area: Rect, state: &FileTreeState, focused: bool, theme: &Theme) {
    let border_style = if focused {
        Style::default().fg(theme.border_focused)
    } else {
        Style::default().fg(theme.border)
    };

    let block = Block::default()
        .title(if focused { " Files  Esc to exit · r reload · Enter expand " }
               else       { " Files  f to focus " })
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(theme.bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let n = state.entries.len();
    if n == 0 {
        frame.render_widget(
            Paragraph::new("  (empty)")
                .style(Style::default().fg(theme.text_muted)),
            inner,
        );
        return;
    }

    let visible = inner.height as usize;
    let offset  = scroll_offset(state.selected, visible, n);

    let lines: Vec<Line> = state.entries.iter()
        .skip(offset)
        .take(visible)
        .enumerate()
        .map(|(i, entry)| {
            let abs = offset + i;
            let indent = "  ".repeat(entry.depth);
            let icon = if entry.is_dir {
                if entry.expanded { "▼ " } else { "▶ " }
            } else {
                "  "
            };
            let max_name = (area.width as usize)
                .saturating_sub(2)
                .saturating_sub(entry.depth * 2 + 2);
            let name: String = entry.name.chars().take(max_name.max(1)).collect();
            let text = format!("{}{}{}", indent, icon, name);

            if abs == state.selected {
                Line::from(Span::styled(
                    text,
                    Style::default()
                        .fg(theme.sel_fg)
                        .bg(if focused { theme.sel_bg_focused } else { theme.sel_bg })
                        .add_modifier(Modifier::BOLD),
                ))
            } else if entry.is_dir {
                Line::from(Span::styled(text, Style::default().fg(theme.text_accent)))
            } else {
                Line::from(Span::styled(text, Style::default().fg(theme.text)))
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

fn scroll_offset(selected: usize, visible: usize, total: usize) -> usize {
    if total <= visible { return 0; }
    let max = total - visible;
    selected.saturating_sub(visible / 2).min(max)
}
