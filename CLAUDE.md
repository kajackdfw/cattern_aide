# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**cattern_aide** (internally `research_ai_ide`) is a Rust-based terminal UI application for managing multiple AI-assisted projects. It uses ratatui for TUI rendering and integrates Claude via both direct Anthropic API (SSE streaming) and subprocess (`claude -p`) modes.

## Build & Run Commands

```bash
cargo build          # compile
cargo run            # launch terminal application
cargo test           # run tests
cargo test <name>    # run a single test by name
cargo clippy         # lint
```

Logs are written to `logs/research_ai_ide.log` — never to stdout (would corrupt TUI).

## Architecture

### State → Agent → UI data flow

```
tokio task → mpsc channel → AgentManager::drain_into → TextContainer → Ratatui Paragraph
```

The event loop pattern (in `src/main.rs`):
1. `agent_manager.drain_into(&mut app.projects)` — flush channels into state (non-blocking `try_recv`)
2. `terminal.draw(|f| ui::draw(f, &app))` — pure render pass (read-only `&App`)
3. `tokio::select!` on 50ms tick + crossterm events — mutation only here via `handle_key` and `drain_into`

**Draw is always pure.** No mutation happens inside `ui::draw`.

### Key structs

- **`TextContainer`** (`src/state/container.rs`): Ring-buffer text storage with `push_line` / `push_partial` and scroll offset. Max 10,000 lines.
- **`AgentMessage`**: Either `Chunk { project_name, tab_kind, text, is_newline }` or `StateChange { project_name, tab_kind, new_state }`. Routing key is `(project_name, tab_kind)`.
- **`AgentState`**: `Idle | Running | Waiting | Error` — drives LED color (Gray / Blue / Yellow / Red).
- **`AgentKind`**: `AiPrompt | TargetCode | Named(String) | Process(String) | HttpProxy { port, target }`

### Agent implementations

- **`api_agent.rs`**: Anthropic SSE — POSTs to `api.anthropic.com/v1/messages` with `"stream": true`, parses `content_block_delta`, sends `Chunk` messages per token.
- **`openai_api_agent.rs`**: OpenAI-compatible SSE — used for opencode. Same structure as `api_agent.rs` but parses `choices[0].delta.content`, uses `Authorization: Bearer` header, and stops on `finish_reason == "stop"`. Endpoint is configurable (`api_base` in config, default `http://localhost:4096/v1`).
- **`subprocess_agent.rs`**: Generic subprocess — accepts `(command, args)` rather than hardcoding `claude`. Spawns via `tokio::process::Command`, reads stdout/stderr line-by-line. Caller passes `["claude", "-p", prompt]` or `["opencode", "run", "--print", prompt]` depending on provider. Stderr lines prefixed `[stderr]`. If line-buffering is an issue, wrap with `stdbuf -oL`.
- **`http_proxy_agent.rs`**: Binds an `axum` listener; forwards requests via `reqwest` to the target; logs request/response as `Chunk` messages. Uses `oneshot` channel for `Ctrl-K` cancellation (same pattern as Process tabs).
- **Process tabs**: Long-running processes stay in `Error` state after unexpected exit — no auto-respawn. Re-run is user-triggered.

### Sidebar rotated text (PNG-based)

Project tab labels are 20×140 px PNGs generated at startup using `ab_glyph` + `imageproc`, then rotated 90° CCW to 140×20 px. Rendered via `ratatui-image` with automatic protocol detection (Kitty → Sixel → iTerm2 → half-block Unicode fallback). Font asset `assets/Inconsolata-Regular.ttf` is embedded via `include_bytes!`.

### Provider selection

Each project in `config.toml` declares `provider = "anthropic"` (default) or `provider = "opencode"`. `AgentKind` and `AgentMessage` are provider-agnostic — the provider is resolved at spawn time in `AgentManager`. OpenCode config block:

```toml
[provider.opencode]
api_base = "http://localhost:4096/v1"
api_key  = ""
model    = "anthropic/claude-sonnet-4-5"
```

## Tech Stack

| Crate | Purpose |
|---|---|
| `ratatui 0.27` + `crossterm 0.27` | TUI framework |
| `tokio` (full) | Async runtime |
| `reqwest 0.12` (json + stream) | Anthropic API HTTP |
| `axum 0.7` | HTTP proxy tab listener |
| `serde` / `serde_json` | Config + SSE parsing |
| `anyhow` / `thiserror` | Error handling |
| `tracing` + `tracing-appender` | File-only logging |
| `ratatui-image 2` | Inline image rendering |
| `image 0.25`, `ab_glyph 0.2`, `imageproc 0.25` | Sidebar label PNG generation |

## Keyboard Bindings

| Key | Action |
|---|---|
| `j/k` or `↑/↓` | Switch project in sidebar |
| `h/l` or `←/→` | Switch horizontal tab |
| `PgUp/PgDn` | Scroll tab content |
| `Enter` (AI Prompt tab) | Submit prompt → spawn agent |
| `Ctrl-K` | Kill process on current tab |
| `Tab` | Cycle tabs forward |
| `q` / `Ctrl-C` | Quit |

## Implementation Build Order

Follow this sequence to avoid circular dependencies:
1. `Cargo.toml`
2. `src/state/` (pure structs, no async) — include `Provider` enum here
3. `src/config.rs` — include `provider` per-project + `[provider.opencode]` block
4. `src/ui/led.rs` + `sidebar.rs`
5. `src/ui/main_area.rs` + `tab_content.rs` + `mod.rs`
6. `src/app.rs` + `src/main.rs` (render-only first)
7. `src/agent/mod.rs` (AgentManager + channels)
8. `src/agent/api_agent.rs` (Anthropic SSE)
9. `src/agent/openai_api_agent.rs` (OpenAI-compat SSE for opencode)
10. `src/agent/subprocess_agent.rs` (generic command/args)
11. `src/agent/http_proxy_agent.rs`
12. Wire `handle_key(Enter)` → `agent_manager.spawn_*` (dispatch on `Provider`)
