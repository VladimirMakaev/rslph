# Phase 6: TUI Interface - Research

**Researched:** 2026-01-19
**Domain:** Rust Terminal User Interface with async subprocess streaming
**Confidence:** HIGH

## Summary

This research investigates implementing a rich terminal UI for rslph that displays status information, live Claude output streaming, and collapsible conversation threads. The existing codebase uses tokio for async operations and already streams Claude CLI output line-by-line via `ClaudeRunner`.

The standard approach is to use **ratatui** (v0.30.0) as the TUI framework with **crossterm** (v0.29.0) as the terminal backend. Ratatui provides immediate-mode rendering with built-in widgets for layouts, progress bars (Gauge), text display (Paragraph), and lists. Crossterm handles raw mode, alternate screen, keyboard/mouse events, and provides an async `EventStream` that integrates seamlessly with tokio.

The key architecture pattern is an **async event loop** using `tokio::select!` to multiplex between: (1) crossterm keyboard/mouse events, (2) subprocess output from ClaudeRunner, and (3) periodic render ticks. State is managed in an `App` struct that accumulates streaming output and tracks UI state (scroll position, current iteration, pause state).

**Primary recommendation:** Use ratatui with crossterm backend, The Elm Architecture (TEA) for state management, and `tokio::select!` to merge event sources into a unified async loop.

## Standard Stack

The established libraries for Rust TUI development:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.30.0 | TUI framework | De facto standard, successor to tui-rs, 1.4M downloads/month |
| crossterm | 0.29.0 | Terminal backend | Cross-platform, async-compatible, most popular backend |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| futures | 0.3.x | Stream combinators | Required for crossterm EventStream |
| tokio-util | 0.7.x | CancellationToken | Already in project, used for graceful shutdown |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| crossterm | termion | termion is Unix-only; crossterm is cross-platform |
| crossterm | termwiz | termwiz has fewer examples, less ecosystem support |
| ratatui | cursive | cursive is higher-level but less flexible for custom layouts |

**Installation:**
```bash
cargo add ratatui crossterm --features crossterm/event-stream
cargo add futures
```

**Cargo.toml:**
```toml
ratatui = "0.30"
crossterm = { version = "0.29", features = ["event-stream"] }
futures = "0.3"
```

## Architecture Patterns

### Recommended Project Structure
```
src/
├── tui/
│   ├── mod.rs           # TUI module exports
│   ├── app.rs           # App state struct (Model in TEA)
│   ├── event.rs         # Event enum and EventHandler
│   ├── ui.rs            # Render functions (View in TEA)
│   ├── widgets/         # Custom widget implementations
│   │   ├── mod.rs
│   │   ├── status_bar.rs    # Header with status info
│   │   ├── progress_bar.rs  # Context usage gauge
│   │   ├── output_view.rs   # Live streaming output
│   │   └── thread_view.rs   # Collapsible message threads
│   └── keybindings.rs   # Keyboard handling (Update in TEA)
├── build/
│   ├── iteration.rs     # Existing - needs TUI integration
│   └── ...
└── subprocess/
    └── runner.rs        # Existing ClaudeRunner
```

