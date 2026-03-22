# AIDE_PLAN.md — research_ai_ide (Rust + Ratatui Terminal AI IDE)

## Context
Build a terminal UI application to manage multiple AI-assisted projects. Each project gets a vertically-stacked sidebar tab (text rotated 90° CCW via character stacking). Within each project, horizontal tabs (AI Prompt, Target Code, named agents) display streaming output with LED status indicators. Claude integration supports both direct API (SSE streaming) and subprocess (`claude -p`) modes, with output routed to specific tab containers via async channels.

---

## Tech Stack
- **Rust** + **ratatui 0.27** + **crossterm 0.27** (event-stream feature)
- **tokio** (full features) for async runtime
- **reqwest 0.12** (json + stream) for Anthropic SSE API
- **serde / serde_json** for config + SSE parsing
- **anyhow / thiserror** for error handling
- **tracing + tracing-appender** for file-only logging (never stdout — corrupts TUI)

---

## File Structure

<pre style="background-color:#1a1a2e; color:#90EE90; padding:1em; border-radius:6px; font-family:monospace;">
research_ai_ide/
├── Cargo.toml
├── config.toml.example
└── src/
    ├── main.rs               # tokio entry, terminal setup/teardown, event loop
    ├── app.rs                # App struct, handle_key, drain_agent_messages
    ├── config.rs             # Config (api_key, model, agent_mode, projects)
    ├── events.rs             # AppEvent enum (Key, Mouse, Resize, Tick)
    ├── ui/
    │   ├── mod.rs            # draw(frame, &amp;app) top-level
    │   ├── layout.rs         # sidebar (6 cols) + main area split
    │   ├── sidebar.rs        # VerticalTabWidget (custom Widget impl)
    │   ├── main_area.rs      # horizontal Tabs widget + LED spans
    │   ├── tab_content.rs    # Paragraph renderer for TextContainer
    │   └── led.rs            # led_span(state) → Span with ● + color
    ├── state/
    │   ├── project.rs        # Project, HorizontalTab, AgentKind
    │   ├── agent.rs          # AgentState enum (Idle/Running/Waiting/Error)
    │   └── container.rs      # TextContainer (push_line, push_partial, scroll)
    └── agent/
        ├── mod.rs            # AgentManager, AgentMessage enum, drain_into
        ├── api_agent.rs      # Anthropic SSE streaming → AgentMessage channel
        ├── subprocess_agent.rs # claude -p subprocess → AgentMessage channel
        └── http_proxy_agent.rs # axum proxy → request/response Chunk messages
</pre>

---

## Core Data Structures

### AgentState (`src/state/agent.rs`)

<pre style="background-color:#1a1a2e; color:#90EE90; padding:1em; border-radius:6px; font-family:monospace;">
pub enum AgentState { Idle, Running, Waiting, Error }
pub enum AgentKind  {
    AiPrompt,
    TargetCode,
    Named(String),
    Process(String),
    HttpProxy { port: u16, target: String },  // e.g. port 8081 → localhost:8080
}
</pre>

### TextContainer (`src/state/container.rs`)

<pre style="background-color:#1a1a2e; color:#90EE90; padding:1em; border-radius:6px; font-family:monospace;">
pub struct TextContainer {
    pub lines:         Vec&lt;String&gt;,
    pub scroll_offset: usize,
    pub max_lines:     usize,   // ring-buffer cap, e.g. 10_000
}
// push_line()    — newline-terminated
// push_partial() — mid-line SSE token chunk
</pre>

### AgentMessage (`src/agent/mod.rs`)

<pre style="background-color:#1a1a2e; color:#90EE90; padding:1em; border-radius:6px; font-family:monospace;">
pub enum AgentMessage {
    Chunk       { project_name, tab_kind, text, is_newline: bool },
    StateChange { project_name, tab_kind, new_state: AgentState },
}
</pre>

Routing key is `(project_name, tab_kind)`. `drain_into(&mut projects)` called on every tick via `try_recv` (non-blocking).

