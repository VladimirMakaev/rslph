# Architecture Research: Ralph (rslph)

**Domain:** Rust CLI/TUI autonomous coding agent
**Researched:** 2026-01-17
**Confidence:** HIGH (verified with official documentation)

## Executive Summary

Ralph requires a layered architecture separating CLI parsing, application logic, subprocess management, and TUI rendering. The async-first design is critical: the Claude subprocess streams output continuously while the TUI must remain responsive. Tokio provides the async runtime, ratatui handles terminal rendering, and channels connect the components.

The recommended pattern is a **Component Architecture with Message Passing** — similar to Elm/TEA but adapted for Rust's ownership model and the async subprocess requirement.

---

## Components

### 1. CLI Parser (Entry Point)

**Responsibility:** Parse command-line arguments, load configuration, dispatch to appropriate command handler.

**Boundaries:**
- IN: Command-line arguments
- OUT: Validated `Config` struct + `Command` enum

**Key traits:**
- Stateless parsing
- Validates early, fails fast
- Merges CLI args with TOML config (CLI wins)

**Technology:** `clap` with derive macros

```rust
#[derive(Parser)]
#[command(name = "ralph", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, global = true)]
    config: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    Plan { plan: String },
    Build { plan: PathBuf },
}
```

---

### 2. Configuration Manager

**Responsibility:** Load, validate, and merge configuration from multiple sources.

**Boundaries:**
- IN: TOML file path, CLI overrides
- OUT: Validated `AppConfig` struct

**Key behaviors:**
- Load from `~/.config/ralph/config.toml` if exists
- Override with CLI arguments
- Provide sensible defaults
- Validate paths (prompts exist, claude command works)

**Technology:** `toml` crate with serde

```rust
#[derive(Debug, Deserialize, Default)]
struct AppConfig {
    claude_command: Option<String>,          // Default: "claude"
    max_iterations: Option<u32>,             // Default: 10
    recent_threads: Option<u32>,             // Default: 3
    prompt_plan: Option<PathBuf>,            // Default: baked-in
    prompt_build: Option<PathBuf>,           // Default: baked-in
    notify_script: Option<PathBuf>,
    notify_shell: Option<String>,            // Default: "sh"
}
```

---

### 3. Application Core (State Machine)

**Responsibility:** Manage application state, coordinate between components, handle business logic.

**Boundaries:**
- IN: Actions from TUI, events from subprocess
- OUT: Commands to subprocess, state updates to TUI

**Key behaviors:**
- Maintain iteration count, task progress
- Parse and update progress file
- Decide when to start/stop iterations
- Detect RALPH_DONE marker

**State structure:**
```rust
struct AppState {
    // Execution state
    mode: AppMode,                    // Plan | Build | Complete | Error
    current_iteration: u32,
    max_iterations: u32,

    // Task tracking
    tasks: Vec<Task>,
    current_task_idx: Option<usize>,

    // Claude subprocess
    subprocess_status: SubprocessStatus,
    output_buffer: Vec<OutputLine>,

    // Progress file
    progress_file: PathBuf,

    // UI state
    scroll_position: usize,
    collapsed_threads: HashSet<usize>,
}

enum AppMode {
    Initializing,
    Planning,
    Building { iteration: u32 },
    Paused,
    Complete { success: bool },
    Error(String),
}
```

---

### 4. Subprocess Manager

**Responsibility:** Spawn and manage Claude CLI subprocess, stream output.

**Boundaries:**
- IN: Command to execute, working directory, prompt
- OUT: Stream of output lines (stdout/stderr), exit status

**Key behaviors:**
- Spawn `claude` command with proper arguments
- Pipe stdin/stdout/stderr
- Stream output line-by-line as it arrives
- Handle subprocess termination gracefully
- Support kill-on-drop for cleanup

**Technology:** `tokio::process::Command` + `tokio-process-stream`

```rust
struct SubprocessManager {
    tx: mpsc::Sender<SubprocessEvent>,
}

enum SubprocessEvent {
    OutputLine { source: OutputSource, line: String },
    Started { pid: u32 },
    Exited { code: Option<i32> },
    Error(String),
}

enum OutputSource {
    Stdout,
    Stderr,
}
```

---

### 5. Progress File Manager

**Responsibility:** Read, parse, update, and write the progress file.

**Boundaries:**
- IN: File path, task completion updates
- OUT: Parsed progress structure, write confirmation

**Key behaviors:**
- Parse markdown progress file format
- Extract task list with checkbox states
- Update completed tasks
- Append to iteration log
- Manage "Recent Attempts" section
- Detect RALPH_DONE marker

