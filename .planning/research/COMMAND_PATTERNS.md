# Async Subprocess Patterns for Rust TUI Applications

**Project:** rslph
**Researched:** 2026-01-17
**Focus:** Async subprocess execution with streaming output for TUI integration
**Confidence:** HIGH (verified with official tokio, tokio-stream, tokio-process-stream documentation)

---

## Executive Summary

This document analyzes async subprocess patterns for Rust TUI applications, specifically for piloting Claude CLI as a subprocess with real-time streaming output. The analysis is based on research of authoritative Rust async ecosystem documentation.

**Key Finding:** The "streams over channels" pattern recommended in PROJECT.md refers to using the `Stream` trait abstraction (from `tokio-stream` or `futures`) instead of raw `mpsc` channels for subprocess output. This provides:

1. **Composability** - Stream combinators (`merge`, `map`, `filter`) enable clean data flow
2. **Type Safety** - Stream items have explicit types (`Item::Stdout` vs `Item::Stderr`)
3. **Backpressure** - Built-in flow control without manual channel tuning
4. **Integration** - Seamless use with `tokio::select!` for event multiplexing

---

## Key Types and Abstractions

### Core Subprocess Types (tokio::process)

| Type | Purpose | Key Methods |
|------|---------|-------------|
| `Command` | Configure and spawn async subprocess | `spawn()`, `stdout()`, `stderr()`, `kill_on_drop()` |
| `Child` | Handle to running subprocess | `stdout.take()`, `stderr.take()`, `wait()`, `kill()` |
| `ChildStdout` | Async readable handle for stdout | Implements `AsyncRead` |
| `ChildStderr` | Async readable handle for stderr | Implements `AsyncRead` |

### Stream Wrapper Types (tokio-stream)

| Type | Purpose | Item Type |
|------|---------|-----------|
| `LinesStream<R>` | Wraps `Lines<R>` as a `Stream` | `Result<String, io::Error>` |

### Process Stream Types (tokio-process-stream / process-stream)

| Type | Purpose | Item Type |
|------|---------|-----------|
| `ProcessLineStream` | Stream lines from subprocess stdout/stderr | `Item::Stdout(String)` / `Item::Stderr(String)` / `Item::Done(ExitStatus)` |
| `ProcessChunkStream` | Stream raw byte chunks | `Item<Bytes>` |

---

## Pattern 1: Streams-First Approach (Recommended)

The streams-first approach treats subprocess output as a `Stream` that can be composed with other async data sources using stream combinators.

### Why Streams Over Channels?

| Aspect | Streams | Channels |
|--------|---------|----------|
| **Composability** | Rich combinator methods (`merge`, `map`, `filter`, `timeout`) | Manual forwarding logic |
| **Type Safety** | Item type encodes source (stdout vs stderr) | Requires enum wrapping |
| **Backpressure** | Automatic via poll-based model | Requires bounded channel tuning |
| **Integration** | Works directly with `tokio::select!` | Works with `tokio::select!` |
| **Complexity** | Single abstraction for data flow | Requires task + channel + consumer |
| **Cancellation** | Drop stream = done | Requires explicit channel close |

### Implementation with tokio-process-stream

```rust
use tokio::process::Command;
use tokio_process_stream::{ProcessLineStream, Item};
use tokio_stream::StreamExt;

async fn stream_claude_output() -> Result<()> {
    // Configure subprocess
    let mut cmd = Command::new("claude");
    cmd.args(&["--print", "prompt.md"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    // Convert to stream - this spawns the process
    let mut stream = ProcessLineStream::try_from(cmd)?;

    // Consume stream
    while let Some(item) = stream.next().await {
        match item {
            Item::Stdout(line) => {
                // Send to TUI
                ui_tx.send(UiEvent::Output { source: Source::Stdout, line }).await?;
            }
            Item::Stderr(line) => {
                // Claude often outputs status to stderr
                ui_tx.send(UiEvent::Output { source: Source::Stderr, line }).await?;
            }
            Item::Done(status) => {
                ui_tx.send(UiEvent::ProcessExited { code: status.code() }).await?;
                break;
            }
        }
    }

    Ok(())
}
```

