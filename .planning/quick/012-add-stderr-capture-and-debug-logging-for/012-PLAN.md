---
phase: quick
plan: 012
type: execute
wave: 1
depends_on: []
files_modified:
  - src/tui/event.rs
  - src/tui/app.rs
  - src/build/iteration.rs
autonomous: true

must_haves:
  truths:
    - "stderr output from Claude CLI is displayed in the TUI"
    - "Debug logging shows subprocess lifecycle events"
    - "When Claude CLI outputs to stderr, user can see those messages"
  artifacts:
    - path: "src/tui/event.rs"
      provides: "SubprocessEvent::Stderr variant"
      contains: "Stderr"
    - path: "src/build/iteration.rs"
      provides: "stderr handling in output processing"
      contains: "OutputLine::Stderr"
  key_links:
    - from: "src/build/iteration.rs"
      to: "SubprocessEvent::Stderr"
      via: "tui_tx.send"
      pattern: "Stderr"
---

<objective>
Add stderr capture and debug logging for Claude CLI subprocess communication.

Purpose: When rslph build runs, stderr from Claude CLI is currently ignored, making it impossible to diagnose issues when the subprocess gets stuck or encounters errors. This task surfaces stderr output to the user and adds debug logging for subprocess lifecycle.

Output: Modified event types and iteration logic to capture and display stderr output.
</objective>

<execution_context>
@/Users/vmakaev/.claude/get-shit-done/workflows/execute-plan.md
@/Users/vmakaev/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@src/subprocess/runner.rs (ClaudeRunner with OutputLine handling)
@src/subprocess/output.rs (OutputLine::Stdout and OutputLine::Stderr)
@src/build/iteration.rs (run_single_iteration ignoring stderr)
@src/tui/event.rs (SubprocessEvent variants)
@src/tui/app.rs (AppEvent handling)
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add Stderr event variant and forward stderr output</name>
  <files>
    src/tui/event.rs
    src/tui/app.rs
    src/build/iteration.rs
  </files>
  <action>
1. In `src/tui/event.rs`:
   - Add `SubprocessEvent::Stderr(String)` variant for stderr output from Claude CLI
   - Add conversion in `From<SubprocessEvent> for AppEvent` that maps `Stderr(s)` to `AppEvent::LogMessage(format!("[stderr] {}", s))` (prefix helps user distinguish stderr from normal logs)

2. In `src/tui/app.rs`:
   - No changes needed - LogMessage already handled and displayed

3. In `src/build/iteration.rs`, update both streaming and non-streaming modes:
   - In the streaming mode section (around line 205-211), add handling for `OutputLine::Stderr`:
     ```rust
     if let OutputLine::Stdout(s) = &line {
         // existing stdout handling
     } else if let OutputLine::Stderr(s) = &line {
         let _ = tui_tx_clone.send(SubprocessEvent::Stderr(s.clone()));
     }
     ```
   - In the non-streaming mode section (around line 237-241), add similar stderr handling:
     ```rust
     for line in &output {
         if let OutputLine::Stdout(s) = line {
             stream_response.process_line(s);
         } else if let OutputLine::Stderr(s) = line {
             // Log stderr in non-TUI mode
             ctx.log(&format!("[stderr] {}", s));
         }
     }
     ```
   - Add debug trace logging at key subprocess lifecycle points:
     - After receiving each output line in streaming mode: `ctx.log(&format!("[DEBUG] Received line type: {:?}", &line));` (only if --verbose is set, but since ctx.log respects verbosity, this is fine)
     - When subprocess channel closes: add a trace log

4. Add unit test in `event.rs` tests section for the new Stderr variant conversion.
  </action>
  <verify>
    - `cargo build` compiles without errors
    - `cargo test` passes all tests including new test for Stderr conversion
    - `cargo clippy` has no new warnings
  </verify>
  <done>
    - SubprocessEvent::Stderr variant exists and converts to LogMessage
    - OutputLine::Stderr lines are forwarded to TUI in streaming mode
    - OutputLine::Stderr lines are logged in non-streaming mode
    - Debug logging shows subprocess lifecycle
  </done>
</task>

<task type="auto">
  <name>Task 2: Add enhanced debug logging for subprocess communication</name>
  <files>
    src/build/iteration.rs
  </files>
  <action>
Add more comprehensive debug logging to trace subprocess communication:

1. At the start of streaming mode processing (before the while loop), log:
   ```rust
   ctx.log("[TRACE] Starting subprocess output streaming");
   ```

2. Inside the streaming loop, after receiving any line:
   ```rust
   // Only log line type, not full content (could be huge)
   match &line {
       OutputLine::Stdout(_) => ctx.log("[TRACE] Received stdout line"),
       OutputLine::Stderr(s) => ctx.log(&format!("[TRACE] Received stderr: {}", s)),
   }
   ```

3. After the streaming loop completes (when channel closes):
   ```rust
   ctx.log("[TRACE] Subprocess output stream ended");
   ```

4. In non-streaming mode, add similar logging:
   - Before processing: `ctx.log("[TRACE] Processing subprocess output (non-streaming)");`
   - After processing: `ctx.log(&format!("[TRACE] Processed {} output lines", output.len()));`

5. Update the existing spawn trace to include the full command being run:
   ```rust
   ctx.log(&format!(
       "[TRACE] Iteration {}: Spawning Claude: {} {}",
       ctx.current_iteration,
       ctx.config.claude_cmd.command,
       combined_args.join(" ")
   ));
   ```
  </action>
  <verify>
    - `cargo build` compiles without errors
    - `cargo test` passes
    - Run `rslph build --verbose` on a test project and verify trace logs appear
  </verify>
  <done>
    - Debug trace logs show subprocess spawn command
    - Debug trace logs show when streaming starts and ends
    - Stderr lines are logged with their content for debugging
  </done>
</task>

</tasks>

<verification>
- `cargo build --release` succeeds
- `cargo test` all tests pass
- `cargo clippy -- -D warnings` no warnings
- Run `rslph build --verbose` with a test case that produces stderr and verify output is visible
</verification>

<success_criteria>
1. When Claude CLI outputs to stderr, those messages appear in the TUI with [stderr] prefix
2. Debug logging shows the full subprocess command being executed
3. Debug logging shows when subprocess output stream starts and ends
4. All existing tests continue to pass
</success_criteria>

<output>
After completion, create `.planning/quick/012-add-stderr-capture-and-debug-logging-for/012-SUMMARY.md`
</output>