```rust
struct ProgressFile {
    path: PathBuf,
    status: ProgressStatus,
    tasks: Vec<Task>,
    recent_attempts: Vec<AttemptRecord>,
    iteration_log: Vec<IterationEntry>,
}

struct Task {
    phase: String,
    description: String,
    completed: bool,
}
```

---

### 6. TUI Renderer

**Responsibility:** Render terminal UI, handle keyboard input.

**Boundaries:**
- IN: Application state, keyboard events
- OUT: Actions/messages to application core

**Key behaviors:**
- Render status bar (iteration, task, model, folder)
- Render context usage progress bar
- Render scrollable output area
- Render collapsible conversation threads
- Handle keyboard navigation (scroll, collapse, quit)

**Technology:** `ratatui` + `crossterm`

**Layout structure:**
```
+------------------------------------------+
| Status: Iteration 3/10 | Task 2/5 | Opus |  <- Fixed status bar
|------------------------------------------|
| [Context: ████████░░ 78%]                |  <- Progress bar
|------------------------------------------|
|                                          |
| Claude output streams here...            |  <- Scrollable area
| > Thread 1 (collapsed)                   |
| v Thread 2 (expanded)                    |
|   Line 1                                 |
|   Line 2                                 |
|                                          |
+------------------------------------------+
| q:quit | j/k:scroll | Enter:toggle       |  <- Fixed footer
+------------------------------------------+
```

---

### 7. Event Loop (Coordinator)

**Responsibility:** Coordinate async events from multiple sources.

**Boundaries:**
- IN: Terminal events, subprocess events, tick events
- OUT: Dispatch to appropriate handlers

**Key behaviors:**
- Poll terminal for keyboard input
- Receive subprocess output via channel
- Trigger periodic renders at frame rate
- Handle graceful shutdown

**Technology:** `tokio::select!` macro

```rust
loop {
    tokio::select! {
        // Terminal keyboard input
        Some(event) = terminal_events.next() => {
            handle_terminal_event(event, &mut app_state);
        }

        // Subprocess output
        Some(sub_event) = subprocess_rx.recv() => {
            handle_subprocess_event(sub_event, &mut app_state);
        }

        // Render tick
        _ = render_interval.tick() => {
            terminal.draw(|f| render_ui(f, &app_state))?;
        }
    }

    if app_state.should_quit() {
        break;
    }
}
```

---

## Data Flow

### Planning Flow

```
User Input (plan text)
       │
       v
┌──────────────────┐
│   CLI Parser     │ ──> Validates arguments
└────────┬─────────┘
         │
         v
┌──────────────────┐
│  Config Manager  │ ──> Loads/merges config
└────────┬─────────┘
         │
         v
┌──────────────────┐
│   App Core       │ ──> Initializes planning mode
└────────┬─────────┘
         │
         v
┌──────────────────┐
│ Subprocess Mgr   │ ──> Spawns: claude --prompt PROMPT_plan
└────────┬─────────┘
         │
    Output stream
         │
         v
┌──────────────────┐
│   TUI Renderer   │ ──> Displays Claude's planning output
└────────┬─────────┘
         │
         v
┌──────────────────┐
│ Progress File    │ ──> Writes structured progress.md
└──────────────────┘
```

### Build Iteration Flow

```
┌───────────────────────────────────────────────────┐
│                  ITERATION LOOP                    │
│                                                    │
│  ┌──────────────┐                                 │
│  │ Progress File │ ──> Read current state          │
│  └──────┬───────┘                                 │
│         │                                          │
│         v                                          │
│  ┌──────────────┐                                 │
│  │  App Core    │ ──> Find next incomplete task   │
│  └──────┬───────┘                                 │
│         │                                          │
│         v                                          │
│  ┌──────────────┐    claude --prompt PROMPT_build │
│  │ Subprocess   │ ──> + progress file context     │
│  └──────┬───────┘                                 │
│         │                                          │
│    ┌────┴────┐                                    │
│    │ Stream  │                                    │
│    └────┬────┘                                    │
│         │                                          │
│  ┌──────┴───────┐                                 │
│  │ mpsc channel │ ──> OutputLine events           │
│  └──────┬───────┘                                 │
│         │                                          │
│  ┌──────┴───────┐                                 │
│  │  Event Loop  │ ──> Receives events             │
│  └──────┬───────┘                                 │
│         │                                          │
│    ┌────┴────┐                                    │
│    │ select! │                                    │
│    └────┬────┘                                    │
│         │                                          │
│  ┌──────┴───────┐                                 │
│  │ TUI Renderer │ ──> Updates display             │
│  └──────────────┘                                 │
│                                                    │
│  [Subprocess exits]                               │
│         │                                          │
│         v                                          │
│  ┌──────────────┐                                 │
│  │ Progress File │ ──> Update with results        │
│  └──────┬───────┘                                 │
│         │                                          │
│         v                                          │
│  Check: RALPH_DONE? Max iterations?               │
│         │                                          │
│    NO ──┴──> Loop again                           │
│    YES ─────> Exit                                │
│                                                    │
└───────────────────────────────────────────────────┘
```

