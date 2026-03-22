use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};
use crate::{app::SidebarSelection, state::project::Project};

const PLUS_SLOT_H: u16 = 3;

pub struct SidebarWidget<'a> {
    pub projects:    &'a [Project],
    pub sidebar_sel: &'a SidebarSelection,
}

impl<'a> Widget for SidebarWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, Style::default().bg(Color::Rgb(20, 20, 40)));

        // Reserve PLUS_SLOT_H rows at the bottom for the "+" button.
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
                (Color::White, Color::Rgb(40, 40, 100))
            } else {
                (Color::Rgb(160, 160, 190), Color::Rgb(20, 20, 40))
            };

            let y_end = (y_start + slot_h).min(plus_y);
            buf.set_style(
                Rect::new(area.left(), y_start, area.width, y_end - y_start),
                Style::default().bg(bg),
            );

            if is_active && area.width > 0 {
                buf.set_string(
                    area.left(), y_start, "▌",
                    Style::default().fg(Color::Cyan).bg(bg),
                );
            }

            let text_x = area.left() + if is_active { 2 } else { 1 };
            if text_x < area.right() {
                let text_style = Style::default().fg(fg).bg(bg);
                for (j, ch) in project.name.chars().enumerate() {
                    let y = y_start + j as u16;
                    if y >= y_end {
                        break;
                    }
                    buf.set_string(text_x, y, &ch.to_string(), text_style);
                }
            }
        }

        // ── "+" add-project slot ───────────────────────────────────────────────
        let is_add = matches!(self.sidebar_sel, SidebarSelection::AddButton);
        let (add_fg, add_bg) = if is_add {
            (Color::White, Color::Rgb(40, 40, 80))
        } else {
            (Color::Green, Color::Rgb(20, 20, 40))
        };

        if plus_y < area.bottom() {
            // Separator line above the "+" slot
            if plus_y > area.top() {
                for x in area.left()..area.right() {
                    buf.set_string(x, plus_y, "─",
                        Style::default().fg(Color::Rgb(60, 60, 80)).bg(Color::Rgb(20, 20, 40)));
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
                        Style::default().fg(Color::Cyan).bg(add_bg),
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

// Phase 2: replace char-stacking with PNG image pipeline:
//   ab_glyph + imageproc → 20×140 px canvas → rotate270 → ratatui_image::StatefulImage
//   Font: assets/Inconsolata-Regular.ttf embedded via include_bytes!
