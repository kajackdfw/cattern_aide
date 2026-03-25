use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use crate::state::filetree::FileTreeState;

pub fn draw_file_tree(frame: &mut Frame, area: Rect, state: &FileTreeState, focused: bool) {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Rgb(60, 60, 100))
    };

    let block = Block::default()
        .title(if focused { " Files  Esc to exit · r reload · Enter expand " }
               else       { " Files  f to focus " })
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(Color::Rgb(12, 12, 28)));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let n = state.entries.len();
    if n == 0 {
        frame.render_widget(
            Paragraph::new("  (empty)")
                .style(Style::default().fg(Color::Rgb(80, 80, 100))),
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
            // Truncate name to avoid wrapping
            let max_name = (area.width as usize)
                .saturating_sub(2)                    // border
                .saturating_sub(entry.depth * 2 + 2); // indent + icon
            let name: String = entry.name.chars().take(max_name.max(1)).collect();
            let text = format!("{}{}{}", indent, icon, name);

            if abs == state.selected {
                Line::from(Span::styled(
                    text,
                    Style::default()
                        .fg(Color::Black)
                        .bg(if focused { Color::Cyan } else { Color::Rgb(70, 70, 130) })
                        .add_modifier(Modifier::BOLD),
                ))
            } else if entry.is_dir {
                Line::from(Span::styled(text, Style::default().fg(Color::Rgb(170, 150, 255))))
            } else {
                Line::from(Span::styled(text, Style::default().fg(Color::Rgb(200, 200, 215))))
            }
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), inner);

    // Scrollbar — only when content overflows
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

/// Keep the selected row centred in the viewport.
fn scroll_offset(selected: usize, visible: usize, total: usize) -> usize {
    if total <= visible { return 0; }
    let max = total - visible;
    selected.saturating_sub(visible / 2).min(max)
}
