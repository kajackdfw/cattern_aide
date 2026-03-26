use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};
use crate::{app::SidebarSelection, state::project::Project, ui::theme::Theme};

const PLUS_SLOT_H: u16 = 3;

pub const CLOSE_CHAR: &str = "✖";

/// Returns (x, y) of the close button for project slot `i`, vertically centred in the slot.
/// Returns `None` if the slot doesn't fit in the sidebar.
pub fn close_button_pos(sidebar: Rect, count: usize, i: usize) -> Option<(u16, u16)> {
    if count == 0 { return None; }
    let plus_y     = sidebar.bottom().saturating_sub(PLUS_SLOT_H);
    let projects_h = plus_y.saturating_sub(sidebar.top());
    let slot_h     = ((projects_h as usize) / count).max(1) as u16;
    let y_start    = sidebar.top() + i as u16 * slot_h;
    if y_start >= plus_y { return None; }
    let y_end = (y_start + slot_h).min(plus_y);
    if y_end == 0 || sidebar.right() == 0 { return None; }
    let y_mid = y_start + (y_end.saturating_sub(y_start)) / 2;
    Some((sidebar.right().saturating_sub(1), y_mid))
}

pub struct SidebarWidget<'a> {
    pub projects:    &'a [Project],
    pub sidebar_sel: &'a SidebarSelection,
    pub theme:       &'a Theme,
}

impl<'a> Widget for SidebarWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = self.theme;
        buf.set_style(area, Style::default().bg(t.sidebar_bg));

        let plus_y     = area.bottom().saturating_sub(PLUS_SLOT_H);
        let projects_h = plus_y.saturating_sub(area.top());
        let count      = self.projects.len();
        let slot_h     = if count == 0 || projects_h == 0 {
            0
        } else {
            ((projects_h as usize) / count).max(1) as u16
        };

        // ── Project slots ──────────────────────────────────────────────────────
        for (i, project) in self.projects.iter().enumerate() {
            let y_start = area.top() + (i as u16) * slot_h;
            if y_start >= plus_y {
                break;
            }

            let is_active = matches!(self.sidebar_sel, SidebarSelection::Project(j) if *j == i);
            let (fg, bg) = if is_active {
                (Color::White, t.sidebar_active_bg)
            } else {
                (t.sidebar_inactive_fg, t.sidebar_bg)
            };

            let y_end = (y_start + slot_h).min(plus_y);
            buf.set_style(
                Rect::new(area.left(), y_start, area.width, y_end - y_start),
                Style::default().bg(bg),
            );

            if is_active && area.width > 0 {
                buf.set_string(
                    area.left(), y_start, "▌",
                    Style::default().fg(t.border_focused).bg(bg),
                );
            }

            let text_x = area.left() + if is_active { 2 } else { 1 };
            if text_x < area.right() {
                let text_style = Style::default().fg(fg).bg(bg);
                for (j, ch) in project.name.chars().enumerate() {
                    let y = y_start + j as u16;
                    if y >= y_end { break; }
                    buf.set_string(text_x, y, &ch.to_string(), text_style);
                }
            }

            // Close button centred vertically in slot
            if let Some((bx, by)) = close_button_pos(area, count, i) {
                buf.set_string(
                    bx, by, CLOSE_CHAR,
                    Style::default().fg(if is_active { t.border_focused } else { t.text_muted }).bg(bg),
                );
            }
        }

        // ── "+" add-project slot ───────────────────────────────────────────────
        let is_add = matches!(self.sidebar_sel, SidebarSelection::AddButton);
        let (add_fg, add_bg) = if is_add {
            (Color::White, t.sidebar_active_bg)
        } else {
            (Color::Green, t.sidebar_bg)
        };

        if plus_y < area.bottom() {
            if plus_y > area.top() {
                for x in area.left()..area.right() {
                    buf.set_string(x, plus_y, "─",
                        Style::default().fg(t.border).bg(t.sidebar_bg));
                }
            }

            let slot_start = (plus_y + 1).min(area.bottom());
            let slot_h = area.bottom().saturating_sub(slot_start);
            if slot_h > 0 {
                buf.set_style(
                    Rect::new(area.left(), slot_start, area.width, slot_h),
                    Style::default().bg(add_bg),
                );
            }

            let mid_y = slot_start;
            if mid_y < area.bottom() {
                if is_add && area.width > 0 {
                    buf.set_string(
                        area.left(), mid_y, "▌",
                        Style::default().fg(t.border_focused).bg(add_bg),
                    );
                }

                let plus_x = area.left() + if is_add { 2 } else { 1 };
                if plus_x < area.right() {
                    buf.set_string(
                        plus_x, mid_y, "+",
                        Style::default().fg(add_fg).bg(add_bg),
                    );
                }
            }
        }
    }
}
