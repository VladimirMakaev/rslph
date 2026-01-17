# Research Summary

**Project:** Ralph (rslph) - Rust CLI/TUI Autonomous Coding Agent
**Synthesized:** 2026-01-17

---

## Stack Recommendation

**Core stack (HIGH confidence):**

| Component | Technology | Rationale |
|-----------|------------|-----------|
| TUI Framework | ratatui 0.30 | De facto Rust TUI standard since 2023; immediate-mode rendering fits async event loop |
| Terminal Backend | crossterm 0.29 | Pure Rust, cross-platform; event-stream feature enables async keyboard handling |
| Async Runtime | tokio 1.49 | Industry standard; native process module critical for Claude CLI subprocess streaming |
| CLI Parser | clap 4.5 | Derive macros, subcommand support, env variable fallbacks |
| Configuration | figment + toml + serde | Layered config merging (file < env < CLI) with type-safe deserialization |
| Error Handling | color-eyre + thiserror | Colorized error reports for TUI apps; domain error definitions via derive |
| Logging | tracing | Async-aware structured logging; file output keeps terminal free for TUI |
| Paths | directories 6.0 | Platform-appropriate config/data directories (XDG on Linux, proper macOS/Windows paths) |

**Key version constraints:**
- tokio must include `process` feature for subprocess management
- crossterm must include `event-stream` feature for async input
- ratatui 0.30 is current stable with all needed widgets

---

## Table Stakes Features

These are non-negotiable for user adoption:

**Core Loop Execution:**
- Autonomous iteration loop with configurable max iterations
- Progress persistence (resume after interruption)
- Completion detection (RALPH_DONE marker or similar)
- Max iteration limits (safety against runaway costs)
- Git auto-commit per iteration (rollback safety)

**Configuration:**
- TOML config file support
- CLI flag overrides (CLI > env > file > defaults)
- Environment variables (RALPH_* prefix)
- Custom prompt paths (PROMPT_plan.md, PROMPT_build.md)

**User Experience:**
- Real-time output streaming from Claude subprocess
- Status display (iteration count, current task)
- Ctrl+C graceful shutdown (save state, clean exit)
- Single iteration mode (--once for human review)

**Task Management:**
- Progress file parsing (markdown format with checkboxes)
- Priority ordering (work on highest priority first)
- Acceptance criteria per task

---

## Key Differentiators

Features that set Ralph apart from simpler implementations:

**TUI Interface (HIGH value, HIGH effort):**
- Rich TUI with status bar, context usage, scrollable output
- Collapsible conversation threads for long runs
- Keyboard navigation (vim-like bindings)
- Multi-pane layout (tasks, output, progress)

**Advanced Loop Control:**
- Verification integration (run tests between iterations)
- Circuit breaker (stop after repeated failures)
- Sleep intervals (rate limiting, cost management)
- Checkpoint/rollback to specific iterations

**Lifecycle Hooks:**
- onComplete hook for notifications
- Configurable notify script with context

---

## Architecture Highlights

**Pattern:** Component Architecture with Message Passing (TEA-inspired)

**Core Design Decisions:**

1. **Async-first subprocess management:** Claude CLI streams output continuously while TUI remains responsive. tokio::process::Command with piped stdout/stderr, streamed line-by-line through mpsc channels.

2. **Event coordination via select!:** Main loop uses `tokio::select!` to multiplex terminal keyboard input, subprocess output events, and render ticks (~60fps interval).

3. **Single owner of state:** One task owns AppState, receives events through channels. No shared mutable state with Arc/Mutex.

4. **Atomic progress file writes:** Write to temp file, fsync, rename. Critical for crash safety.

**Component Boundaries:**
```
CLI Parser -> Config Manager -> App Core
                                   |
                    +----------+---+---+----------+
                    |          |       |          |
              Subprocess   Progress    TUI    Prompts
               Manager       File    Renderer  (embedded)
```

**Module Structure:**
- `cli/` - clap definitions, command dispatch
- `config/` - TOML loading, type definitions
- `app/` - state machine, actions, event loop
- `subprocess/` - spawn, stream, events
- `progress/` - parse/write progress.md
- `tui/` - terminal, renderer, widgets

