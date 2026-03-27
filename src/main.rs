use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};
use tokio::time::{interval, MissedTickBehavior};

mod agent;
mod app;
mod clipboard;
mod config;
mod events;
mod state;
mod ui;

use app::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Logging — file only; never write to stdout after raw mode is enabled
    let _ = std::fs::create_dir_all("logs");
    let file_appender = tracing_appender::rolling::daily("logs", "research_ai_ide.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    // Config
    let cfg     = config::load("config.toml").unwrap_or_else(|_| config::default_config());
    let tick_ms = cfg.general.tick_ms;
    let mut app = App::from_config(cfg, "config.toml");

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Event loop
    let mut tick_interval = interval(Duration::from_millis(tick_ms));
    tick_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut events = EventStream::new();

    loop {
        app.drain_agents();
        let frame_size = terminal.draw(|f| ui::draw(f, &app))?.area;
        app.terminal_size = frame_size;

        if app.should_quit {
            break;
        }

        tokio::select! {
            _ = tick_interval.tick() => {}
            Some(event) = events.next() => {
                match event {
                    Ok(Event::Key(key))          => app.handle_key(key),
                    Ok(Event::Resize(cols, rows)) => app.handle_resize(cols, rows),
                    Ok(Event::Mouse(m))           => {
                        let size = terminal.size().unwrap_or_default();
                        app.handle_mouse(m, size);
                    }
                    Ok(_)  => {}
                    Err(e) => tracing::error!("event error: {e}"),
                }
            }
        }
    }

    // Teardown
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}
