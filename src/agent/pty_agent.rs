use std::io::{Read, Write};
use tokio::sync::{mpsc, oneshot};
use vte::{Parser, Perform, Params};
use crate::state::agent::{AgentKind, AgentState};
use super::AgentMessage;

// ── Minimal VT screen emulator ──────────────────────────────────────────────

struct Screen {
    cols:       usize,
    rows:       usize,
    cells:      Vec<Vec<char>>,
    cursor_row: usize,
    cursor_col: usize,
}

impl Screen {
    fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols, rows,
            cells: vec![vec![' '; cols]; rows],
            cursor_row: 0,
            cursor_col: 0,
        }
    }

    fn newline(&mut self) {
        if self.cursor_row + 1 < self.rows {
            self.cursor_row += 1;
        } else {
            self.cells.remove(0);
            self.cells.push(vec![' '; self.cols]);
        }
    }

    fn to_lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self.cells
            .iter()
            .map(|row| row.iter().collect::<String>().trim_end().to_string())
            .collect();
        // Trim trailing blank lines
        while lines.last().map(|l: &String| l.is_empty()).unwrap_or(false) {
            lines.pop();
        }
        lines
    }
}

impl Perform for Screen {
    fn print(&mut self, c: char) {
        if self.cursor_row < self.rows && self.cursor_col < self.cols {
            self.cells[self.cursor_row][self.cursor_col] = c;
            self.cursor_col += 1;
        }
        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            self.newline();
        }
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x0D => { self.cursor_col = 0; }
            0x0A => { self.newline(); }
            0x08 => { if self.cursor_col > 0 { self.cursor_col -= 1; } }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, action: char) {
        let ps: Vec<u16> = params.iter()
            .map(|p| p.first().copied().unwrap_or(0))
            .collect();
        let p0 = ps.first().copied().unwrap_or(0);
        let p1 = ps.get(1).copied().unwrap_or(0);

        match action {
            'H' | 'f' => {
                let r = (p0.saturating_sub(1) as usize).min(self.rows.saturating_sub(1));
                let c = (p1.saturating_sub(1) as usize).min(self.cols.saturating_sub(1));
                self.cursor_row = r;
                self.cursor_col = c;
            }
            'A' => { self.cursor_row = self.cursor_row.saturating_sub(p0.max(1) as usize); }
            'B' => { self.cursor_row = (self.cursor_row + p0.max(1) as usize).min(self.rows.saturating_sub(1)); }
            'C' => { self.cursor_col = (self.cursor_col + p0.max(1) as usize).min(self.cols.saturating_sub(1)); }
            'D' => { self.cursor_col = self.cursor_col.saturating_sub(p0.max(1) as usize); }
            'G' => { self.cursor_col = (p0.saturating_sub(1) as usize).min(self.cols.saturating_sub(1)); }
            'J' => match p0 {
                0 => {
                    for c in self.cursor_col..self.cols { self.cells[self.cursor_row][c] = ' '; }
                    for r in (self.cursor_row + 1)..self.rows {
                        for c in 0..self.cols { self.cells[r][c] = ' '; }
                    }
                }
                1 => {
                    for r in 0..self.cursor_row {
                        for c in 0..self.cols { self.cells[r][c] = ' '; }
                    }
                    for c in 0..=self.cursor_col.min(self.cols.saturating_sub(1)) {
                        self.cells[self.cursor_row][c] = ' ';
                    }
                }
                2 | 3 => {
                    for r in 0..self.rows { for c in 0..self.cols { self.cells[r][c] = ' '; } }
                    self.cursor_row = 0; self.cursor_col = 0;
                }
                _ => {}
            }
            'K' => match p0 {
                0 => { for c in self.cursor_col..self.cols { self.cells[self.cursor_row][c] = ' '; } }
                1 => { for c in 0..=self.cursor_col.min(self.cols.saturating_sub(1)) { self.cells[self.cursor_row][c] = ' '; } }
                2 => { for c in 0..self.cols { self.cells[self.cursor_row][c] = ' '; } }
                _ => {}
            }
            'd' => { self.cursor_row = (p0.saturating_sub(1) as usize).min(self.rows.saturating_sub(1)); }
            _ => {} // Ignore SGR (m), cursor save/restore, etc.
        }
    }

    fn hook(&mut self, _: &Params, _: &[u8], _: bool, _: char) {}
    fn put(&mut self, _: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _: &[&[u8]], _: bool) {}
    fn esc_dispatch(&mut self, _: &[u8], _: bool, byte: u8) {
        match byte {
            b'7' => {} // Save cursor - ignore
            b'8' => {} // Restore cursor - ignore
            _ => {}
        }
    }
}

