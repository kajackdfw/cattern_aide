use ratatui::style::Color;

pub struct Theme {
    pub bg:                  Color,  // primary panel background
    pub bg_alt:              Color,  // alternate bg (git panel etc.)
    pub content_bg:          Color,  // tab content / prompt input bg
    pub border:              Color,  // unfocused border
    pub border_focused:      Color,  // focused border / accent
    pub text:                Color,  // primary text
    pub text_dim:            Color,  // secondary / inactive text
    pub text_muted:          Color,  // very dim (empty states)
    pub text_accent:         Color,  // accent text (dirs, branches, keywords)
    pub sel_fg:              Color,  // selected item fg
    pub sel_bg:              Color,  // selected item bg (unfocused)
    pub sel_bg_focused:      Color,  // selected item bg (focused)
    pub sidebar_bg:          Color,
    pub sidebar_active_bg:   Color,
    pub sidebar_inactive_fg: Color,
}

pub fn dawn() -> Theme {
    Theme {
        bg:                  Color::Rgb(12, 12, 28),
        bg_alt:              Color::Rgb(10, 10, 22),
        content_bg:          Color::Rgb(8, 8, 18),
        border:              Color::Rgb(60, 60, 100),
        border_focused:      Color::Cyan,
        text:                Color::Rgb(200, 200, 215),
        text_dim:            Color::Rgb(120, 120, 155),
        text_muted:          Color::Rgb(80, 80, 100),
        text_accent:         Color::Rgb(170, 150, 255),
        sel_fg:              Color::Black,
        sel_bg:              Color::Rgb(70, 70, 130),
        sel_bg_focused:      Color::Cyan,
        sidebar_bg:          Color::Rgb(20, 20, 40),
        sidebar_active_bg:   Color::Rgb(40, 40, 100),
        sidebar_inactive_fg: Color::Rgb(160, 160, 190),
    }
}

pub fn matrix() -> Theme {
    Theme {
        bg:                  Color::Rgb(0, 8, 4),
        bg_alt:              Color::Rgb(0, 6, 2),
        content_bg:          Color::Rgb(0, 4, 2),
        border:              Color::Rgb(0, 60, 30),
        border_focused:      Color::Rgb(0, 200, 100),
        text:                Color::Rgb(180, 240, 180),
        text_dim:            Color::Rgb(100, 170, 100),
        text_muted:          Color::Rgb(40, 100, 40),
        text_accent:         Color::Rgb(100, 255, 150),
        sel_fg:              Color::Black,
        sel_bg:              Color::Rgb(0, 55, 28),
        sel_bg_focused:      Color::Rgb(0, 200, 100),
        sidebar_bg:          Color::Rgb(0, 12, 6),
        sidebar_active_bg:   Color::Rgb(0, 40, 20),
        sidebar_inactive_fg: Color::Rgb(100, 200, 120),
    }
}

pub fn by_name(name: &str) -> Theme {
    match name {
        "matrix" => matrix(),
        _         => dawn(),
    }
}

/// All valid theme names in display order.
pub const NAMES: &[&str] = &["dawn", "matrix"];

pub fn next_theme(current: &str) -> &'static str {
    let pos = NAMES.iter().position(|&n| n == current).unwrap_or(0);
    NAMES[(pos + 1) % NAMES.len()]
}