### Pattern 1: The Elm Architecture (TEA)
**What:** Divide app into Model (state), Update (event handling), View (rendering)
**When to use:** All ratatui applications benefit from this separation
**Example:**
```rust
// Source: https://ratatui.rs/concepts/application-patterns/the-elm-architecture/

// Model - Application state
pub struct App {
    // Status bar state
    current_iteration: u32,
    max_iterations: u32,
    current_task: u32,
    total_tasks: u32,
    context_usage: f64,  // 0.0 to 1.0
    model_name: String,
    project_name: String,

    // Output view state
    messages: Vec<Message>,
    scroll_offset: u16,

    // Navigation state
    viewing_iteration: u32,  // Which iteration we're viewing
    is_paused: bool,
    should_quit: bool,
}

// Message enum for events
pub enum AppEvent {
    // Keyboard events
    ScrollUp,
    ScrollDown,
    PrevIteration,
    NextIteration,
    TogglePause,
    Quit,

    // Subprocess events
    ClaudeOutput(String),
    ClaudeUsage { input: u64, output: u64 },
    IterationComplete { tasks_done: u32 },

    // Tick events
    Render,
}

// Update - process events and modify state
fn update(app: &mut App, event: AppEvent) {
    match event {
        AppEvent::ScrollUp => app.scroll_offset = app.scroll_offset.saturating_sub(1),
        AppEvent::ScrollDown => app.scroll_offset = app.scroll_offset.saturating_add(1),
        AppEvent::TogglePause => app.is_paused = !app.is_paused,
        AppEvent::ClaudeOutput(line) => app.messages.push(Message::new(line)),
        AppEvent::Quit => app.should_quit = true,
        // ... etc
    }
}

// View - render state to terminal
fn view(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(2),  // Header (2 lines per CONTEXT.md)
        Constraint::Fill(1),    // Main output area
        Constraint::Length(1),  // Footer with key hints
    ]).areas(frame.area());

    render_header(frame, chunks[0], app);
    render_output(frame, chunks[1], app);
    render_footer(frame, chunks[2], app);
}
```

### Pattern 2: Async Event Loop with tokio::select!
**What:** Multiplex multiple async sources into unified event stream
**When to use:** When combining keyboard input, subprocess output, and render ticks
**Example:**
```rust
// Source: https://ratatui.rs/tutorials/counter-async-app/full-async-events/

use crossterm::event::{Event as CrosstermEvent, EventStream, KeyCode};
use futures::StreamExt;
use tokio::sync::mpsc;

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<AppEvent>,
}

impl EventHandler {
    pub fn new(
        mut subprocess_rx: mpsc::UnboundedReceiver<SubprocessEvent>,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            let mut event_stream = EventStream::new();
            let mut render_interval = tokio::time::interval(Duration::from_millis(33)); // ~30 FPS

            loop {
                tokio::select! {
                    // Keyboard/mouse events from crossterm
                    maybe_event = event_stream.next() => {
                        if let Some(Ok(event)) = maybe_event {
                            if let Some(app_event) = convert_crossterm_event(event) {
                                let _ = tx.send(app_event);
                            }
                        }
                    }

                    // Subprocess output from ClaudeRunner
                    maybe_output = subprocess_rx.recv() => {
                        if let Some(output) = maybe_output {
                            let _ = tx.send(AppEvent::from(output));
                        }
                    }

                    // Render tick
                    _ = render_interval.tick() => {
                        let _ = tx.send(AppEvent::Render);
                    }
                }
            }
        });

        Self { rx }
    }

    pub async fn next(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }
}
```

### Pattern 3: Terminal Setup with Panic Safety
**What:** Properly initialize raw mode/alternate screen and restore on panic
**When to use:** Always - terminal corruption on panic is critical to avoid
**Example:**
```rust
// Source: https://ratatui.rs/recipes/apps/panic-hooks/

use std::panic;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{EnableMouseCapture, DisableMouseCapture},
};
use ratatui::prelude::*;

pub fn init_terminal() -> std::io::Result<Terminal<CrosstermBackend<std::io::Stderr>>> {
    // Install panic hook BEFORE entering raw mode
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Restore terminal before showing panic
        let _ = restore_terminal();
        original_hook(panic_info);
    }));

    // Enter raw mode and alternate screen
    enable_raw_mode()?;
    let mut stderr = std::io::stderr();
    execute!(stderr, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stderr);
    Terminal::new(backend)
}

pub fn restore_terminal() -> std::io::Result<()> {
    disable_raw_mode()?;
    execute!(
        std::io::stderr(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}
```