### Message Types

```rust
// From subprocess to app
enum SubprocessEvent {
    Started { pid: u32 },
    OutputLine { source: OutputSource, line: String },
    Exited { code: Option<i32> },
    Error(String),
}

// From terminal to app
enum InputEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    Quit,
}

// Internal app actions
enum Action {
    ScrollUp(usize),
    ScrollDown(usize),
    ToggleCollapse(usize),
    Quit,
    ForceStop,
    NextIteration,
}
```

---

## Module Organization

Recommended single-crate structure with clear module boundaries:

```
rslph/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, CLI parsing
│   ├── lib.rs               # Public API (optional, for testing)
│   │
│   ├── cli/
│   │   ├── mod.rs           # CLI module root
│   │   ├── args.rs          # Clap derive definitions
│   │   └── commands.rs      # Command dispatch
│   │
│   ├── config/
│   │   ├── mod.rs           # Config module root
│   │   ├── loader.rs        # TOML loading
│   │   └── types.rs         # Config structs
│   │
│   ├── app/
│   │   ├── mod.rs           # App module root
│   │   ├── state.rs         # AppState struct
│   │   ├── actions.rs       # Action enum, reducers
│   │   └── runner.rs        # Main event loop
│   │
│   ├── subprocess/
│   │   ├── mod.rs           # Subprocess module root
│   │   ├── manager.rs       # Spawn, stream, kill
│   │   └── events.rs        # SubprocessEvent types
│   │
│   ├── progress/
│   │   ├── mod.rs           # Progress module root
│   │   ├── parser.rs        # Parse progress.md
│   │   ├── writer.rs        # Write/update progress.md
│   │   └── types.rs         # Task, ProgressFile structs
│   │
│   ├── tui/
│   │   ├── mod.rs           # TUI module root
│   │   ├── terminal.rs      # Terminal init/restore
│   │   ├── renderer.rs      # Main render function
│   │   ├── widgets/
│   │   │   ├── mod.rs
│   │   │   ├── status_bar.rs
│   │   │   ├── progress_bar.rs
│   │   │   ├── output_view.rs
│   │   │   └── thread_list.rs
│   │   └── events.rs        # Terminal event handling
│   │
│   ├── prompts/
│   │   ├── mod.rs           # Prompts module root
│   │   ├── plan.rs          # PROMPT_plan (embedded)
│   │   └── build.rs         # PROMPT_build (embedded)
│   │
│   └── error.rs             # Error types (thiserror)
│
├── prompts/                  # Source for embedded prompts
│   ├── PROMPT_plan.md
│   └── PROMPT_build.md
│
└── tests/
    ├── integration/
    │   ├── cli_tests.rs
    │   └── progress_tests.rs
    └── fixtures/
        └── sample_progress.md
```

### Module Visibility

```rust
// main.rs
mod cli;
mod config;
mod app;
mod subprocess;
mod progress;
mod tui;
mod prompts;
mod error;

use cli::Cli;
use config::AppConfig;
use app::App;
use error::Result;
```

### Embedding Prompts

Use `include_str!` for compile-time embedding:

```rust
// prompts/plan.rs
pub const PROMPT_PLAN: &str = include_str!("../../prompts/PROMPT_plan.md");

// prompts/build.rs
pub const PROMPT_BUILD: &str = include_str!("../../prompts/PROMPT_build.md");
```

---

## Async Patterns

### Pattern 1: Tokio MPSC for Subprocess Events

The subprocess manager runs in its own task and sends events through a channel:

```rust
use tokio::sync::mpsc;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};

async fn spawn_claude(
    config: &SubprocessConfig,
    tx: mpsc::Sender<SubprocessEvent>,
) -> Result<()> {
    let mut child = Command::new(&config.claude_command)
        .args(&["--prompt", &config.prompt])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let tx_out = tx.clone();
    let tx_err = tx.clone();

    // Stream stdout
    tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = tx_out.send(SubprocessEvent::OutputLine {
                source: OutputSource::Stdout,
                line,
            }).await;
        }
    });

    // Stream stderr
    tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = tx_err.send(SubprocessEvent::OutputLine {
                source: OutputSource::Stderr,
                line,
            }).await;
        }
    });

    // Wait for exit
    let status = child.wait().await?;
    tx.send(SubprocessEvent::Exited {
        code: status.code()
    }).await?;

    Ok(())
}
```