### Implementation with process-stream (Alternative)

```rust
use process_stream::{Process, ProcessExt, StreamExt};

async fn stream_with_process_stream() -> Result<()> {
    let mut process: Process = "claude".into();
    process.args(&["--print", "prompt.md"]);

    let mut stream = process.spawn_and_stream()?;

    while let Some(output) = stream.next().await {
        println!("{output}");  // Output includes source prefix
    }

    Ok(())
}
```

---

## Pattern 2: Manual Stream Construction with LinesStream

For finer control, construct streams manually from tokio's async I/O primitives.

### Implementation

```rust
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_stream::wrappers::LinesStream;
use tokio_stream::StreamExt;

#[derive(Debug, Clone)]
enum OutputLine {
    Stdout(String),
    Stderr(String),
    Exit(Option<i32>),
}

async fn manual_stream_subprocess() -> Result<()> {
    let mut child = Command::new("claude")
        .args(&["--print", "prompt.md"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    // Take ownership of stdout/stderr
    let stdout = child.stdout.take().expect("stdout not piped");
    let stderr = child.stderr.take().expect("stderr not piped");

    // Create line streams
    let stdout_lines = LinesStream::new(BufReader::new(stdout).lines())
        .map(|r| r.map(OutputLine::Stdout));

    let stderr_lines = LinesStream::new(BufReader::new(stderr).lines())
        .map(|r| r.map(OutputLine::Stderr));

    // Merge streams - interleaves stdout and stderr fairly
    let mut merged = stdout_lines.merge(stderr_lines);

    // Consume merged stream
    while let Some(result) = merged.next().await {
        let line = result?;
        handle_output_line(line).await;
    }

    // Wait for process exit
    let status = child.wait().await?;
    handle_output_line(OutputLine::Exit(status.code())).await;

    Ok(())
}
```

### Key Insight: merge() vs select!

The `merge()` combinator from `StreamExt` provides fair interleaving of two streams with the same item type. This is cleaner than using `tokio::select!` for combining two output streams:

```rust
// Using merge (cleaner)
let merged = stdout_stream.merge(stderr_stream);
while let Some(item) = merged.next().await { ... }

// Using select! (more verbose)
loop {
    tokio::select! {
        Some(line) = stdout_stream.next() => { ... }
        Some(line) = stderr_stream.next() => { ... }
        else => break,
    }
}
```

---

## Pattern 3: Hybrid Stream + Channel for TUI Integration

For TUI applications, a hybrid approach works well: streams for subprocess I/O, channels for cross-component communication.

### Architecture

```
                     Stream-based I/O
                           |
                           v
+----------------+    +----------+    +---------+
|   Subprocess   | -> |  Stream  | -> | Channel | -> Event Loop -> TUI
|   (claude)     |    | (merged) |    |  (mpsc) |
+----------------+    +----------+    +---------+
                                           ^
                                           |
                      +------------+       |
                      |  Terminal  | ------+
                      |  Events    |
                      +------------+
```

### Implementation

