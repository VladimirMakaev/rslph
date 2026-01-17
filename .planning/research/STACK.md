# Stack Research: Rust CLI/TUI Autonomous Coding Agent

**Project:** Ralph (rslph)
**Researched:** 2026-01-17
**Focus:** CLI application with TUI, subprocess management, TOML configuration

---

## Recommended Stack

### TUI Framework

| Technology | Version | Confidence |
|------------|---------|------------|
| **ratatui** | 0.30.0 | HIGH |

**Rationale:**
- Ratatui is the de facto standard for Rust TUI development in 2025/2026, having fully replaced the deprecated tui-rs since 2023
- Active community with official async template and extensive documentation
- Supports all features Ralph needs: layouts, styled text, widgets for status bars, scrollable areas
- Multiple backend support (crossterm default, termion, termwiz) provides flexibility
- Immediate-mode rendering model works well with async event loops

**Key Features for Ralph:**
- `Layout` for splitting terminal into status bar + main content + collapsible sections
- `Paragraph` with `Wrap` for streaming output display
- `Block` with borders for visual thread separation
- `Scrollbar` widget for long output navigation
- `Line` and `Span` for styled status information

**Configuration:**
```toml
[dependencies]
ratatui = { version = "0.30", default-features = true }
```

---

### Terminal Backend

| Technology | Version | Confidence |
|------------|---------|------------|
| **crossterm** | 0.29.0 | HIGH |

**Rationale:**
- Default backend for ratatui, battle-tested integration
- Pure Rust implementation (no system dependencies like ncurses)
- Cross-platform: Windows, macOS, Linux without configuration
- `event-stream` feature enables async event handling with tokio
- Supports all terminal features Ralph needs: raw mode, alternate screen, mouse events

**Key Features for Ralph:**
- `event-stream` feature for async keyboard/mouse input
- Cursor hiding during TUI operation
- Alternate screen to preserve user's terminal state
- Proper cleanup on panic/exit

**Configuration:**
```toml
[dependencies]
crossterm = { version = "0.29", features = ["event-stream"] }
```

---

### Async Runtime

| Technology | Version | Confidence |
|------------|---------|------------|
| **tokio** | 1.49.0 | HIGH |

**Rationale:**
- Industry standard async runtime for Rust
- Native `tokio::process` module for async subprocess management (critical for piloting Claude CLI)
- Excellent integration with ratatui via crossterm's event-stream
- Rich ecosystem: tracing integration, channels, timers
- `tokio-util` provides `CancellationToken` for graceful shutdown

**Key Features for Ralph:**
- `tokio::process::Command` for spawning Claude CLI
- `tokio::io::AsyncBufReadExt` for streaming stdout/stderr line-by-line
- `tokio::select!` for multiplexing subprocess output with keyboard events
- `tokio::sync::mpsc` for event channels between components
- `tokio::time` for iteration delays and timeouts

**Configuration:**
```toml
[dependencies]
tokio = { version = "1.49", features = ["full"] }
tokio-util = { version = "0.7", features = ["rt"] }
```

Note: `features = ["full"]` includes: rt-multi-thread, io-util, io-std, fs, net, time, process, sync, signal, macros.

---

### CLI Argument Parsing

| Technology | Version | Confidence |
|------------|---------|------------|
| **clap** | 4.5.54 | HIGH |

**Rationale:**
- De facto standard for Rust CLI argument parsing
- Derive macro API makes defining subcommands (`ralph plan`, `ralph build`) ergonomic
- Automatic help generation, error messages, shell completions
- Supports CLI overrides for configuration values (e.g., `--model`, `--max-iterations`)
- Integrates cleanly with figment for layered configuration

**Key Features for Ralph:**
- `#[derive(Parser)]` for type-safe argument definitions
- Subcommand support for `plan` and `build` commands
- Optional arguments with defaults from config
- Value validation and type coercion

**Configuration:**
```toml
[dependencies]
clap = { version = "4.5", features = ["derive", "env"] }
```

The `env` feature allows environment variable fallbacks for arguments.

---

### Configuration

| Technology | Version | Confidence |
|------------|---------|------------|
| **figment** | 0.10.19 | HIGH |
| **toml** | 0.9.11 | HIGH |
| **serde** | 1.0.228 | HIGH |

**Rationale:**
- Figment provides layered configuration merging (defaults < file < env < CLI)
- Native TOML support via feature flag
- Seamlessly integrates with clap for CLI overrides
- Type-safe deserialization into Rust structs via serde
- Supports multiple config file locations (project-local, user home)