// ── PTY agent ───────────────────────────────────────────────────────────────

pub async fn run(
    command:      String,
    args:         Vec<String>,
    project_name: String,
    tab_kind:     AgentKind,
    cwd:          String,
    cols:         u16,
    rows:         u16,
    tx:           mpsc::UnboundedSender<AgentMessage>,
    cancel_rx:    oneshot::Receiver<()>,
    stdin_rx:     mpsc::UnboundedReceiver<Vec<u8>>,
) {
    use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

    let state_msg = |s: AgentState| AgentMessage::StateChange {
        project_name: project_name.clone(),
        tab_kind: tab_kind.clone(),
        new_state: s,
    };

    tx.send(state_msg(AgentState::Running)).ok();

    let pty_system = NativePtySystem::default();
    let pair = match pty_system.openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 }) {
        Ok(p) => p,
        Err(e) => {
            tx.send(AgentMessage::Chunk {
                project_name: project_name.clone(),
                tab_kind: tab_kind.clone(),
                text: format!("[pty error] {e}"),
                is_newline: true,
            }).ok();
            tx.send(state_msg(AgentState::Error)).ok();
            return;
        }
    };

    let mut cmd = CommandBuilder::new(&command);
    for arg in &args { cmd.arg(arg); }
    cmd.cwd(&cwd);
    // Set TERM so apps know they have a real terminal
    cmd.env("TERM", "xterm-256color");

    let mut child = match pair.slave.spawn_command(cmd) {
        Ok(c) => c,
        Err(e) => {
            tx.send(AgentMessage::Chunk {
                project_name: project_name.clone(),
                tab_kind: tab_kind.clone(),
                text: format!("[spawn error] {e}"),
                is_newline: true,
            }).ok();
            tx.send(state_msg(AgentState::Error)).ok();
            return;
        }
    };
    drop(pair.slave); // Must drop slave or PTY may deadlock

    let mut reader = pair.master.try_clone_reader().unwrap();
    let mut writer = pair.master.take_writer().unwrap();

    // Std channel: tokio stdin_rx → std write thread
    let (write_tx, write_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        while let Ok(bytes) = write_rx.recv() {
            if writer.write_all(&bytes).is_err() { break; }
            let _ = writer.flush();
        }
    });

    let write_tx2 = write_tx.clone();
    tokio::spawn(async move {
        let mut stdin_rx = stdin_rx;
        while let Some(bytes) = stdin_rx.recv().await {
            if write_tx2.send(bytes).is_err() { break; }
        }
    });

    // Std thread: PTY reader → tokio channel
    let (pty_tx, mut pty_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => { pty_tx.send(buf[..n].to_vec()).ok(); }
            }
        }
    });

    // Main async loop: parse PTY output and send screen snapshots
    let mut screen = Screen::new(cols as usize, rows as usize);
    let mut parser = Parser::new();
    let mut cancel_rx = std::pin::pin!(cancel_rx);

    loop {
        tokio::select! {
            biased;
            _ = &mut cancel_rx => {
                child.kill().ok();
                tx.send(state_msg(AgentState::Idle)).ok();
                return;
            }
            bytes_opt = pty_rx.recv() => {
                match bytes_opt {
                    None => break,
                    Some(bytes) => {
                        for &b in &bytes {
                            parser.advance(&mut screen, b);
                        }
                        tx.send(AgentMessage::ScreenUpdate {
                            project_name: project_name.clone(),
                            tab_kind: tab_kind.clone(),
                            lines: screen.to_lines(),
                        }).ok();
                    }
                }
            }
        }
    }

    // Wait for child exit (blocking — run in separate thread)
    let exit_status = tokio::task::spawn_blocking(move || child.wait()).await;
    match exit_status {
        Ok(Ok(s)) if s.success() => { tx.send(state_msg(AgentState::Idle)).ok(); }
        _ => { tx.send(state_msg(AgentState::Error)).ok(); }
    }
}