---

## Critical Pitfalls to Avoid

**Phase 1 (Foundation) - Address Before Building:**

| Pitfall | Consequence | Prevention |
|---------|-------------|------------|
| Terminal state corruption on panic | User terminal unusable | Custom panic hook restores terminal BEFORE any TUI code |
| Pipe buffer deadlock | Hang when Claude produces large output | Stream output concurrently, never wait-then-read |
| Zombie process accumulation | Resource exhaustion over iterations | Always wait() on every Child, use kill_on_drop |
| Config precedence wrong | CLI flags ignored | Correct order: defaults < file < env < explicit CLI |
| Progress file corruption on crash | Lost progress | Atomic write: temp file -> fsync -> rename |

**Phase 2 (Core Loop) - Address During Implementation:**

| Pitfall | Consequence | Prevention |
|---------|-------------|------------|
| Infinite loop / stuck agent | Token costs spiral, no progress | Hard iteration limits, loop detection, progress assertions |
| No circuit breaker | Repeated failures waste resources | Max retry count, exponential backoff, failure classification |
| Concurrent access to progress file | Corrupted state from parallel runs | File locking with flock/fs2, single-instance enforcement |

**Phase 3 (Polish) - Address When Relevant:**

| Pitfall | Consequence | Prevention |
|---------|-------------|------------|
| Unicode width miscalculation | Layout corruption with CJK/emoji | Use unicode-width crate, test with non-ASCII |
| Resize event race conditions | Crash or garbled output | Handle Resize events explicitly, guard against zero dimensions |

---

## Recommended Build Order

### Phase 1: Foundation (No TUI)

**Goal:** Subprocess spawning and config work correctly without any visual interface.

**Order:** Config -> CLI -> Progress -> Subprocess

1. **Config types + loader**
   - AppConfig struct with serde(default)
   - TOML loading from standard paths
   - Validation (paths exist, etc.)
   - *Avoid:* Serde default gotchas, unknown field silent failure

2. **CLI parser**
   - clap derive structs
   - plan/build subcommands
   - Config merging (CLI overrides file)
   - *Avoid:* Config precedence errors

3. **Progress file parser/writer**
   - Parse markdown format
   - Extract task checkboxes
   - Atomic write implementation
   - *Avoid:* Crash corruption, schema rigidity

4. **Subprocess manager**
   - Spawn Claude CLI with piped output
   - Stream stdout/stderr to channel
   - Handle exit codes
   - *Avoid:* Pipe deadlock, zombie processes, stdin not closed

**Validation:** Can run `ralph build` headlessly, spawn Claude, capture output, update progress file.

**Research flag:** Standard patterns, no additional research needed.

### Phase 2: App Core (State Machine + Build Loop)

**Goal:** Complete iteration loop logic, completion detection, iteration limits.

**Order:** State -> Actions -> Runner -> Iteration Loop

5. **App state definitions**
   - AppState, AppMode enums
   - Task tracking
   - Output buffer (ring buffer, bounded)

6. **Actions and state transitions**
   - Action enum (Quit, NextIteration, etc.)
   - Reducer pattern for state updates
   - Completion detection (RALPH_DONE marker)

7. **Event loop runner**
   - tokio::select! coordination
   - Subprocess event handling
   - Iteration control logic
   - *Avoid:* Infinite loops, no circuit breaker

8. **Build command implementation**
   - Read progress file
   - Find next incomplete task
   - Spawn Claude with context
   - Update progress on completion
   - *Avoid:* Context truncation, tool hallucination

**Validation:** Can run full iteration loop, auto-commit, detect completion, respect max iterations.

**Research flag:** Consider `/gsd:research-phase` for completion detection patterns if complex.

### Phase 3: TUI (Visual Layer)

**Goal:** Professional terminal interface with live output streaming.

**Order:** Terminal -> Widgets -> Renderer -> Integration

9. **Terminal setup**
   - Raw mode, alternate screen
   - Panic hook for cleanup
   - *Avoid:* Terminal state corruption

