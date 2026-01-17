# Pitfalls Research: Ralph (rslph)

**Domain:** Rust CLI/TUI autonomous AI coding agent
**Researched:** 2026-01-17
**Confidence:** HIGH (verified via official documentation and authoritative sources)

---

## TUI Pitfalls (Ratatui/Crossterm)

### Pitfall 1: Terminal State Corruption on Panic

**What goes wrong:** When the application panics, the terminal remains in raw mode or alternate screen mode. The user's terminal becomes unusable - no cursor, no echo, garbled output.

**Why it happens:** Rust's default panic handler doesn't know about terminal state. The cleanup code in `Drop` implementations never runs if the panic unwinds before reaching them.

**Warning signs:**
- No custom panic hook in initialization code
- Using `?` operators in TUI code without panic handling
- Terminal becomes unresponsive during development/testing

**Prevention strategy:**
```rust
// Set up panic hook BEFORE entering alternate screen
let original_hook = std::panic::take_hook();
std::panic::set_hook(Box::new(move |panic_info| {
    // Intentionally ignore errors - we're already panicking
    let _ = crossterm::terminal::disable_raw_mode();
    let _ = crossterm::execute!(std::io::stdout(), LeaveAlternateScreen);
    original_hook(panic_info);
}));
```

**Suggested phase:** Phase 1 (TUI foundation) - Must be in place before any TUI work.

