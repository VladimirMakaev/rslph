# Phase 2: Subprocess Management - Research

**Researched:** 2026-01-17
**Domain:** Rust async subprocess management with Tokio
**Confidence:** HIGH

## Summary

Phase 2 implements subprocess management for running Claude CLI with real-time output streaming, signal handling, and timeout management. The Rust ecosystem has mature, well-documented solutions for all requirements through Tokio's async runtime.

The standard approach uses `tokio::process::Command` for async subprocess spawning with piped stdout/stderr, `tokio::io::BufReader` with `AsyncBufReadExt::lines()` for line-by-line streaming, `tokio::signal::ctrl_c()` with `CancellationToken` for graceful shutdown, and `tokio::time::timeout()` for stuck process detection. Process group management via `process_group(0)` prevents signal inheritance issues.

**Primary recommendation:** Use Tokio ecosystem (tokio, tokio-util) with explicit process group isolation and structured cancellation via CancellationToken. Do not use third-party subprocess wrapper crates - tokio::process is sufficient and well-maintained.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tokio | 1.49+ | Async runtime with process support | De facto standard async runtime, official subprocess API |
| tokio-util | 0.7+ | CancellationToken, TaskTracker | Official companion crate for shutdown patterns |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| nix | 0.29+ | Low-level Unix signal handling | Only if need process group kill beyond SIGKILL |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| tokio::process | tokio-process-stream | Adds Stream trait abstraction, but unnecessary complexity for single process |
| tokio::signal | ctrlc crate | ctrlc is callback-based, harder to integrate with async; tokio::signal is native async |
| Manual shutdown | tokio-graceful-shutdown | Full subsystem management, overkill for single subprocess |
| Manual child reaping | kill_tree | Recursive process tree killing, but Claude is single process |

**Installation:**
```bash
cargo add tokio --features full
cargo add tokio-util --features sync
```

Note: `tokio --features full` includes: rt-multi-thread, io-util, io-std, net, time, process, sync, signal, fs, macros

## Architecture Patterns

### Recommended Module Structure
```
src/
├── subprocess/
│   ├── mod.rs           # Module exports
│   ├── runner.rs        # ClaudeRunner struct - spawns and manages Claude process
│   ├── output.rs        # OutputLine enum, streaming abstraction
│   └── signals.rs       # Signal handling, shutdown coordination
├── lib.rs               # Add: pub mod subprocess
└── ...existing modules
```

### Pattern 1: Structured Subprocess Runner
**What:** Encapsulate subprocess lifecycle in a struct with async methods
**When to use:** Always - provides clean API and resource management
**Example:**
```rust
// Source: Tokio official docs pattern
use tokio::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, BufReader};
use std::process::Stdio;

pub struct ClaudeRunner {
    child: Child,
    stdout_reader: BufReader<tokio::process::ChildStdout>,
    stderr_reader: BufReader<tokio::process::ChildStderr>,
}

impl ClaudeRunner {
    pub async fn spawn(claude_path: &str, args: &[&str]) -> std::io::Result<Self> {
        let mut child = Command::new(claude_path)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .process_group(0)  // New process group - prevents SIGINT inheritance
            .kill_on_drop(true)  // Safety: kill child if handle dropped
            .spawn()?;

        let stdout = child.stdout.take().expect("stdout piped");
        let stderr = child.stderr.take().expect("stderr piped");

        Ok(Self {
            child,
            stdout_reader: BufReader::new(stdout),
            stderr_reader: BufReader::new(stderr),
        })
    }
}
```

### Pattern 2: Concurrent stdout/stderr Reading with select!
**What:** Read from both streams simultaneously to avoid buffer deadlock
**When to use:** Always when capturing both stdout and stderr
**Example:**
```rust
// Source: Tokio select tutorial + AsyncBufReadExt docs
use tokio::io::AsyncBufReadExt;

#[derive(Debug, Clone)]
pub enum OutputLine {
    Stdout(String),
    Stderr(String),
    ProcessExited(Option<i32>),
}

impl ClaudeRunner {
    pub async fn next_line(&mut self) -> Option<OutputLine> {
        let mut stdout_line = String::new();
        let mut stderr_line = String::new();

        tokio::select! {
            result = self.stdout_reader.read_line(&mut stdout_line) => {
                match result {
                    Ok(0) => None, // EOF
                    Ok(_) => Some(OutputLine::Stdout(stdout_line.trim_end().to_string())),
                    Err(_) => None,
                }
            }
            result = self.stderr_reader.read_line(&mut stderr_line) => {
                match result {
                    Ok(0) => None, // EOF
                    Ok(_) => Some(OutputLine::Stderr(stderr_line.trim_end().to_string())),
                    Err(_) => None,
                }
            }
        }
    }
}
```