10. **Individual widgets**
    - Status bar (iteration, task, model)
    - Progress bar (context usage)
    - Output view (scrollable)
    - Thread list (collapsible)

11. **Main renderer**
    - Layout composition
    - Render from AppState
    - ~60fps render interval

12. **Event integration**
    - Keyboard handling in select! loop
    - Scroll, toggle, quit actions
    - *Avoid:* Async blocking, resize race conditions

**Validation:** Full TUI with live output, keyboard navigation, graceful quit.

**Research flag:** Standard ratatui patterns, but complex - reference async-template.

### Phase 4: Polish

**Goal:** Embedded prompts, notifications, error refinement.

13. **Embedded prompts**
    - include_str! for PROMPT_plan.md, PROMPT_build.md
    - Override paths in config

14. **Notification system**
    - notify_script execution on completion
    - Pass context (success/failure, task count)

15. **Error handling refinement**
    - Replace unwraps
    - User-friendly messages
    - color-eyre integration

**Research flag:** No additional research needed.

---

## Open Questions

Questions to resolve during requirements phase:

1. **Progress file format:** Research covered markdown with checkboxes. Is a structured format (TOML/JSON) better for programmatic updates? Trade-off: human readability vs parsing reliability.

2. **Completion detection specifics:** What exact marker does Ralph look for? RALPH_DONE? `<promise>COMPLETE</promise>`? Configurable pattern?

3. **Git integration scope:** Auto-commit only? Auto-stage? Commit message format? Should Ralph create branches?

4. **Multi-agent support:** Research identified parallel agents as a differentiator but HIGH complexity. Is this v1 scope or future?

5. **Claude CLI invocation details:** What exact arguments? `claude --prompt FILE`? `claude --print`? How does Ralph pass context?

6. **Verification integration:** Run tests automatically between iterations? Configurable command? Parse test output?

7. **Context usage tracking:** How to obtain token count from Claude CLI for progress bar? API response? Estimate from output length?

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All technologies verified via official docs; canonical 2025/2026 Rust CLI/TUI stack |
| Features | HIGH | Cross-referenced multiple autonomous coding tools; clear table stakes vs differentiators |
| Architecture | HIGH | Patterns from ratatui async-template, tokio best practices |
| Pitfalls | HIGH for TUI/subprocess (official docs), MEDIUM for AI agent patterns (community sources) |

**Gaps:**
- Claude CLI exact invocation pattern not documented in research
- Token/cost tracking method unclear
- Verification integration specifics TBD

---

## Sources

**Stack Research:**
- [ratatui 0.30 docs](https://docs.rs/ratatui/)
- [crossterm 0.29 docs](https://docs.rs/crossterm/)
- [tokio 1.49 docs](https://docs.rs/tokio/)
- [clap 4.5 docs](https://docs.rs/clap/)
- [figment docs](https://docs.rs/figment/)
- [ratatui async-template](https://github.com/ratatui/async-template)

**Feature Research:**
- [kylemclaren/ralph reference implementation](https://github.com/kylemclaren/ralph)
- [Ralph Wiggum Loop Gist](https://gist.github.com/Mburdo/ce99c9b08601aaf771efaabf1260d4c0)
- [Aider documentation](https://aider.chat/)
- [OpenCode TUI patterns](https://github.com/opencode-ai/opencode)

**Architecture Research:**
- [Ratatui Application Patterns](https://ratatui.rs/concepts/application-patterns/)
- [Tokio Channels Tutorial](https://tokio.rs/tokio/tutorial/channels)
- [tokio-process-stream](https://lib.rs/crates/tokio-process-stream)

**Pitfalls Research:**
- [Ratatui Panic Hooks](https://ratatui.rs/recipes/apps/panic-hooks/)
- [Rust std::process::Child docs](https://doc.rust-lang.org/std/process/struct.Child.html)
- [Vectara: Awesome Agent Failures](https://github.com/vectara/awesome-agent-failures)
- [Portkey: LLM App Resilience](https://portkey.ai/blog/retries-fallbacks-and-circuit-breakers-in-llm-apps/)