**Key Features for Ralph:**
- `Figment::new()` chain for layered config sources
- `providers::Serialized` to merge clap args into config
- `providers::Toml` for ralph.toml parsing
- `providers::Env` for RALPH_* environment variables

**Configuration:**
```toml
[dependencies]
figment = { version = "0.10", features = ["toml", "env"] }
toml = "0.9"
serde = { version = "1.0", features = ["derive"] }
```

---

### Process Management

| Technology | Version | Confidence |
|------------|---------|------------|
| **tokio::process** | (included in tokio) | HIGH |

**Rationale:**
- Built into tokio, no additional dependency
- `Command::new().stdout(Stdio::piped()).stderr(Stdio::piped())` for output capture
- `child.stdout.take()` returns async `ChildStdout` for streaming
- Native async/await integration
- `child.kill()` and `child.wait()` for lifecycle management

**Key Features for Ralph:**
- Spawn Claude CLI as subprocess
- Stream stdout/stderr in real-time for TUI display
- Capture exit codes for iteration logic
- Kill subprocess on user interrupt (Ctrl+C)

**Usage Pattern:**
```rust
use tokio::process::Command;
use tokio::io::{BufReader, AsyncBufReadExt};

let mut child = Command::new("claude")
    .args(&["--print", prompt_file])
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;

let stdout = child.stdout.take().unwrap();
let mut reader = BufReader::new(stdout).lines();

while let Some(line) = reader.next_line().await? {
    // Send to TUI for display
}
```

---

### Error Handling

| Technology | Version | Confidence |
|------------|---------|------------|
| **color-eyre** | 0.6.5 | HIGH |
| **thiserror** | 2.0.17 | HIGH |

**Rationale:**
- color-eyre provides beautiful, colorized error reports with backtraces
- Recommended by ratatui's official async template
- thiserror for defining domain-specific error types with derive macros
- Both work together: thiserror for error definitions, color-eyre for reporting

**Configuration:**
```toml
[dependencies]
color-eyre = "0.6"
thiserror = "2.0"
```

**Usage Pattern:**
```rust
// In main.rs
fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    // ...
}

// Domain errors with thiserror
#[derive(Debug, thiserror::Error)]
pub enum RalphError {
    #[error("Config file not found: {0}")]
    ConfigNotFound(PathBuf),

    #[error("Claude CLI failed with exit code {0}")]
    ClaudeExitError(i32),
}
```

---

### Logging/Tracing

| Technology | Version | Confidence |
|------------|---------|------------|
| **tracing** | 0.1.44 | HIGH |
| **tracing-subscriber** | (latest) | HIGH |

**Rationale:**
- Structured, async-aware logging designed for tokio applications
- Span-based tracing tracks execution across async boundaries
- Integrates with color-eyre for span traces in error reports
- Can log to file while TUI uses terminal

**Key Features for Ralph:**
- Log subprocess events, iteration starts/ends
- Debug output to file (not terminal, which TUI uses)
- `#[instrument]` attribute for automatic span creation

**Configuration:**
```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

---

### Path/Directory Management

| Technology | Version | Confidence |
|------------|---------|------------|
| **directories** | 6.0.0 | HIGH |

**Rationale:**
- Platform-appropriate paths for config, data, cache
- XDG Base Directory spec on Linux, proper paths on macOS/Windows
- `ProjectDirs` provides application-specific directories

**Key Features for Ralph:**
- Config file location: `~/.config/ralph/ralph.toml` (Linux), `~/Library/Application Support/ralph/` (macOS)
- Data directory for progress files
- Cache directory for temporary files

**Configuration:**
```toml
[dependencies]
directories = "6.0"
```

---

### Supporting Crates

| Crate | Version | Purpose | Confidence |
|-------|---------|---------|------------|
| **futures** | 0.3 | Async utilities (StreamExt, select) | HIGH |
| **chrono** | 0.4 | Timestamps for progress files | MEDIUM |
| **uuid** | 1.0 | Unique IDs for iterations | MEDIUM |

**Configuration:**
```toml
[dependencies]
futures = "0.3"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
```

---

## Full Cargo.toml Dependencies

```toml
[dependencies]
# TUI
ratatui = { version = "0.30", default-features = true }
crossterm = { version = "0.29", features = ["event-stream"] }

# Async runtime
tokio = { version = "1.49", features = ["full"] }
tokio-util = { version = "0.7", features = ["rt"] }
futures = "0.3"

# CLI
clap = { version = "4.5", features = ["derive", "env"] }

# Configuration
figment = { version = "0.10", features = ["toml", "env"] }
toml = "0.9"
serde = { version = "1.0", features = ["derive"] }

