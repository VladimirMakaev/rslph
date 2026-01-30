---
phase: quick
plan: 009
type: execute
wave: 1
depends_on: []
files_modified:
  - src/subprocess/runner.rs
autonomous: true

must_haves:
  truths:
    - "When Claude CLI fails, rslph returns an error instead of hanging"
    - "Error message includes exit code and stderr content for debugging"
  artifacts:
    - path: "src/subprocess/runner.rs"
      provides: "Exit status checking in run_to_completion and run_with_channel"
      contains: "status.success()"
  key_links:
    - from: "run_to_completion"
      to: "RslphError::Subprocess"
      via: "exit status check"
      pattern: "if !status\\.success\\(\\)"
---

<objective>
Handle Claude CLI failures gracefully instead of hanging when the subprocess exits with non-zero status.

Purpose: When Claude CLI fails during planning or building, the current code ignores the exit status and returns Ok(output), causing downstream code to try parsing invalid/incomplete output and hang.

Output: Modified `run_to_completion` and `run_with_channel` methods that check exit status and return `RslphError::Subprocess` with exit code and stderr content when the process fails.
</objective>

<context>
@.planning/STATE.md
@src/subprocess/runner.rs
@src/error.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Check exit status in run_to_completion and run_with_channel</name>
  <files>src/subprocess/runner.rs</files>
  <action>
Modify both `run_to_completion` and `run_with_channel` to check the exit status after the process finishes:

1. In `run_to_completion` (around line 200-203):
   - Change `let _ = self.child.wait().await;` to `let status = self.child.wait().await?;`
   - After getting status, check `if !status.success()`
   - If failed, collect stderr lines from the output Vec (filter for `OutputLine::Stderr` variants)
   - Return `Err(RslphError::Subprocess(format!("Process exited with code {}: {}", exit_code, stderr_content)))`
   - Include the exit code (use `status.code().unwrap_or(-1)`) and stderr lines joined by newlines

2. In `run_with_channel` (around line 268-271):
   - Same pattern: change `let _ = self.child.wait().await;` to check status
   - Since output is streamed via channel and not collected, just report exit code
   - Return `Err(RslphError::Subprocess(format!("Process exited with code {}", exit_code)))`

Note: `run_with_timeout` wraps `run_to_completion` so it inherits the fix automatically.

Helper pattern for extracting stderr from output Vec:
```rust
let stderr_content: String = output
    .iter()
    .filter_map(|l| match l {
        OutputLine::Stderr(s) => Some(s.as_str()),
        _ => None,
    })
    .collect::<Vec<_>>()
    .join("\n");
```
  </action>
  <verify>
Run `cargo build` to ensure compilation succeeds.
Run `cargo test subprocess` to ensure existing tests pass.
  </verify>
  <done>
`run_to_completion` and `run_with_channel` return `Err(RslphError::Subprocess(...))` when process exits with non-zero status.
  </done>
</task>

<task type="auto">
  <name>Task 2: Add tests for failed subprocess handling</name>
  <files>src/subprocess/runner.rs</files>
  <action>
Add two new tests to the existing test module in `src/subprocess/runner.rs`:

1. `test_run_to_completion_returns_error_on_failure`:
   - Spawn a process that exits with error: `/bin/sh -c "echo error >&2; exit 1"`
   - Call `run_to_completion` with a CancellationToken
   - Assert result is `Err(RslphError::Subprocess(...))`
   - Assert error message contains "exit" and "1" (the exit code)
   - Assert error message contains "error" (the stderr output)

2. `test_run_with_channel_returns_error_on_failure`:
   - Spawn a process that exits with error: `/bin/sh -c "exit 42"`
   - Create mpsc channel and call `run_with_channel`
   - Assert result is `Err(RslphError::Subprocess(...))`
   - Assert error message contains "42" (the exit code)
  </action>
  <verify>
Run `cargo test subprocess` - all tests should pass including the new ones.
  </verify>
  <done>
Two new tests verify that subprocess failures are properly detected and reported with exit codes.
  </done>
</task>

</tasks>

<verification>
1. `cargo build` - compiles without errors
2. `cargo test subprocess` - all tests pass
3. `cargo clippy` - no new warnings
</verification>

<success_criteria>
- Processes that exit with non-zero status cause `RslphError::Subprocess` errors
- Error messages include exit code for debugging
- Error messages include stderr content where available (in `run_to_completion`)
- All existing tests continue to pass
- Two new tests verify the failure handling behavior
</success_criteria>

<output>
After completion, create `.planning/quick/009-handle-claude-cli-failures-instead-of-ha/009-SUMMARY.md`
</output>
