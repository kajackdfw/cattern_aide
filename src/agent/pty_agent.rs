use std::io::{Read, Write};
use tokio::sync::{mpsc, oneshot};
use crate::state::agent::{AgentKind, AgentState};
use crate::state::pty_screen::{PtyCell, PtyColor, PtyScreen};
use super::AgentMessage;

fn vt100_color(c: vt100::Color) -> PtyColor {
    match c {
        vt100::Color::Default      => PtyColor::Default,
        vt100::Color::Idx(n)       => PtyColor::Indexed(n),
        vt100::Color::Rgb(r, g, b) => PtyColor::Rgb(r, g, b),
    }
}

fn screen_to_pty(screen: &vt100::Screen) -> PtyScreen {
    let (rows, cols) = screen.size();
    (0..rows).map(|r| {
        (0..cols).map(|c| {
            let cell = screen.cell(r, c);
            let contents = cell.map(|c| c.contents().to_string()).unwrap_or_default();
            let fg  = cell.map(|c| vt100_color(c.fgcolor())).unwrap_or(PtyColor::Default);
            let bg  = cell.map(|c| vt100_color(c.bgcolor())).unwrap_or(PtyColor::Default);
            let bold      = cell.map(|c| c.bold()).unwrap_or(false);
            let italic    = cell.map(|c| c.italic()).unwrap_or(false);
            let underline = cell.map(|c| c.underline()).unwrap_or(false);
            let reversed  = cell.map(|c| c.inverse()).unwrap_or(false);
            PtyCell { ch: contents, fg, bg, bold, italic, underline, reversed }
        }).collect()
    }).collect()
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
    resize_rx:    mpsc::UnboundedReceiver<(u16, u16)>,
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
    let mut parser = vt100::Parser::new(rows, cols, 0);
    let mut cancel_rx = std::pin::pin!(cancel_rx);
    let mut resize_rx = resize_rx;

    loop {
        tokio::select! {
            biased;
            _ = &mut cancel_rx => {
                child.kill().ok();
                tx.send(state_msg(AgentState::Idle)).ok();
                return;
            }
            resize = resize_rx.recv() => {
                if let Some((new_cols, new_rows)) = resize {
                    let _ = pair.master.resize(PtySize {
                        rows: new_rows, cols: new_cols,
                        pixel_width: 0, pixel_height: 0,
                    });
                    parser.set_size(new_rows, new_cols);
                }
            }
            bytes_opt = pty_rx.recv() => {
                match bytes_opt {
                    None => break,
                    Some(bytes) => {
                        parser.process(&bytes);
                        tx.send(AgentMessage::ScreenUpdate {
                            project_name: project_name.clone(),
                            tab_kind: tab_kind.clone(),
                            screen: screen_to_pty(parser.screen()),
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