```rust
use tokio::sync::mpsc;
use tokio::process::Command;
use tokio_process_stream::{ProcessLineStream, Item};
use tokio_stream::StreamExt;
use crossterm::event::{Event, EventStream};

// Unified event type for the application
enum AppEvent {
    // Subprocess events
    SubprocessOutput { source: OutputSource, line: String },
    SubprocessExited { code: Option<i32> },
    SubprocessError(String),

    // Terminal events
    Key(KeyEvent),
    Resize(u16, u16),

    // Control events
    Tick,
    Render,
}

async fn run_subprocess_task(
    cmd: Command,
    tx: mpsc::Sender<AppEvent>,
) {
    match ProcessLineStream::try_from(cmd) {
        Ok(mut stream) => {
            while let Some(item) = stream.next().await {
                let event = match item {
                    Item::Stdout(line) => AppEvent::SubprocessOutput {
                        source: OutputSource::Stdout,
                        line
                    },
                    Item::Stderr(line) => AppEvent::SubprocessOutput {
                        source: OutputSource::Stderr,
                        line
                    },
                    Item::Done(status) => AppEvent::SubprocessExited {
                        code: status.code()
                    },
                };

                if tx.send(event).await.is_err() {
                    break;  // Receiver dropped, stop processing
                }
            }
        }
        Err(e) => {
            let _ = tx.send(AppEvent::SubprocessError(e.to_string())).await;
        }
    }
}

async fn run_event_loop(
    mut rx: mpsc::Receiver<AppEvent>,
    mut terminal: Terminal<impl Backend>,
    mut app_state: AppState,
) -> Result<()> {
    let mut terminal_events = EventStream::new();
    let mut render_interval = tokio::time::interval(Duration::from_millis(16));

    loop {
        tokio::select! {
            // Channel events (subprocess output, subprocess errors)
            Some(event) = rx.recv() => {
                app_state.handle_event(event);
            }

            // Terminal events (keyboard, mouse)
            Some(Ok(event)) = terminal_events.next() => {
                match event {
                    Event::Key(key) => app_state.handle_event(AppEvent::Key(key)),
                    Event::Resize(w, h) => app_state.handle_event(AppEvent::Resize(w, h)),
                    _ => {}
                }
            }

            // Render tick
            _ = render_interval.tick() => {
                terminal.draw(|f| app_state.render(f))?;
            }
        }

        if app_state.should_quit() {
            break;
        }
    }

    Ok(())
}
```

---

## Pattern 4: Error Handling

### Stream Error Handling

Streams naturally propagate errors through the `Result` item type:

```rust
use tokio_stream::StreamExt;

let mut stream = LinesStream::new(reader.lines());

while let Some(result) = stream.next().await {
    match result {
        Ok(line) => process_line(line),
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
            // Process terminated unexpectedly
            break;
        }
        Err(e) => {
            tracing::error!("Stream error: {}", e);
            // Continue or break based on error severity
        }
    }
}
```

### Subprocess Lifecycle Errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum SubprocessError {
    #[error("Failed to spawn subprocess: {0}")]
    SpawnFailed(#[from] std::io::Error),

    #[error("Subprocess exited with code {0}")]
    NonZeroExit(i32),

    #[error("Subprocess was killed by signal")]
    Killed,

    #[error("Subprocess output stream error: {0}")]
    StreamError(std::io::Error),
}

async fn run_subprocess(cmd: Command) -> Result<(), SubprocessError> {
    let mut stream = ProcessLineStream::try_from(cmd)
        .map_err(SubprocessError::SpawnFailed)?;

    while let Some(item) = stream.next().await {
        match item {
            Item::Done(status) => {
                if let Some(code) = status.code() {
                    if code != 0 {
                        return Err(SubprocessError::NonZeroExit(code));
                    }
                } else {
                    return Err(SubprocessError::Killed);
                }
            }
            _ => {}
        }
    }

    Ok(())
}
```

---

## Pattern 5: Graceful Shutdown and Cancellation

### Using kill_on_drop

The simplest approach - process is killed when the `Child` handle drops:

```rust
let child = Command::new("claude")
    .kill_on_drop(true)  // Kill when Child drops
    .spawn()?;

// When child goes out of scope or is dropped, process is killed
```

### Manual Kill with Timeout

For cleaner shutdown with timeout:

```rust
use tokio::time::{timeout, Duration};

async fn shutdown_subprocess(mut child: Child) -> Result<()> {
    // Try graceful shutdown first (SIGTERM)
    child.start_kill()?;

    // Wait up to 5 seconds for exit
    match timeout(Duration::from_secs(5), child.wait()).await {
        Ok(Ok(status)) => {
            tracing::info!("Subprocess exited gracefully: {:?}", status);
            Ok(())
        }
        Ok(Err(e)) => {
            tracing::error!("Error waiting for subprocess: {}", e);
            Err(e.into())
        }
        Err(_) => {
            tracing::warn!("Subprocess didn't exit in time, force killing");
            child.kill().await?;
            Ok(())
        }
    }
}
```

### Cancellation with CancellationToken

For coordinated shutdown across tasks:

```rust
use tokio_util::sync::CancellationToken;