---

## Key Architectural Decisions

### Rotated Text (Sidebar) — PNG-based

Each sidebar tab label is a **20 × 140 px PNG generated in memory** at startup (and on project rename). The name is rendered at 18 px, truncated with `…` if it exceeds 136 px, then the canvas is rotated 90° CCW. Display is handled by `ratatui-image`:

- **Universal fallback**: half-block Unicode (`▀▄` with 24-bit ANSI color) — works in any true-color terminal
- **Auto-upgrade**: Kitty graphics protocol, Sixel, or iTerm2 inline images when the terminal supports them

**Additional Cargo dependencies:**

<pre style="background-color:#1a1a2e; color:#90EE90; padding:1em; border-radius:6px; font-family:monospace;">
ratatui-image = { version = "2", features = ["crossterm"] }
image          = "0.25"   # ImageBuffer, rotate270
ab_glyph       = "0.2"    # embed TTF, rasterize glyphs
imageproc      = "0.25"   # draw_text_mut
</pre>

**Image generation pipeline** (`src/ui/sidebar.rs`):
1. Embed font: `ab_glyph::FontRef::try_from_slice(include_bytes!("../../assets/font.ttf"))`
2. Create a `20 × 140` RGBA `ImageBuffer` filled with the dark background color
3. Measure glyph advance widths at 18 px scale; truncate name + append `…` until ≤ 136 px
4. Draw text with `imageproc::draw_text_mut` at (2, 0)
5. Rotate 90° CCW with `image::imageops::rotate270` → produces a `140 × 20` px image that fits the sidebar cell
6. Convert to `DynamicImage`; cache as a `StatefulImage` per project

**Asset:** `assets/Inconsolata-Regular.ttf` embedded via `include_bytes!` — no runtime font file required.

Active-tab indicator (`▌` Span) is rendered on top of the image cell as before.

### LED Indicators
`led_span(state)` returns `Span::styled("●", Style::fg(color))`:
- Blue = Running, Gray = Idle, Yellow = Waiting, Red = Error

Each horizontal tab title is `Line::from(vec![led_span, " ", tab_name])` fed into `ratatui::widgets::Tabs`.

### Event Loop (`src/main.rs`)

<pre style="background-color:#1a1a2e; color:#90EE90; padding:1em; border-radius:6px; font-family:monospace;">
loop {
    agent_manager.drain_into(&amp;mut app.projects);  // flush channels → state
    terminal.draw(|f| ui::draw(f, &amp;app));         // render
    tokio::select! {
        _ = tick_interval.tick() => {}             // 50ms / 20 FPS
        event = crossterm_events.next() => { ... } // key/mouse/resize
    }
}
</pre>

Draw is pure (`&App`). All mutation is in `handle_key` + `drain_into`.

### HTTP Proxy Tab (`AgentKind::HttpProxy`)
Spawns an in-process `axum` listener on `port` that forwards all requests to `target`:

1. Bind `axum` on `0.0.0.0:{port}` in a background `tokio::task`
2. For each incoming request, capture method, path, headers, and body
3. Forward to `target` via `reqwest`
4. Capture response status and body
5. Send formatted `Chunk` messages to the tab:
   ```
   → POST /api/users
     Body: {"name": "alice"}
   ← 201 Created
     Body: {"id": 42, "name": "alice"}
   ```
6. Uses the same `oneshot` cancellation as `Process` tabs (`Ctrl-K` shuts down the listener)

**Additional Cargo dependency:**
```toml
axum = "0.7"
```

Routing key and `AgentMessage` variants are unchanged — proxy events are just `Chunk` messages like any other tab.

### Claude API Agent (`src/agent/api_agent.rs`)
1. Send `StateChange(Running)`
2. POST to `https://api.anthropic.com/v1/messages` with `"stream": true`
3. Buffer raw bytes; process complete SSE events delimited by `"\n\n"`
4. Parse `content_block_delta` → `delta.text`; split on `\n`; send `Chunk` messages with `is_newline` flag
5. Send `StateChange(Idle)` or `StateChange(Error)` on completion