### Anti-Patterns to Avoid
- **Blocking in event loop:** Never use `std::thread::sleep` or blocking I/O in the async event loop
- **Rendering without tick rate control:** Don't render on every event - use fixed frame rate (30-60 FPS)
- **Direct terminal writes:** Always go through ratatui's Frame - direct writes corrupt the buffer
- **Forgetting terminal restore:** Always setup panic hooks before entering raw mode

## Don't Hand-Roll

Problems that have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Progress bar | Custom ASCII art | `ratatui::widgets::Gauge` | Handles ratio/percent, colors, Unicode |
| Text scrolling | Manual offset math | `Paragraph::scroll((y, x))` | Handles wrapping, viewport correctly |
| List selection | Manual index tracking | `ListState` with `select_*` methods | Handles scroll offset, wrapping |
| Keyboard parsing | Match on raw bytes | `crossterm::event::KeyEvent` | Cross-platform, modifier keys handled |
| Async events | Manual poll loop | `crossterm::event::EventStream` | Integrates with tokio, proper backpressure |
| Layout math | Manual Rect calculations | `Layout::vertical/horizontal` | Constraint system, caching, proper splitting |
| Terminal restore | Manual cleanup code | `ratatui::init()` + panic hook | Handles all edge cases, tested |

**Key insight:** Ratatui widgets handle many edge cases (Unicode width, terminal resize, color fallbacks) that are tedious and error-prone to implement correctly.

## Common Pitfalls

### Pitfall 1: Terminal Not Restored on Panic
**What goes wrong:** Panic leaves terminal in raw mode, no echo, cursor hidden
**Why it happens:** Panic unwinds stack before cleanup code runs
**How to avoid:** Install panic hook BEFORE entering raw mode (see Pattern 3)
**Warning signs:** Terminal requires `reset` command after crash

### Pitfall 2: Blocking the Event Loop
**What goes wrong:** UI freezes, keyboard unresponsive
**Why it happens:** Synchronous operations in the async event loop
**How to avoid:** All I/O must be async, use `spawn_blocking` for CPU work
**Warning signs:** UI doesn't update while subprocess runs

### Pitfall 3: Render Performance Degradation
**What goes wrong:** High CPU usage, flickering, slow updates
**Why it happens:** Rendering on every event, no frame rate limit
**How to avoid:** Use fixed render interval (30-60 FPS), only render on Render event
**Warning signs:** CPU spikes during fast output, visible flicker

### Pitfall 4: Event Queue Overflow
**What goes wrong:** Events processed out of order, UI lags behind reality
**Why it happens:** Subprocess outputs faster than UI can render
**How to avoid:** Use bounded channels with backpressure, or batch updates
**Warning signs:** Scroll position jumps, output appears delayed

### Pitfall 5: Mouse Events Not Captured
**What goes wrong:** Mouse clicks/scroll do nothing
**Why it happens:** Forgot to execute `EnableMouseCapture`
**How to avoid:** Include in terminal init, pair with `DisableMouseCapture` on restore
**Warning signs:** Keyboard works but mouse doesn't

### Pitfall 6: Scroll Offset Exceeds Content
**What goes wrong:** Blank screen, panic on index out of bounds
**Why it happens:** Scroll offset not clamped to content length
**How to avoid:** Clamp scroll offset: `min(offset, content.len().saturating_sub(viewport_height))`
**Warning signs:** Blank areas appear, crash on empty content

## Code Examples

Verified patterns from official documentation:

### Layout: 2-Line Header + Body + Footer
```rust
// Source: https://docs.rs/ratatui/latest/ratatui/layout/struct.Layout.html

fn render_layout(frame: &mut Frame, app: &App) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(2),  // 2 lines per CONTEXT.md
        Constraint::Fill(1),    // Remaining space
        Constraint::Length(1),  // Single line footer
    ]).areas(frame.area());

    render_header(frame, header, app);
    render_body(frame, body, app);
    render_footer(frame, footer, app);
}
```