async fn run_with_cancellation(
    cmd: Command,
    cancel: CancellationToken,
    tx: mpsc::Sender<AppEvent>,
) {
    let mut stream = ProcessLineStream::try_from(cmd).unwrap();

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                // Cancellation requested
                // Stream drop will trigger kill_on_drop
                break;
            }
            Some(item) = stream.next() => {
                // Process item...
                if tx.send(item.into()).await.is_err() {
                    break;
                }
            }
            else => break,  // Stream exhausted
        }
    }
}
```

---

## TUI Integration Patterns

### Status Bar with Subprocess Info

```rust
struct StatusBarState {
    iteration: u32,
    max_iterations: u32,
    task_index: usize,
    total_tasks: usize,
    model: String,
    subprocess_status: SubprocessStatus,
}

enum SubprocessStatus {
    NotStarted,
    Running { pid: u32 },
    Exited { code: Option<i32> },
    Error(String),
}

fn render_status_bar(area: Rect, buf: &mut Buffer, state: &StatusBarState) {
    let status_text = match &state.subprocess_status {
        SubprocessStatus::NotStarted => "Idle".to_string(),
        SubprocessStatus::Running { pid } => format!("Running (PID {})", pid),
        SubprocessStatus::Exited { code: Some(0) } => "Completed".to_string(),
        SubprocessStatus::Exited { code: Some(c) } => format!("Failed (code {})", c),
        SubprocessStatus::Exited { code: None } => "Killed".to_string(),
        SubprocessStatus::Error(e) => format!("Error: {}", e),
    };

    let text = format!(
        "Iteration {}/{} | Task {}/{} | {} | {}",
        state.iteration, state.max_iterations,
        state.task_index, state.total_tasks,
        state.model,
        status_text
    );

    Paragraph::new(text)
        .style(Style::default().bg(Color::Blue))
        .render(area, buf);
}
```

### Streaming Output Display

```rust
struct OutputBuffer {
    lines: VecDeque<OutputLine>,
    max_lines: usize,
    scroll_offset: usize,
}

impl OutputBuffer {
    fn push(&mut self, line: OutputLine) {
        self.lines.push_back(line);
        if self.lines.len() > self.max_lines {
            self.lines.pop_front();
        }
    }

    fn visible_lines(&self, viewport_height: usize) -> impl Iterator<Item = &OutputLine> {
        self.lines
            .iter()
            .skip(self.scroll_offset)
            .take(viewport_height)
    }
}

fn render_output(area: Rect, buf: &mut Buffer, buffer: &OutputBuffer) {
    let lines: Vec<Line> = buffer
        .visible_lines(area.height as usize)
        .map(|output_line| {
            let style = match output_line.source {
                OutputSource::Stdout => Style::default(),
                OutputSource::Stderr => Style::default().fg(Color::Yellow),
            };
            Line::styled(&output_line.text, style)
        })
        .collect();

    Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Output"))
        .render(area, buf);
}
```

---

## Recommended Stack for rslph

Based on this research, the recommended approach for rslph subprocess management:

### Dependencies

```toml
[dependencies]
tokio = { version = "1.49", features = ["full"] }
tokio-stream = { version = "0.1", features = ["io-util"] }
tokio-util = { version = "0.7", features = ["rt"] }
tokio-process-stream = "0.4"  # OR
# process-stream = "0.3"      # Alternative

# TUI
ratatui = "0.30"
crossterm = { version = "0.29", features = ["event-stream"] }
```

### Architecture

1. **Subprocess Task**: Use `tokio-process-stream` to create a stream from the Claude CLI subprocess
2. **Event Channel**: `mpsc::Sender<AppEvent>` bridges subprocess stream to main event loop
3. **Event Loop**: `tokio::select!` multiplexes subprocess events, terminal events, and render ticks
4. **Output Buffer**: Ring buffer (`VecDeque`) with configurable size for recent output
5. **Error Handling**: `thiserror` for subprocess errors, `color-eyre` for user-facing reports

### Subprocess Manager Module

```rust
// subprocess/manager.rs
pub struct SubprocessManager {
    cancel_token: CancellationToken,
    task_handle: Option<JoinHandle<()>>,
}