### Pattern 3: Graceful Shutdown with CancellationToken
**What:** Use CancellationToken for structured cancellation across async boundaries
**When to use:** For Ctrl+C handling and timeout-triggered shutdown
**Example:**
```rust
// Source: Tokio graceful shutdown guide
use tokio_util::sync::CancellationToken;
use tokio::signal;

pub async fn run_with_cancellation(
    runner: &mut ClaudeRunner,
    cancel_token: CancellationToken,
) -> Result<(), Error> {
    loop {
        tokio::select! {
            biased;  // Check cancellation first

            _ = cancel_token.cancelled() => {
                // Graceful shutdown requested
                runner.terminate().await?;
                return Ok(());
            }
            line = runner.next_line() => {
                match line {
                    Some(output) => handle_output(output),
                    None => break, // Process ended
                }
            }
        }
    }
    Ok(())
}

// Signal handler setup
pub async fn setup_signal_handler(cancel_token: CancellationToken) {
    tokio::spawn(async move {
        signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
        cancel_token.cancel();
    });
}
```

### Pattern 4: Timeout with Process Termination
**What:** Wrap subprocess execution in timeout, kill on expiry
**When to use:** For PROC-04 stuck Claude timeout
**Example:**
```rust
// Source: tokio::time::timeout docs
use tokio::time::{timeout, Duration};

pub async fn run_with_timeout(
    runner: &mut ClaudeRunner,
    max_duration: Duration,
) -> Result<(), Error> {
    match timeout(max_duration, runner.run_to_completion()).await {
        Ok(result) => result,
        Err(_elapsed) => {
            // Timeout occurred - kill the process
            runner.kill().await?;
            Err(Error::Timeout)
        }
    }
}
```

### Pattern 5: Clean Process Termination
**What:** SIGTERM first, wait, then SIGKILL if needed
**When to use:** For graceful Ctrl+C shutdown (PROC-03)
**Example:**
```rust
// Source: tokio::process::Child docs + nix::sys::signal
use std::time::Duration;
use tokio::time::sleep;

impl ClaudeRunner {
    pub async fn terminate(&mut self) -> std::io::Result<()> {
        // Try graceful termination first
        if let Some(id) = self.child.id() {
            // Send SIGTERM to process group
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;
                let _ = kill(Pid::from_raw(-(id as i32)), Signal::SIGTERM);
            }

            // Wait briefly for graceful exit
            tokio::select! {
                _ = self.child.wait() => return Ok(()),
                _ = sleep(Duration::from_secs(5)) => {}
            }
        }

        // Force kill if still running
        self.child.kill().await
    }

    pub async fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill().await
    }
}
```

### Anti-Patterns to Avoid
- **Reading stdout/stderr sequentially:** Causes buffer deadlock if one fills while reading other
- **Not setting process_group(0):** Child inherits terminal SIGINT, making graceful shutdown impossible
- **Dropping Child without wait/kill:** Creates zombie processes on Unix
- **Using read_line in select without fresh buffer:** Accumulates partial reads incorrectly
- **Recreating sleep futures in loop:** Loses timeout state, causes non-deterministic behavior

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Async subprocess spawning | std::process + threads | tokio::process::Command | Native async, no thread overhead |
| Cancellation signaling | mpsc channels + flags | tokio_util::CancellationToken | Designed for this, handles edge cases |
| Line-by-line reading | Manual buffer management | AsyncBufReadExt::lines() | Handles partial reads, newline variants |
| Timeout handling | spawn + sleep + channels | tokio::time::timeout() | Composable, automatic cancellation |
| Signal handling | libc + signal handlers | tokio::signal::ctrl_c() | Async-safe, cross-platform |
| Process group killing | Manual kill loops | process_group(0) + nix::kill | OS handles group signal delivery |

**Key insight:** Tokio's subprocess and signal APIs are designed to work together. Using std::process with threads loses this integration and creates complex synchronization issues.

## Common Pitfalls

### Pitfall 1: Buffer Deadlock
**What goes wrong:** Reading stdout blocks while stderr buffer fills (or vice versa), deadlocking the process
**Why it happens:** OS pipe buffers are limited (~64KB). If child writes to stderr faster than parent reads, buffer fills and child blocks.
**How to avoid:** Always read stdout and stderr concurrently using `tokio::select!` or spawn separate tasks for each
**Warning signs:** Process hangs after producing some output, works with small outputs but fails with large