# Error handling
color-eyre = "0.6"
thiserror = "2.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Utilities
directories = "6.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
```

---

## Alternatives Considered

### TUI Framework

| Alternative | Why Not |
|-------------|---------|
| **tui-rs** | Deprecated since 2023. Ratatui is the direct successor with active maintenance. Do not use. |
| **cursive** (0.21.1) | Retained-mode model is more complex for streaming output. Less async integration. Smaller community. Good for dialog-heavy apps, not for live-updating displays. |
| **dioxus-tui** | Experimental. Brings React-like model which is overkill. Less mature. |

### Async Runtime

| Alternative | Why Not |
|-------------|---------|
| **async-std** | Smaller ecosystem. Less integration with ratatui/crossterm. tokio is the clear winner in 2025 with broader adoption. |
| **smol** | Minimalist, lacks tokio::process equivalent. Would need additional crates. |

### CLI Parsing

| Alternative | Why Not |
|-------------|---------|
| **argh** | Google-style, simpler but less flexible. No shell completions. Fewer features. clap is the community standard. |
| **pico-args** | Minimal, no derive. More boilerplate. Good for tiny CLIs, not feature-rich apps. |
| **structopt** | Deprecated. Merged into clap 3+. Use clap with derive feature instead. |

### Configuration

| Alternative | Why Not |
|-------------|---------|
| **config** crate | Older API. Figment has cleaner layering and better clap integration. |
| **confy** | Too simple for layered config (file + env + CLI). Good for trivial apps. |
| **toml only** | Would need manual merging logic. Figment handles this. |

### Error Handling

| Alternative | Why Not |
|-------------|---------|
| **anyhow** (1.0.100) | Good for applications, but color-eyre provides better error formatting and is specifically recommended for TUI apps. Both work similarly; color-eyre has nicer output. |
| **eyre** (without color) | Works, but why skip colorized output? |

---

## Confidence Notes

### HIGH Confidence
- **TUI stack (ratatui + crossterm + tokio):** This is the canonical 2025/2026 stack. The ratatui async-template demonstrates this exact combination. Versions verified from docs.rs.
- **CLI (clap):** Undisputed leader with 4.5.x series stable.
- **Configuration (figment + toml + serde):** Well-documented, actively maintained, clean APIs.
- **Error handling (color-eyre + thiserror):** Recommended by ratatui templates.

### MEDIUM Confidence
- **chrono and uuid versions:** Verified these are current, but they're utility crates with stable APIs. Exact versions less critical.
- **tracing-subscriber features:** The `env-filter` feature is commonly used, but Ralph's specific logging needs may require adjustment.

### Potential Gaps
- **Progress file format:** Research covered config/TOML but not whether a specific format (JSON, TOML, custom) is better for progress tracking between iterations. Likely TOML or JSON with serde.
- **TUI component architecture:** The ratatui async-template uses a `Component` trait pattern. May want to research this pattern more deeply during implementation.

---

## Sources

### Official Documentation (HIGH confidence)
- [ratatui 0.30.0 docs](https://docs.rs/ratatui/latest/ratatui/)
- [crossterm 0.29.0 docs](https://docs.rs/crossterm/latest/crossterm/)
- [tokio 1.49.0 docs](https://docs.rs/tokio/latest/tokio/)
- [clap 4.5.54 docs](https://docs.rs/clap/latest/clap/)
- [figment 0.10.19 docs](https://docs.rs/figment/latest/figment/)
- [color-eyre 0.6.5 docs](https://docs.rs/color-eyre/latest/color_eyre/)
- [thiserror 2.0.17 docs](https://docs.rs/thiserror/latest/thiserror/)
- [serde 1.0.228 docs](https://docs.rs/serde/latest/serde/)
- [directories 6.0.0 on lib.rs](https://lib.rs/crates/directories/)

### Tutorials and Templates (MEDIUM confidence)
- [ratatui async-template](https://github.com/ratatui/async-template) - Official async component architecture template
- [Async Counter App Tutorial](https://ratatui.rs/tutorials/counter-async-app/) - Demonstrates tokio + crossterm + ratatui integration

### Community/Ecosystem (supplementary)
- [Choosing Your Async Champion: Tokio vs. async-std in 2025](https://medium.com/rustaceans/choosing-your-async-champion-tokio-vs-async-std-in-2025-a142d3899b66)
- [From tui-rs to Ratatui migration guide](https://blog.orhun.dev/ratatui-0-23-0/)
- [Rust Error Handling Guide 2025](https://markaicode.com/rust-error-handling-2025-guide/)
