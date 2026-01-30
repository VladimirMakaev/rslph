---
phase: quick
plan: 009
subsystem: subprocess
tags: [error-handling, subprocess, cli]

dependency-graph:
  requires: []
  provides:
    - subprocess-failure-detection
    - exit-status-checking
  affects: []

tech-stack:
  added: []
  patterns:
    - exit-status-checking
    - stderr-capture-in-errors

file-tracking:
  key-files:
    created: []
    modified:
      - src/subprocess/runner.rs

metrics:
  duration: 3m
  completed: 2026-01-30
---

# Quick Task 009: Handle Claude CLI Failures Instead of Hanging

**One-liner:** Exit status checking in run_to_completion and run_with_channel returns RslphError::Subprocess with exit code and stderr on non-zero exit.

## What Was Built

Modified the subprocess runner to detect and report Claude CLI failures instead of silently ignoring non-zero exit statuses and hanging downstream.

### Task Commits

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Check exit status in run_to_completion and run_with_channel | 98e39ea | src/subprocess/runner.rs |
| 2 | Add tests for failed subprocess handling | 691a7b8 | src/subprocess/runner.rs |

## Technical Details

### Changes to run_to_completion

Before:
```rust
let _ = self.child.wait().await;
Ok(output)
```

After:
```rust
let status = self.child.wait().await
    .map_err(|e| RslphError::Subprocess(e.to_string()))?;

if !status.success() {
    let exit_code = status.code().unwrap_or(-1);
    let stderr_content: String = output
        .iter()
        .filter_map(|l| match l {
            OutputLine::Stderr(s) => Some(s.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    return Err(RslphError::Subprocess(format!(
        "Process exited with code {}: {}",
        exit_code, stderr_content
    )));
}
Ok(output)
```

### Changes to run_with_channel

Before:
```rust
let _ = self.child.wait().await;
Ok(())
```

After:
```rust
let status = self.child.wait().await
    .map_err(|e| RslphError::Subprocess(e.to_string()))?;

if !status.success() {
    let exit_code = status.code().unwrap_or(-1);
    return Err(RslphError::Subprocess(format!(
        "Process exited with code {}",
        exit_code
    )));
}
Ok(())
```

Note: `run_with_channel` doesn't include stderr content in the error message because output is streamed to the channel rather than collected.

### Test Coverage

Two new tests were added:

1. `test_run_to_completion_returns_error_on_failure` - Verifies that:
   - Process with exit code 1 returns `Err(RslphError::Subprocess(...))`
   - Error message contains the exit code
   - Error message contains stderr output

2. `test_run_with_channel_returns_error_on_failure` - Verifies that:
   - Process with exit code 42 returns `Err(RslphError::Subprocess(...))`
   - Error message contains the exit code

## Deviations from Plan

None - plan executed exactly as written.

## Verification

- `cargo build` - Compiles without errors
- `cargo test subprocess` - All 39 tests pass (including 2 new)
- `cargo clippy` - No warnings
