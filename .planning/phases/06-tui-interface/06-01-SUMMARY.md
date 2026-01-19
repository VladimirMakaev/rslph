---
phase: 06-tui-interface
plan: 01
subsystem: tui
tags: [ratatui, crossterm, async-events, terminal-ui]

dependency-graph:
  requires: []
  provides:
    - TUI module foundation with App state, EventHandler, terminal setup
    - Async event merging via tokio::select!
    - Panic-safe terminal initialization
  affects:
    - 06-02 (UI layout and rendering)
    - 06-03 (Build loop integration)
    - 06-04 (Pause and interaction)

tech-stack:
  added:
    - ratatui@0.30 (TUI framework)
    - crossterm@0.29 (terminal backend with event-stream)
    - futures@0.3 (StreamExt for EventStream)
  patterns:
    - TEA (The Elm Architecture) for state management
    - Async event loop with tokio::select!
    - Panic hook chaining for terminal safety

file-tracking:
  key-files:
    created:
      - src/tui/mod.rs
      - src/tui/terminal.rs
      - src/tui/app.rs
      - src/tui/event.rs
    modified:
      - Cargo.toml
      - src/lib.rs

decisions:
  - id: TUI-STDERR-BACKEND
    choice: Use stderr for terminal backend
    why: Keeps stdout available for non-TUI output if needed
  - id: TUI-PANIC-HOOK-CHAIN
    choice: Chain panic hooks instead of replacing
    why: Preserves existing panic behavior (e.g., color-eyre) while ensuring terminal restore
  - id: TUI-UNBOUNDED-CHANNEL
    choice: Use unbounded channels for event handling
    why: Avoids backpressure issues with fast Claude output; memory use is bounded by session length

metrics:
  duration: 2m 27s
  completed: 2026-01-19
---

# Phase 6 Plan 1: TUI Module Foundation Summary

**One-liner:** TUI foundation with ratatui/crossterm, App state struct following TEA, async EventHandler merging keyboard/subprocess/timer events, and panic-safe terminal setup.

## What Was Built

### 1. TUI Module Structure
Created `src/tui/` module with four files:
- `mod.rs` - Module exports and documentation
- `terminal.rs` - Terminal initialization and restoration with panic safety
- `app.rs` - Application state struct (Model) and event enum (Message)
- `event.rs` - Async event handler for event merging

### 2. Dependencies Added
```toml
ratatui = "0.30"
crossterm = { version = "0.29", features = ["event-stream"] }
futures = "0.3"
```

### 3. Terminal Setup (`terminal.rs`)

**`init_terminal()`:**
- Installs panic hook BEFORE entering raw mode (chains with existing hook)
- Enables raw mode (disables line buffering/echo)
- Enters alternate screen (preserves original terminal content)
- Enables mouse capture (for scroll events)
- Returns `Terminal<CrosstermBackend<Stderr>>`

**`restore_terminal()`:**
- Disables raw mode
- Leaves alternate screen
- Disables mouse capture
- Called by panic hook and on normal exit

### 4. App State (`app.rs`)

```rust
pub struct App {
    // Status bar state
    pub current_iteration: u32,
    pub max_iterations: u32,
    pub current_task: u32,
    pub total_tasks: u32,
    pub context_usage: f64,  // 0.0 to 1.0
    pub model_name: String,
    pub project_name: String,
    pub log_path: Option<PathBuf>,

    // Output view state
    pub messages: Vec<Message>,
    pub scroll_offset: u16,

    // Navigation state
    pub viewing_iteration: u32,
    pub is_paused: bool,
    pub should_quit: bool,
}
```

**`AppEvent` enum:**
- Navigation: ScrollUp, ScrollDown, PrevIteration, NextIteration
- Control: TogglePause, Quit
- Subprocess: ClaudeOutput(String), ContextUsage(f64), IterationComplete
- Timer: Render

### 5. Event Handler (`event.rs`)

Uses `tokio::select!` to merge three event sources:
1. **Keyboard/mouse** via crossterm `EventStream`
2. **Subprocess events** via `mpsc::UnboundedReceiver<SubprocessEvent>`
3. **Render ticks** via `tokio::time::interval` (configurable FPS, default 30)

Key mappings:
- `j`/`Down` - ScrollDown
- `k`/`Up` - ScrollUp
- `{` - PrevIteration
- `}` - NextIteration
- `p` - TogglePause
- `q`/`Esc`/`Ctrl+C` - Quit
- Mouse scroll - ScrollUp/ScrollDown

## Test Results

```
running 9 tests
test tui::app::tests::test_app_update_quit ... ok
test tui::app::tests::test_app_event_variants ... ok
test tui::app::tests::test_app_update_context_usage ... ok
test tui::app::tests::test_app_update_scroll ... ok
test tui::app::tests::test_message_new ... ok
test tui::app::tests::test_app_update_toggle_pause ... ok
test tui::app::tests::test_app_new ... ok
test tui::app::tests::test_app_update_claude_output ... ok
test tui::event::tests::test_subprocess_event_into_app_event ... ok

test result: ok. 9 passed; 0 failed
```

## Deviations from Plan

None - plan executed exactly as written. The three tasks were implemented cohesively in one commit since they are interdependent (mod.rs exports from all files).

## Commits

| Hash | Type | Description |
|------|------|-------------|
| 17534ee | feat | add TUI module with dependencies and core structure |

## Next Phase Readiness

**Ready for 06-02 (UI Layout and Rendering):**
- App state struct has all required fields for rendering
- Terminal setup ready for ratatui Frame rendering
- EventHandler ready to drive main loop

**Dependencies satisfied:**
- ratatui 0.30 installed and working
- crossterm 0.29 with event-stream feature working
- futures 0.3 for StreamExt on EventStream

**No blockers identified.**