### Pitfall 2: Zombie Processes
**What goes wrong:** Child processes become zombies, consuming PID table entries
**Why it happens:** On Unix, parent must call wait() to reap children. Dropping Child handle without wait/kill doesn't reap.
**How to avoid:** Always call `child.wait()`, `child.kill().await`, or use `kill_on_drop(true)`
**Warning signs:** `ps aux | grep defunct` shows zombie processes after running rslph

### Pitfall 3: Signal Inheritance
**What goes wrong:** Pressing Ctrl+C kills both rslph AND Claude, preventing state saving
**Why it happens:** Child processes inherit parent's process group by default, receiving same terminal signals
**How to avoid:** Use `.process_group(0)` when spawning to put child in new group
**Warning signs:** Claude dies instantly on Ctrl+C without chance for rslph to save state

### Pitfall 4: Cancellation Safety in select!
**What goes wrong:** Partial data loss when select! branch is cancelled mid-read
**Why it happens:** `read_line()` is NOT cancellation-safe - partial reads are lost when future is dropped
**How to avoid:** Use `lines().next_line()` which IS cancellation-safe, or manage buffer state externally
**Warning signs:** Truncated or missing lines in output, especially under load

### Pitfall 5: Timeout Doesn't Kill
**What goes wrong:** Process continues running after timeout, consuming resources
**Why it happens:** `tokio::time::timeout()` only drops the future, doesn't send kill signal
**How to avoid:** Explicitly call `child.kill()` in timeout error handler
**Warning signs:** Claude processes accumulate in background after timeouts

### Pitfall 6: SIGTERM vs SIGKILL
**What goes wrong:** Process doesn't terminate cleanly, may leave corrupted state
**Why it happens:** Using SIGKILL immediately doesn't give process chance to clean up
**How to avoid:** Send SIGTERM first, wait (e.g., 5 seconds), then SIGKILL if still alive
**Warning signs:** Corrupted conversation logs, file locks not released

## Code Examples

Verified patterns from official sources:

### Complete Subprocess Runner with All Features
```rust
// Source: Composite from Tokio docs (process, signal, time modules)
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub enum OutputLine {
    Stdout(String),
    Stderr(String),
}

pub struct ClaudeRunner {
    child: Child,
    stdout: tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    stderr: tokio::io::Lines<BufReader<tokio::process::ChildStderr>>,
}

impl ClaudeRunner {
    pub async fn spawn(
        claude_path: &str,
        prompt: &str,
        working_dir: &Path,
    ) -> std::io::Result<Self> {
        let mut child = Command::new(claude_path)
            .arg("--print")  // Non-interactive mode
            .arg("-p")
            .arg(prompt)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .process_group(0)  // CRITICAL: Isolate from terminal signals
            .kill_on_drop(true)  // Safety net
            .spawn()?;

        let stdout = child.stdout.take().expect("stdout configured as piped");
        let stderr = child.stderr.take().expect("stderr configured as piped");

        Ok(Self {
            child,
            stdout: BufReader::new(stdout).lines(),
            stderr: BufReader::new(stderr).lines(),
        })
    }

    /// Read next line from either stdout or stderr (cancellation-safe)
    pub async fn next_output(&mut self) -> Option<OutputLine> {
        tokio::select! {
            result = self.stdout.next_line() => {
                match result {
                    Ok(Some(line)) => Some(OutputLine::Stdout(line)),
                    _ => None,
                }
            }
            result = self.stderr.next_line() => {
                match result {
                    Ok(Some(line)) => Some(OutputLine::Stderr(line)),
                    _ => None,
                }
            }
        }
    }

    /// Get process ID (for external monitoring)
    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    /// Graceful termination: SIGTERM -> wait -> SIGKILL
    pub async fn terminate_gracefully(&mut self, grace_period: Duration) -> std::io::Result<()> {
        #[cfg(unix)]
        if let Some(id) = self.child.id() {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;
            // Negative PID sends to process group
            let _ = kill(Pid::from_raw(-(id as i32)), Signal::SIGTERM);

            // Wait for graceful exit or timeout
            if timeout(grace_period, self.child.wait()).await.is_ok() {
                return Ok(());
            }
        }

        // Force kill if still running
        self.child.kill().await
    }

    /// Wait for process to complete
    pub async fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.child.wait().await
    }
}
```

### Signal Handler Setup
```rust
// Source: Tokio signal docs + graceful shutdown guide
use tokio::signal;
use tokio_util::sync::CancellationToken;

pub fn setup_ctrl_c_handler(token: CancellationToken) {
    tokio::spawn(async move {
        if let Err(e) = signal::ctrl_c().await {
            eprintln!("Failed to listen for Ctrl+C: {}", e);
            return;
        }
        token.cancel();
    });
}
```