### Pattern 2: Select! for Event Coordination

The main event loop uses `tokio::select!` to handle multiple event sources:

```rust
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use crossterm::event::{Event, EventStream};
use futures::StreamExt;

async fn run_event_loop(
    mut app: App,
    mut subprocess_rx: mpsc::Receiver<SubprocessEvent>,
    mut terminal: Terminal<CrosstermBackend<Stderr>>,
) -> Result<()> {
    let mut reader = EventStream::new();
    let mut render_interval = interval(Duration::from_millis(16)); // ~60fps

    loop {
        tokio::select! {
            // Terminal input (keyboard, mouse, resize)
            Some(Ok(event)) = reader.next() => {
                if let Event::Key(key) = event {
                    app.handle_key(key);
                }
            }

            // Subprocess output
            Some(event) = subprocess_rx.recv() => {
                app.handle_subprocess_event(event);
            }

            // Render tick
            _ = render_interval.tick() => {
                terminal.draw(|f| app.render(f))?;
            }
        }

        if app.should_quit() {
            break;
        }
    }

    Ok(())
}
```

### Pattern 3: Graceful Shutdown

Handle Ctrl+C and subprocess cleanup:

```rust
use tokio::signal;

async fn run_with_shutdown(app: App) -> Result<()> {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
    };

    tokio::select! {
        result = run_event_loop(app) => result,
        _ = ctrl_c => {
            // Cleanup
            Ok(())
        }
    }
}
```

### Pattern 4: Backpressure Handling

Use bounded channels to prevent memory exhaustion from fast subprocess output:

```rust
// Bounded channel - sender will await if buffer full
let (tx, rx) = mpsc::channel::<SubprocessEvent>(1000);

// If TUI can't keep up, subprocess output will buffer
// up to 1000 events, then subprocess task will await
```

---

## Build Order

Build components in dependency order, validating each before proceeding:

### Phase 1: Foundation (No TUI)

**Order:** Config -> CLI -> Progress -> Subprocess

1. **Config types + loader** (no dependencies)
   - Define `AppConfig` struct
   - Implement TOML loading with serde
   - Unit test config parsing

2. **CLI parser** (depends on: config types)
   - Define clap structs
   - Wire config loading
   - Test: `ralph --help`, `ralph plan "test"` exits cleanly

3. **Progress file parser/writer** (no dependencies)
   - Define `Task`, `ProgressFile` structs
   - Parse markdown format
   - Write/update logic
   - Unit test with fixtures

4. **Subprocess manager** (depends on: tokio)
   - Spawn process, capture output
   - Stream to channel
   - Test: spawn `echo "hello"`, receive output

**Validation checkpoint:** Can load config, parse arguments, read/write progress files, spawn subprocess and capture output.

### Phase 2: App Core (State Machine)

**Order:** State -> Actions -> Runner

5. **App state** (depends on: config, progress types)
   - Define `AppState`, `AppMode` enums
   - State initialization from config

6. **Actions/reducers** (depends on: state)
   - Define `Action` enum
   - Implement state transitions
   - Unit test state machine

7. **Event loop runner** (depends on: state, subprocess, actions)
   - Wire `tokio::select!` loop
   - Handle subprocess events
   - Test: Run iteration, capture output, update state

**Validation checkpoint:** Can run a complete build iteration headlessly (no TUI), updating progress file.

### Phase 3: TUI (Visual Layer)

**Order:** Terminal -> Widgets -> Renderer -> Integration

8. **Terminal setup** (depends on: ratatui)
   - Init/restore terminal
   - Raw mode, alternate screen
   - Test: Show blank screen, exit cleanly

9. **Individual widgets** (depends on: terminal)
   - Status bar widget
   - Progress bar widget
   - Output view widget (scrollable)
   - Thread list widget (collapsible)
   - Test each widget in isolation

10. **Main renderer** (depends on: widgets, state)
    - Layout composition
    - Render from `AppState`
    - Test: Render static state

11. **Event integration** (depends on: renderer, event loop)
    - Add keyboard handling to event loop
    - Wire render interval
    - Full integration test

**Validation checkpoint:** Can run `ralph build` with TUI, see live output, scroll, quit.

### Phase 4: Polish

**Order:** Prompts -> Notifications -> Error handling

12. **Embedded prompts** (no dependencies)
    - Create PROMPT_plan.md, PROMPT_build.md
    - Embed with `include_str!`

