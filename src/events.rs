use crossterm::event::{KeyEvent, MouseEvent};

#[allow(dead_code)]
pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Tick,
    Quit,
}