### Main Execution Loop with Timeout
```rust
// Source: Composite pattern from Tokio tutorials
use std::time::Duration;
use tokio::time::timeout;

pub async fn run_claude_iteration(
    runner: &mut ClaudeRunner,
    cancel: CancellationToken,
    max_duration: Duration,
) -> Result<Vec<OutputLine>, Error> {
    let mut output = Vec::new();

    let result = timeout(max_duration, async {
        loop {
            tokio::select! {
                biased;

                _ = cancel.cancelled() => {
                    return Err(Error::Cancelled);
                }

                line = runner.next_output() => {
                    match line {
                        Some(l) => output.push(l),
                        None => break,  // Process finished
                    }
                }
            }
        }
        Ok(())
    }).await;

    match result {
        Ok(Ok(())) => Ok(output),
        Ok(Err(e)) => Err(e),
        Err(_timeout) => {
            runner.terminate_gracefully(Duration::from_secs(5)).await?;
            Err(Error::Timeout)
        }
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| std::process + threads | tokio::process | Tokio 1.0 (2020) | Native async subprocess |
| mpsc channels for cancellation | CancellationToken | tokio-util 0.6 (2021) | Cleaner shutdown patterns |
| unsafe pre_exec for process group | process_group(0) | Rust 1.64 (2022) | Safe, portable process isolation |
| Manual zombie reaping | kill_on_drop + runtime reaping | Tokio 1.0 | Automatic cleanup on best-effort basis |

**Deprecated/outdated:**
- `before_exec()`: Deprecated, use `pre_exec()` instead
- Thread-based subprocess: Use tokio::process for async integration
- ctrlc crate with tokio: Use native tokio::signal::ctrl_c() instead

## Open Questions

Things that couldn't be fully resolved:

1. **Claude CLI exact output format**
   - What we know: Claude CLI can output JSONL format, has `--print` flag for non-interactive mode
   - What's unclear: Exact flags for rslph use case (streaming vs batch, output format options)
   - Recommendation: Test with actual Claude CLI, document discovered flags in implementation

2. **Optimal timeout duration**
   - What we know: Need configurable timeout (PROC-04)
   - What's unclear: What's a reasonable default? Claude operations can legitimately take minutes.
   - Recommendation: Use config.max_iteration_timeout with generous default (e.g., 10 minutes), allow override

3. **State saving on Ctrl+C timing**
   - What we know: Need to save state before Claude terminates (PROC-03)
   - What's unclear: How much time between rslph receiving SIGINT and Claude receiving SIGTERM?
   - Recommendation: With process_group(0), rslph controls timing completely. Save state BEFORE sending SIGTERM.

## Sources

### Primary (HIGH confidence)
- [Tokio process module docs](https://docs.rs/tokio/latest/tokio/process/index.html) - Command, Child, spawning
- [Tokio signal module docs](https://docs.rs/tokio/latest/tokio/signal/index.html) - ctrl_c(), Unix signals
- [Tokio time::timeout docs](https://docs.rs/tokio/latest/tokio/time/fn.timeout.html) - Timeout wrapper
- [AsyncBufReadExt docs](https://docs.rs/tokio/latest/tokio/io/trait.AsyncBufReadExt.html) - lines(), next_line()
- [Tokio select tutorial](https://tokio.rs/tokio/tutorial/select) - Concurrent async operations
- [Tokio graceful shutdown guide](https://tokio.rs/tokio/topics/shutdown) - CancellationToken patterns
- [tokio_util CancellationToken](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html) - API reference
- [CommandExt::process_group](https://doc.rust-lang.org/stable/std/os/unix/process/trait.CommandExt.html) - Stable since Rust 1.64
- [tokio::process::Child docs](https://docs.rs/tokio/latest/tokio/process/struct.Child.html) - kill, wait, zombie warnings

### Secondary (MEDIUM confidence)
- [Tokio zombie process issue #2685](https://github.com/tokio-rs/tokio/issues/2685) - Zombie handling discussion and fixes
- [nix::sys::signal::kill](https://docs.rs/nix/latest/nix/sys/signal/fn.kill.html) - Process group signals
- [claude-stream tool](https://github.com/shitchell/claude-stream) - Claude JSONL output format reference

### Tertiary (LOW confidence)
- Medium articles on async patterns - General guidance, not authoritative

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Official Tokio documentation, stable APIs
- Architecture: HIGH - Patterns from official Tokio guides and tutorials
- Pitfalls: HIGH - Documented in official docs and issue tracker

**Research date:** 2026-01-17
**Valid until:** 2026-02-17 (Tokio is stable, unlikely to change significantly)
