#[derive(Debug, Clone, Default)]
pub struct TextContainer {
    pub lines:         Vec<String>,
    pub scroll_offset: usize,
    pub max_lines:     usize,
}

impl TextContainer {
    pub fn new() -> Self {
        Self { lines: Vec::new(), scroll_offset: 0, max_lines: 10_000 }
    }

    pub fn push_line(&mut self, text: String) {
        self.lines.push(text);
        if self.lines.len() > self.max_lines {
            self.lines.remove(0);
            self.scroll_offset = self.scroll_offset.saturating_sub(1);
        }
    }

    pub fn push_partial(&mut self, text: &str) {
        match self.lines.last_mut() {
            Some(last) => last.push_str(text),
            None       => self.lines.push(text.to_string()),
        }
    }

    /// Replace all lines at once (used by PTY screen snapshots).
    pub fn set_lines(&mut self, lines: Vec<String>) {
        self.lines = lines;
        if self.lines.len() > self.max_lines {
            self.lines.drain(0..self.lines.len() - self.max_lines);
        }
    }

    /// Replace the content of the last line in-place (used to update partial lines).
    pub fn replace_last_line(&mut self, text: String) {
        match self.lines.last_mut() {
            Some(last) => *last = text,
            None       => self.lines.push(text),
        }
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    pub fn scroll_down(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(n);
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.scroll_offset = 0;
    }

    /// Returns a clamped offset without mutating self.
    pub fn clamped_offset(&self, visible_lines: usize) -> usize {
        let max = self.lines.len().saturating_sub(visible_lines);
        self.scroll_offset.min(max)
    }
}