### Header: Status Bar Layout
```rust
// Source: https://docs.rs/ratatui/latest/ratatui/layout/struct.Layout.html

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    // Split header into two rows
    let [row1, row2] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
    ]).areas(area);

    // Row 1: "rslph" on left | "project-name (model-name)" on right
    let [left, right] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Fill(1),
    ]).areas(row1);

    frame.render_widget(
        Paragraph::new("rslph").style(Style::default().bold()),
        left
    );
    frame.render_widget(
        Paragraph::new(format!("{} ({})", app.project_name, app.model_name))
            .alignment(Alignment::Right),
        right
    );

    // Row 2: "Iter X/Y | Task X/Y | 45% [progress bar]"
    render_status_line(frame, row2, app);
}
```

### Progress Bar with Traffic Light Colors
```rust
// Source: https://ratatui.rs/examples/widgets/gauge/

fn context_bar_color(ratio: f64) -> Color {
    if ratio < 0.5 {
        Color::Green
    } else if ratio < 0.8 {
        Color::Yellow
    } else {
        Color::Red
    }
}

fn render_context_bar(frame: &mut Frame, area: Rect, ratio: f64) {
    let color = context_bar_color(ratio);
    let percent = (ratio * 100.0) as u16;

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(color))
        .ratio(ratio)
        .label(format!("{}%", percent));

    frame.render_widget(gauge, area);
}
```

### Live Output with Scrolling Paragraph
```rust
// Source: https://docs.rs/ratatui/latest/ratatui/widgets/struct.Paragraph.html

fn render_output(frame: &mut Frame, area: Rect, app: &App) {
    let content: Vec<Line> = app.messages.iter()
        .map(|msg| Line::from(msg.text.as_str()))
        .collect();

    let paragraph = Paragraph::new(content)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset, 0));  // (y, x) offset

    frame.render_widget(paragraph, area);
}
```

### Keyboard Event Handling
```rust
// Source: https://docs.rs/crossterm/0.28.1/crossterm/event/struct.KeyEvent.html

fn convert_crossterm_event(event: CrosstermEvent) -> Option<AppEvent> {
    match event {
        CrosstermEvent::Key(key) => {
            match key.code {
                KeyCode::Char('j') => Some(AppEvent::ScrollDown),
                KeyCode::Char('k') => Some(AppEvent::ScrollUp),
                KeyCode::Char('{') => Some(AppEvent::PrevIteration),
                KeyCode::Char('}') => Some(AppEvent::NextIteration),
                KeyCode::Char('p') => Some(AppEvent::TogglePause),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(AppEvent::Quit)
                }
                _ => None,
            }
        }
        CrosstermEvent::Mouse(mouse) => {
            match mouse.kind {
                MouseEventKind::ScrollUp => Some(AppEvent::ScrollUp),
                MouseEventKind::ScrollDown => Some(AppEvent::ScrollDown),
                _ => None,
            }
        }
        _ => None,
    }
}
```

### Pause Overlay
```rust
// Source: ratatui widgets documentation

fn render_pause_overlay(frame: &mut Frame, area: Rect) {
    // Center a message in the area
    let message = "PAUSED - press p to resume";
    let width = message.len() as u16 + 4;
    let height = 3;

    let popup_area = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width: width.min(area.width),
        height: height.min(area.height),
    };

    // Clear the area first
    frame.render_widget(Clear, popup_area);

    // Render the popup
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let text = Paragraph::new(message)
        .alignment(Alignment::Center)
        .block(block);

    frame.render_widget(text, popup_area);
}
```

## Context Usage Parsing

The existing `stream_json.rs` already parses usage from Claude CLI output:

```rust
// From src/subprocess/stream_json.rs - already implemented
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}
```

**Context Usage Calculation:**
- Claude Opus 4.5 context window: 200,000 tokens (per Anthropic docs)
- Percentage = `(input_tokens + output_tokens) / 200_000 * 100`
- This is an approximation - actual limits may vary by model