13. **Notification system** (depends on: subprocess)
    - Call notify script on completion
    - Test: Script receives correct arguments

14. **Error handling refinement** (all modules)
    - Replace unwraps with proper error handling
    - User-friendly error messages
    - color-eyre integration

---

## Component Dependency Graph

```
                    ┌─────────┐
                    │  CLI    │
                    └────┬────┘
                         │
            ┌────────────┼────────────┐
            │            │            │
            v            v            v
      ┌─────────┐  ┌─────────┐  ┌─────────┐
      │ Config  │  │ Prompts │  │ Error   │
      └────┬────┘  └────┬────┘  └────┬────┘
           │            │            │
           └────────────┼────────────┘
                        │
                        v
                  ┌─────────┐
                  │App Core │
                  └────┬────┘
                       │
         ┌─────────────┼─────────────┐
         │             │             │
         v             v             v
   ┌──────────┐  ┌──────────┐  ┌──────────┐
   │Subprocess│  │ Progress │  │   TUI    │
   │ Manager  │  │   File   │  │ Renderer │
   └──────────┘  └──────────┘  └──────────┘
```

---

## Anti-Patterns to Avoid

### 1. Blocking the Event Loop

**Wrong:**
```rust
// This blocks everything
let output = std::process::Command::new("claude").output()?;
```

**Right:**
```rust
// Async subprocess with streaming
let child = tokio::process::Command::new("claude").spawn()?;
```

### 2. Shared Mutable State Without Channels

**Wrong:**
```rust
// Race conditions
let state = Arc::new(Mutex::new(AppState::new()));
// Multiple tasks lock/unlock
```

**Right:**
```rust
// Single owner of state, receives events through channels
let (tx, rx) = mpsc::channel(100);
// Tasks send events, one task owns and updates state
```

### 3. Unbounded Output Buffers

**Wrong:**
```rust
// Will OOM if subprocess outputs enough
let mut all_output: Vec<String> = Vec::new();
while let Some(line) = lines.next_line().await? {
    all_output.push(line);
}
```

**Right:**
```rust
// Ring buffer for recent output
struct OutputBuffer {
    lines: VecDeque<String>,
    max_lines: usize,
}
```

### 4. Tight Render Loop

**Wrong:**
```rust
// Burns CPU
loop {
    terminal.draw(|f| render(f, &state))?;
}
```

**Right:**
```rust
// Render at fixed interval
let mut interval = tokio::time::interval(Duration::from_millis(16));
loop {
    tokio::select! {
        _ = interval.tick() => {
            terminal.draw(|f| render(f, &state))?;
        }
        // ...other events
    }
}
```

---

## Technology Stack Summary

| Component | Technology | Version | Confidence |
|-----------|------------|---------|------------|
| Async Runtime | tokio | latest stable | HIGH |
| TUI Rendering | ratatui | 0.29+ | HIGH |
| Terminal Backend | crossterm | 0.28+ | HIGH |
| CLI Parsing | clap (derive) | 4.5+ | HIGH |
| Config Parsing | toml + serde | latest | HIGH |
| Error Handling | thiserror + color-eyre | latest | HIGH |
| Process Streaming | tokio::process | built-in | HIGH |
| Tree Widget | tui-tree-widget | 0.24+ | MEDIUM |
| Scrollable View | tui-scrollview | latest | MEDIUM |

---

## Sources

- [Ratatui Application Patterns](https://ratatui.rs/concepts/application-patterns/) - Official documentation on TEA and Component patterns
- [Ratatui Async Template](https://ratatui.github.io/async-template/) - Opinionated async TUI structure
- [Tokio Channels Tutorial](https://tokio.rs/tokio/tutorial/channels) - MPSC and oneshot patterns
- [Tokio Process Command](https://docs.rs/tokio/latest/tokio/process/struct.Command.html) - Async subprocess API
- [tokio-process-stream](https://lib.rs/crates/tokio-process-stream) - Streaming subprocess output
- [Cargo Workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html) - Multi-crate organization
- [Clap Derive Guide](https://infobytes.guru/articles/rust-cli-clap-guide.html) - CLI parsing patterns
- [TOML Crate](https://lib.rs/crates/toml) - Configuration parsing
- [tui-tree-widget](https://lib.rs/crates/tui-tree-widget) - Collapsible tree views
- [Ratatui Scrollbar](https://docs.rs/ratatui/latest/ratatui/widgets/struct.Scrollbar.html) - Scrollable content
- [Awesome Ratatui](https://github.com/ratatui/awesome-ratatui) - Widget ecosystem