### Subprocess Agent (`src/agent/subprocess_agent.rs`)
1. `tokio::process::Command::new("claude").args(["-p", prompt])`
2. `stdout(Stdio::piped())` + `stderr(Stdio::piped())`
3. `tokio::select!` loop over `BufReader::lines()` on both streams
4. Each line → `Chunk { is_newline: true }`; stderr lines prefixed with `[stderr]`
5. If line-buffering issues arise, wrap with `stdbuf -oL claude -p ...`

### Process Tab (`AgentKind::Process(String)`)
Long-running processes (e.g. `vite dev`, `npm run watch`) use the same subprocess pipe as above, with two additions:

- **Cancellation**: each spawned task receives a `tokio::sync::oneshot::Receiver<()>`. The `AgentManager` holds the corresponding `Sender`. When the user kills the tab (e.g. `Ctrl-K`), `AgentManager::kill(project, tab_kind)` sends on the channel; the task calls `child.kill().await` and exits.
- **No auto-respawn**: process tabs stay in `Error` state after unexpected exit so the user can see the exit output. Re-run is triggered explicitly (same key as initial spawn).

`AgentMessage` gains no new variants — stdout/stderr lines are still `Chunk` messages and state transitions still use `StateChange`.

---

## Keyboard Navigation
| Key | Action |
|-----|--------|
| `j/k` or `↑/↓` | Switch project (sidebar) |
| `h/l` or `←/→` | Switch horizontal tab |
| `PgUp/PgDn` | Scroll tab content |
| `Enter` on AI Prompt | Submit prompt → spawn agent |
| `Ctrl-K` | Kill running process on current tab |
| `Tab` | Cycle tabs forward |
| `q` / `Ctrl-C` | Quit |

---

## Build Order
1. `Cargo.toml` dependencies
2. `src/state/` — pure structs, no async
3. `src/config.rs` — TOML loading
4. `src/ui/led.rs` + `sidebar.rs` — verify custom Widget API
5. `src/ui/main_area.rs` + `tab_content.rs` + `mod.rs` — full draw path
6. `src/app.rs` + `src/main.rs` — terminal setup + event loop (render only)
7. `src/agent/mod.rs` — AgentManager + channels
8. `src/agent/api_agent.rs` — SSE streaming
9. `src/agent/subprocess_agent.rs` — subprocess pipe
10. `src/agent/http_proxy_agent.rs` — axum proxy listener
11. Wire `handle_key(Enter)` → `agent_manager.spawn_*`

---

## Answering "Can Claude Code be channeled to text containers?"

**Yes.** Two mechanisms:

1. **Subprocess mode**: Run `claude -p "<prompt>"` as a `tokio::process::Command` with `stdout(Stdio::piped())`. A background task reads each line via `BufReader::lines()` and sends it as an `AgentMessage::Chunk` to the correct tab container. This works for any Claude Code session, including multi-step agentic runs.

2. **API mode**: Call the Anthropic Messages API with `"stream": true` and consume the SSE byte stream. Each `content_block_delta` event carries a `delta.text` token that is sent immediately to the tab container, giving real-time streaming output.

In both cases, the routing is: `tokio task → mpsc channel → AgentManager::drain_into → TextContainer.push_line/push_partial → Ratatui Paragraph widget`.

---

## Verification
- `cargo build` — should compile clean
- `cargo run` — terminal opens with sidebar showing configured projects
- Manual: add a project in config, verify sidebar tab appears with rotated name
- Manual: open AI Prompt tab, type a prompt, press Enter → LED turns Blue → streams response → LED turns Gray
- Manual: trigger an error (bad API key) → LED turns Red
- Manual: resize terminal → layout reflows without artifacts
- Check `logs/research_ai_ide.log` for tracing output (nothing should appear on screen)