**Integration:** Send usage updates via the event channel when parsing stream events:
```rust
if let Some(usage) = event.usage() {
    let ratio = (usage.input_tokens + usage.output_tokens) as f64 / 200_000.0;
    tx.send(AppEvent::ContextUsage(ratio.min(1.0))).ok();
}
```

## Integration with Existing ClaudeRunner

The current `ClaudeRunner::run_to_completion` collects all output. For TUI, we need streaming:

```rust
// New method to add to ClaudeRunner
pub fn into_stream(self) -> impl Stream<Item = OutputLine> {
    futures::stream::unfold(self, |mut runner| async {
        runner.next_output().await.map(|line| (line, runner))
    })
}

// Or simpler: channel-based approach
pub async fn run_with_channel(
    &mut self,
    tx: mpsc::UnboundedSender<OutputLine>,
    cancel_token: CancellationToken,
) -> Result<(), RslphError> {
    loop {
        tokio::select! {
            biased;

            _ = cancel_token.cancelled() => {
                self.terminate_gracefully(Duration::from_secs(5)).await?;
                return Err(RslphError::Cancelled);
            }

            line = self.next_output() => {
                match line {
                    Some(l) => {
                        if tx.send(l).is_err() {
                            // Receiver dropped, stop
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }
    let _ = self.child.wait().await;
    Ok(())
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| tui-rs | ratatui | 2023 | tui-rs unmaintained, ratatui is active fork |
| termion | crossterm | 2020+ | crossterm is cross-platform, termion Unix-only |
| Thread-based events | Async EventStream | 2023 | crossterm `event-stream` feature with tokio |
| `ratatui::init()` manual | `ratatui::init()` with auto-panic-hook | v0.28+ | Simplifies setup, auto-restores on panic |

**Deprecated/outdated:**
- `tui-rs`: Unmaintained since 2023, use `ratatui` instead
- `crossterm::event::poll()` with threads: Use `EventStream` with async instead
- Manual terminal init: Use `ratatui::init()` which handles all setup

## Open Questions

Things that need validation during implementation:

1. **Thread View Implementation**
   - What we know: CONTEXT.md says "messages like Claude CLI does"
   - What's unclear: Exact formatting of Claude CLI message display
   - Recommendation: Check Claude CLI output format, match styling

2. **Context Window Size Per Model**
   - What we know: Opus 4.5 has 200K context
   - What's unclear: If we'll support other models with different limits
   - Recommendation: Make context limit configurable or model-aware

3. **Pause Behavior Details**
   - What we know: CONTEXT.md says "stops current iteration gracefully"
   - What's unclear: Exactly when/how to interrupt ClaudeRunner
   - Recommendation: Use existing CancellationToken, save progress before stopping

## Sources

### Primary (HIGH confidence)
- [ratatui docs.rs](https://docs.rs/ratatui/latest/ratatui/) - Widgets, Layout, core API
- [ratatui.rs tutorials](https://ratatui.rs/tutorials/counter-async-app/) - Async event patterns
- [crossterm docs.rs](https://docs.rs/crossterm/0.28.1/crossterm/) - Event handling, terminal control

### Secondary (MEDIUM confidence)
- [ratatui async-template](https://github.com/ratatui/async-template) - Project structure patterns
- [awesome-ratatui](https://github.com/ratatui/awesome-ratatui) - Widget ecosystem overview
- [libhunt comparison](https://www.libhunt.com/compare-ratatui-org--ratatui-vs-crossterm) - Framework comparison

### Tertiary (LOW confidence)
- WebSearch results on architecture best practices - General patterns

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - ratatui + crossterm is well-established, documented
- Architecture: HIGH - TEA pattern + async select is documented official approach
- Pitfalls: HIGH - Common issues well-documented in tutorials
- Context parsing: MEDIUM - Uses existing stream_json.rs, percentage calculation is approximation

**Research date:** 2026-01-19
**Valid until:** 2026-03-19 (ratatui ecosystem is stable, 60-day validity)