**Source:** [Ratatui Panic Hooks Documentation](https://ratatui.rs/recipes/apps/panic-hooks/)

---

### Pitfall 2: Async Runtime Blocking in TUI Event Loop

**What goes wrong:** The TUI freezes, becomes unresponsive, or misses keyboard input. Render loop stutters visibly.

**Why it happens:** Mixing blocking operations with async event handling. Using `tokio::spawn_blocking` for tasks that should be on dedicated threads. Blocking operations starving the async runtime.

**Warning signs:**
- Keyboard input feels "laggy" or batched
- Frame rate drops during subprocess output
- `spawn_blocking` tasks cannot be cancelled/aborted
- Single-threaded runtime with any blocking operations

**Prevention strategy:**
- Use `tokio::spawn` for async work, NOT `spawn_blocking` for long-running tasks
- Keep async code reaching `.await` points every 10-100 microseconds
- Use dedicated threads with message-passing channels for truly blocking operations
- Use `crossterm::event::EventStream` with `tokio::select!` for event handling

**Suggested phase:** Phase 1 (TUI foundation) - Architecture decision that's hard to change later.

**Source:** [Ratatui Forum: tokio::spawn vs spawn_blocking](https://forum.ratatui.rs/t/understanding-tokio-spawn-and-tokio-spawn-blocking/74)

---

### Pitfall 3: Unicode/Wide Character Rendering Corruption

**What goes wrong:** Japanese, Chinese, emoji, or other wide characters cause layout corruption. Text overflows boundaries. Alignment breaks.

**Why it happens:** Wide characters occupy 2 terminal cells but are 1 Rust `char`. Width calculation doesn't account for this. Ratatui has had multiple PRs fixing multi-width character rendering.

**Warning signs:**
- Text boundaries look wrong with non-ASCII content
- Scrolling artifacts with CJK characters
- Status bar content overflows

**Prevention strategy:**
- Use `unicode-width` crate for string width calculations
- Test with non-ASCII content early (emoji in task names, etc.)
- Don't assume `str.len()` equals display width
- Be aware of terminal font differences affecting rendering

**Suggested phase:** Phase 2 (streaming output) - When displaying Claude's potentially multilingual output.

**Source:** [Ratatui PR #1764: Multi-width character rendering fix](http://hub.fantasygmm.top/ratatui/ratatui/pull/1764)

---

### Pitfall 4: Resize Event Handling Race Conditions

**What goes wrong:** Application crashes or renders incorrectly after terminal resize. Layout calculations use stale dimensions.

**Why it happens:** Resize events arrive asynchronously. Render happens before dimensions update. Widget calculations overflow with negative or zero dimensions.

**Warning signs:**
- Crash on terminal resize
- Garbled output after resizing
- Panic in widget rendering with "subtract with overflow"

**Prevention strategy:**
- Handle `Event::Resize(width, height)` events explicitly
- Re-query terminal size before rendering, don't cache aggressively
- Guard against zero-width/height areas in layout calculations
- Use `Rect::default()` checks before calculations

**Suggested phase:** Phase 3 (collapsible threads) - Complex layouts are most affected.

**Source:** [Ratatui Terminal and EventHandler Recipe](https://ratatui.rs/recipes/apps/terminal-and-event-handler/)

---

## Subprocess Pitfalls (Process Management)

### Pitfall 5: Pipe Buffer Deadlock

**What goes wrong:** Application hangs indefinitely. Child process appears stuck. No output received.

**Why it happens:** OS pipes have finite buffer sizes (~64KB on Linux). If the child writes more output than the buffer holds WITHOUT the parent reading, the child blocks on write. If the parent is waiting for exit before reading, deadlock.

**Warning signs:**
- Claude CLI produces lots of output, then hangs
- `child.wait()` never returns
- Works with small outputs, fails with large ones

**Prevention strategy:**
```rust
// WRONG: Wait then read
let status = child.wait()?;
let output = read_stdout(&child)?; // DEADLOCK if output > pipe buffer

// RIGHT: Read while running
// Option 1: Use wait_with_output() which handles this
let output = child.wait_with_output()?;

// Option 2: For streaming, read in separate task
let stdout = child.stdout.take().expect("stdout piped");
tokio::spawn(async move {
    let reader = BufReader::new(stdout);
    // Read lines as they come
});
child.wait()?;
```

**Suggested phase:** Phase 1 (subprocess foundation) - Core to Claude CLI integration.

**Source:** [Rust std::process::Child Documentation](https://doc.rust-lang.org/std/process/struct.Child.html)

---

### Pitfall 6: Zombie Process Accumulation

**What goes wrong:** System slows down. "Too many open files" errors. Process table exhaustion.

**Why it happens:** Child processes that exit but aren't `wait()`ed become zombies. Rust's `Child` does NOT automatically wait on drop. Long-running agent spawning many iterations accumulates zombies.

**Warning signs:**
- `ps aux | grep defunct` shows zombie processes
- System resource exhaustion after many iterations
- PID exhaustion on long runs

**Prevention strategy:**
- ALWAYS call `wait()`, `try_wait()`, or `wait_with_output()` on every child
- Use RAII wrapper that waits on drop
- Monitor process count during testing
- Consider `Child::kill()` + `wait()` for cleanup

**Suggested phase:** Phase 1 (subprocess foundation) - Must be correct from the start.

**Source:** [Rust std::process::Child Documentation](https://doc.rust-lang.org/std/process/struct.Child.html)

---

### Pitfall 7: stdin Not Closed Before Wait

**What goes wrong:** Deadlock where parent waits for child, child waits for stdin EOF.

**Why it happens:** Child process reads from stdin until EOF. Parent holds stdin handle open while waiting. Neither can proceed.

**Warning signs:**
- Hang when child expects stdin input
- Works interactively, fails programmatically
- `child.wait()` never returns

**Prevention strategy:**
```rust
// Take and drop stdin before waiting
let mut child = Command::new("claude")
    .stdin(Stdio::piped())
    .spawn()?;

if let Some(mut stdin) = child.stdin.take() {
    stdin.write_all(input.as_bytes())?;
    // stdin dropped here, sending EOF
}

child.wait()?; // Now safe
```

**Note:** `wait()` and `wait_with_output()` automatically close stdin, but `try_wait()` does NOT.

**Suggested phase:** Phase 1 (subprocess foundation) - Part of correct subprocess handling.

**Source:** [Rust std::process::Child Documentation](https://doc.rust-lang.org/std/process/struct.Child.html)

---

## Ralph Loop Pitfalls (Autonomous AI Agent)

### Pitfall 8: Infinite Loop / Stuck Agent

**What goes wrong:** Agent repeats the same action infinitely. Token costs spiral. No progress despite activity.

**Why it happens:** Agent loses track of completed actions. Context window truncation drops completion markers. Ambiguous success criteria. No termination condition.

**Warning signs:**
- Same task appearing in consecutive iterations
- Identical command sequences repeating
- Token consumption spikes without progress
- Progress file shows same step repeatedly

**Prevention strategy:**
- **Hard iteration limits:** Maximum N iterations per task
- **Loop detection:** Hash recent actions, detect repeats
- **Progress assertions:** "Did we make forward progress?" check each iteration
- **Cost caps:** Kill after $X spent on single task
- **Timeout:** Wall-clock limit per task
- **Distinct completion criteria:** Explicit "done" markers, not "seems done"

**Suggested phase:** Phase 2 (build command) - Core to the Ralph loop implementation.

**Source:** [GitHub: Cursor AI Agent Infinite Loop Issue](https://github.com/cursor/cursor/issues/3327), [Galileo: How to Debug AI Agents](https://galileo.ai/blog/debug-ai-agents)

---

### Pitfall 9: Context Truncation Loses Critical Instructions

**What goes wrong:** Agent "forgets" constraints partway through. Produces output violating earlier instructions. Ignores progress file content.

**Why it happens:** Claude CLI's context window is finite. Earlier messages get truncated as conversation grows. Ralph's core value (fresh context per iteration) helps, but within-iteration truncation can occur.

**Warning signs:**
- Agent ignores constraints after many tool calls
- Behavior degrades with longer task files
- Works for small tasks, fails for complex ones

**Prevention strategy:**
- **Keep prompts concise:** Essential info only
- **Front-load critical constraints:** Important stuff at start AND end
- **Use progress file wisely:** Summary, not transcript
- **Monitor token usage:** Track per-iteration consumption
- **Fresh context IS the mitigation:** The Ralph Wiggum pattern exists for this reason

**Suggested phase:** Phase 2 (build command) - Prompt design and iteration structure.

**Source:** [Vectara: Awesome Agent Failures](https://github.com/vectara/awesome-agent-failures)

---

### Pitfall 10: Tool Hallucination / Wrong Tool Selection

**What goes wrong:** Agent calls non-existent tools. Passes invalid arguments. Selects wrong operation (DELETE vs ARCHIVE).

**Why it happens:** LLM generates plausible-sounding but incorrect tool calls. Schema mismatches between expected and actual APIs.

**Warning signs:**
- Claude CLI errors on tool execution
- Unexpected file operations
- Arguments that look reasonable but are wrong

**Prevention strategy:**
- **Validate tool outputs:** Don't trust, verify
- **Explicit tool constraints:** In prompt, list exact available operations
- **Confirmation for destructive ops:** Never auto-approve DELETE/overwrite
- **Parse and validate:** Check tool call structure before execution

**Suggested phase:** Phase 2 (build command) - Tool result handling.

**Source:** [Vectara: Awesome Agent Failures](https://github.com/vectara/awesome-agent-failures)

---

### Pitfall 11: No Circuit Breaker for Repeated Failures

**What goes wrong:** Agent keeps retrying failed operation. Error messages accumulate. Costs rise without progress.

**Why it happens:** Retries without backoff or limits. No distinction between transient and permanent failures. "Retry storms" at scale.

**Warning signs:**
- Same error appearing repeatedly in output
- Progress file shows many failed attempts
- Claude CLI exits with same error pattern

**Prevention strategy:**
- **Exponential backoff:** Increasing delays between retries
- **Max retry count:** N attempts then fail the task
- **Failure classification:** Transient (retry) vs permanent (escalate)
- **Circuit breaker:** After N failures, pause all operations for cooldown

**Suggested phase:** Phase 2 (build command) - Error handling in iteration loop.

**Source:** [Portkey: Retries, Fallbacks, and Circuit Breakers in LLM Apps](https://portkey.ai/blog/retries-fallbacks-and-circuit-breakers-in-llm-apps/)

---

## Configuration Pitfalls (TOML + CLI)

### Pitfall 12: Incorrect Configuration Precedence

**What goes wrong:** CLI flags don't override config file. Environment variables ignored. Confusing "where did this value come from?"

**Why it happens:** Merge order in configuration layering is wrong. Default values override explicitly-set file values. Can't distinguish "user set this" from "this is the default."

**Warning signs:**
- `--flag=value` seems to have no effect
- Config file values always used regardless of CLI
- Debugging requires reading multiple config sources

**Prevention strategy:**
- **Correct precedence (lowest to highest):**
  1. CLI defaults (clap defaults)
  2. Config file (TOML)
  3. Environment variables
  4. Explicit CLI arguments
- **Track value sources:** Use `clap::parser::ValueSource::DefaultValue`
- **Separate default from set:** Two parse passes - defaults vs explicit
- **Use Figment or similar:** Purpose-built for layered config

**Suggested phase:** Phase 1 (configuration) - Foundation that's hard to change.

**Source:** [clap + Figment integration example](https://gist.github.com/qknight/0ec68e64634e3eb7b9f9d00691f22443)

---

### Pitfall 13: Serde Default Value Gotchas

**What goes wrong:** Optional fields cause deserialization errors. Missing fields panic instead of using defaults. Partial config files rejected.

**Why it happens:** serde requires all fields present by default. `#[serde(default)]` needs to be on every optional field. `Option<T>` behaves differently than expected.

**Warning signs:**
- "missing field" errors on valid partial configs
- Users must specify every field even for defaults
- Adding new config field breaks existing config files

**Prevention strategy:**
```rust
#[derive(Deserialize)]
#[serde(default)] // Default for entire struct
struct Config {
    #[serde(default = "default_iterations")]
    max_iterations: u32,

    #[serde(default)]
    optional_feature: Option<String>, // None if missing
}

impl Default for Config { /* ... */ }
```

- Use `#[serde(default)]` on struct AND/OR individual fields
- Provide `Default` impl for the whole struct
- Test with minimal config files
- Test with empty config file

**Suggested phase:** Phase 1 (configuration) - Before users create config files.

**Source:** [Rust Users Forum: Serde TOML Deserialize Options](https://users.rust-lang.org/t/serde-toml-deserialize-with-on-options/77347)

---

### Pitfall 14: Unknown Fields Silent Failure

**What goes wrong:** User typos in config file are silently ignored. `max_iteratons` (typo) works but has no effect.

**Why it happens:** serde by default ignores unknown fields. User thinks config is applied but it's not.

**Warning signs:**
- Config changes seem to have no effect
- Users report "broken" features that work fine
- Debugging reveals config typos

**Prevention strategy:**
```rust
#[derive(Deserialize)]
#[serde(deny_unknown_fields)] // Reject typos
struct Config {
    // ...
}
```

- Use `#[serde(deny_unknown_fields)]` in production
- Provide helpful error messages listing valid fields
- Consider warning-not-error mode for forward compatibility

**Suggested phase:** Phase 1 (configuration) - User experience improvement.

---

## State Management Pitfalls (Progress File)

### Pitfall 15: Progress File Corruption on Crash

**What goes wrong:** Progress file is empty, truncated, or contains partial data after crash. Agent loses all progress.

**Why it happens:** Direct file write interrupted by crash. File opened for write, truncated, then crash before content written. No atomic write semantics.

**Warning signs:**
- Progress file empty after crash
- "Unexpected EOF" parsing progress file
- Data loss after power failure / kill -9

**Prevention strategy:**
- **Atomic write pattern:**
  1. Write to temporary file
  2. `fsync()` the temporary file
  3. Rename temp to target (atomic on POSIX)
- Use `safe-write` crate or implement pattern manually:
```rust
use std::fs;
use std::io::Write;

fn atomic_write(path: &Path, content: &str) -> io::Result<()> {
    let temp_path = path.with_extension("tmp");
    let mut file = fs::File::create(&temp_path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?; // fsync
    fs::rename(&temp_path, path)?; // atomic
    Ok(())
}
```

**Note:** Windows doesn't support atomic rename over existing file - need explicit delete first.

**Suggested phase:** Phase 1 (progress file) - Core to persistence integrity.

**Source:** [safe-write crate documentation](https://lib.rs/crates/safe-write)

---

### Pitfall 16: Concurrent Access to Progress File

**What goes wrong:** Two ralph instances corrupt each other's state. Race conditions cause lost updates.

**Why it happens:** Multiple processes or threads accessing same file without locking. Read-modify-write races.

**Warning signs:**
- Progress "jumps back" between runs
- Data from one run appears in another
- Inconsistent state after parallel execution

**Prevention strategy:**
- **File locking:** Use `flock()` or `fs2` crate
- **Single-instance enforcement:** PID file or lock file
- **Warn on concurrent access:** Detect and abort gracefully
```rust
use fs2::FileExt;

let file = File::open(progress_path)?;
file.try_lock_exclusive()?; // Fail if locked
// Now safe to operate
```

**Suggested phase:** Phase 2 (build command) - When persistence becomes critical.

---

### Pitfall 17: Progress Schema Evolution

**What goes wrong:** New ralph version can't read old progress file. Users lose progress on upgrade.

**Why it happens:** Schema changes without migration path. Strict parsing rejects old formats.

**Warning signs:**
- "Unknown field" or "missing field" after upgrade
- Users report progress loss after update
- Inability to add new features to progress format

**Prevention strategy:**
- **Version field in progress file:**
```toml
schema_version = 1

[task]
# ...
```
- **Migration functions:** `migrate_v1_to_v2()`
- **Lenient parsing for forward compatibility:** Unknown fields warn, don't fail
- **Test upgrade paths:** Include old format files in test suite

**Suggested phase:** Phase 2 (build command) - Before first release with persistent state.

---

## Summary: Phase-Mapped Pitfalls

| Phase | Critical Pitfalls | Must Address |
|-------|------------------|--------------|
| Phase 1 (Foundation) | Terminal panic corruption, Pipe deadlock, Zombie processes, Config precedence, Atomic writes | Before any real functionality |
| Phase 2 (Core Loop) | Infinite loop detection, Context truncation, Circuit breaker, Concurrent access, Schema evolution | Core Ralph loop implementation |
| Phase 3 (Polish) | Unicode rendering, Resize handling | Complex TUI features |

## Confidence Notes

| Area | Confidence | Source |
|------|------------|--------|
| TUI pitfalls | HIGH | Ratatui official documentation |
| Subprocess pitfalls | HIGH | Rust std library documentation |
| AI loop pitfalls | MEDIUM | Community sources, issue trackers |
| Configuration pitfalls | HIGH | Official serde/clap docs + community examples |
| State management pitfalls | HIGH | Known patterns, crate documentation |

---

## Sources

**Official Documentation:**
- [Ratatui Panic Hooks](https://ratatui.rs/recipes/apps/panic-hooks/)
- [Ratatui Alternate Screen](https://ratatui.rs/concepts/backends/alternate-screen/)
- [Ratatui Async Event Stream](https://ratatui.rs/tutorials/counter-async-app/async-event-stream/)
- [Rust std::process::Child](https://doc.rust-lang.org/std/process/struct.Child.html)

**Community Resources:**
- [Ratatui Forum: tokio spawn discussion](https://forum.ratatui.rs/t/understanding-tokio-spawn-and-tokio-spawn-blocking/74)
- [clap + Figment integration](https://gist.github.com/qknight/0ec68e64634e3eb7b9f9d00691f22443)
- [safe-write crate](https://lib.rs/crates/safe-write)

**AI Agent Patterns:**
- [Vectara: Awesome Agent Failures](https://github.com/vectara/awesome-agent-failures)
- [Galileo: Debug AI Agents](https://galileo.ai/blog/debug-ai-agents)
- [Portkey: LLM App Resilience Patterns](https://portkey.ai/blog/retries-fallbacks-and-circuit-breakers-in-llm-apps/)
- [Cursor AI Infinite Loop Issue](https://github.com/cursor/cursor/issues/3327)