impl SubprocessManager {
    pub fn spawn(
        config: &SubprocessConfig,
        event_tx: mpsc::Sender<AppEvent>,
    ) -> Self {
        let cancel_token = CancellationToken::new();
        let token = cancel_token.clone();

        let mut cmd = Command::new(&config.claude_command);
        cmd.args(&config.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let handle = tokio::spawn(async move {
            run_subprocess_stream(cmd, token, event_tx).await;
        });

        Self {
            cancel_token,
            task_handle: Some(handle),
        }
    }

    pub async fn stop(&mut self) {
        self.cancel_token.cancel();
        if let Some(handle) = self.task_handle.take() {
            let _ = handle.await;
        }
    }
}
```

---

## Pitfalls This Pattern Avoids

### 1. Blocking the Event Loop

**Problem:** Using `std::process::Command::output()` blocks the async runtime.

**Solution:** Always use `tokio::process::Command` with async spawning.

### 2. Unbounded Memory Growth

**Problem:** Storing all subprocess output in a `Vec<String>` can exhaust memory.

**Solution:** Use bounded ring buffer (`VecDeque` with `max_lines`) or drop old output.

### 3. Deadlock on Full Channels

**Problem:** Unbounded channels grow forever; bounded channels can deadlock if not drained.

**Solution:** Use stream-based approach where backpressure is natural, or ensure channel is always drained by event loop.

### 4. Zombie Processes

**Problem:** Dropping `Child` without waiting leaves zombie processes.

**Solution:** Always use `kill_on_drop(true)` or explicitly call `child.wait()`.

### 5. Lost stderr Output

**Problem:** Only capturing stdout misses Claude CLI status messages.

**Solution:** Capture both stdout and stderr, merge streams with typed items.

### 6. Stdout/Stderr Race Condition

**Problem:** Reading stdout and stderr separately can cause interleaving issues.

**Solution:** Use `merge()` combinator for fair interleaving, or use `tokio-process-stream` which handles this.

### 7. Hard Kills on User Interrupt

**Problem:** SIGKILL doesn't allow Claude CLI to clean up.

**Solution:** Use graceful shutdown with timeout (SIGTERM first, then SIGKILL after delay).

---

## Sources

### Official Documentation (HIGH confidence)
- [tokio::process::Child](https://docs.rs/tokio/latest/tokio/process/struct.Child.html) - Async subprocess management
- [tokio::process module](https://tikv.github.io/doc/tokio/process/index.html) - Process spawning overview
- [tokio::sync::mpsc](https://docs.rs/tokio/latest/tokio/sync/mpsc/) - Async channel documentation
- [tokio::select! tutorial](https://tokio.rs/tokio/tutorial/select) - Event multiplexing patterns
- [tokio-stream::wrappers::LinesStream](https://docs.rs/tokio-stream/latest/tokio_stream/wrappers/struct.LinesStream.html) - Lines-to-stream wrapper
- [tokio-stream::StreamExt](https://docs.rs/tokio-stream/latest/tokio_stream/trait.StreamExt.html) - Stream combinators including merge

### Library Documentation (HIGH confidence)
- [tokio-process-stream on lib.rs](https://lib.rs/crates/tokio-process-stream) - Subprocess-to-stream wrapper
- [process-stream on GitHub](https://github.com/kkharji/process-stream) - Alternative subprocess streaming

### Ratatui Patterns (HIGH confidence)
- [Ratatui Async Event Stream Tutorial](https://ratatui.rs/tutorials/counter-async-app/async-event-stream/) - tokio::select! with TUI
- [Ratatui Full Async Events Tutorial](https://ratatui.rs/tutorials/counter-async-app/full-async-events/) - Tick and render events
- [Ratatui Async Template](https://github.com/ratatui/async-template) - Component architecture

### Note on Reference Implementation

The PROJECT.md references `/Users/vmakaev/fbsource/fbcode/linttool/crates/command` for async subprocess patterns. This path was inaccessible during research (permission denied - likely a protected corporate codebase). The patterns documented here are based on authoritative public Rust ecosystem documentation and represent the current best practices for async subprocess handling in TUI applications.
