---
status: diagnosed
phase: 08-token-tracking
source: [08-01-SUMMARY.md, 08-02-SUMMARY.md, 08-03-SUMMARY.md]
started: 2026-01-20T02:40:00Z
updated: 2026-01-20T02:50:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Token Display in TUI Status Bar
expected: Run `cargo run -- build` on a project. Status bar shows "In: X | Out: Y | CacheW: Z | CacheR: W" during execution.
result: issue
reported: "I see the header but it refreshes every time. You need to track running total. Add a TUI test for this too"
severity: major

### 2. Token Reporting in Plan Command
expected: Run `cargo run -- plan` on a project. After completion, see "Tokens used: In: X | Out: Y | CacheW: Z | CacheR: W" output.
result: pass

### 3. Human-Readable Token Formatting
expected: During build or plan, token counts with thousands show "k" suffix (e.g., "In: 5.2k") and millions show "M" suffix.
result: issue
reported: "If you don't have a TUI test for this you should build one"
severity: minor

### 4. Token Accumulation Across Iterations
expected: During multi-iteration build, token counts in status bar increase with each iteration (not reset).
result: issue
reported: "The numbers are jumping I don't think they are accumulated. Write a test for this yourself and check the snapshot"
severity: major

## Summary

total: 4
passed: 1
issues: 3
pending: 0
skipped: 0

## Gaps

- truth: "Token counts accumulate as running total in TUI status bar"
  status: failed
  reason: "User reported: I see the header but it refreshes every time. You need to track running total. Add a TUI test for this too"
  severity: major
  test: 1
  root_cause: "In src/tui/app.rs:439-442, TokenUsage handler uses = instead of += so each event overwrites previous values"
  artifacts:
    - path: "src/tui/app.rs"
      issue: "AppEvent::TokenUsage handler uses assignment instead of accumulation"
      lines: "439-442"
  missing:
    - "Change = to += for token accumulation"
    - "Add TUI test for token accumulation"
  debug_session: ""

- truth: "Human-readable token formatting has TUI test coverage"
  status: failed
  reason: "User reported: If you don't have a TUI test for this you should build one"
  severity: minor
  test: 3
  root_cause: "Missing TUI snapshot test that verifies k/M suffix formatting in status bar"
  artifacts: []
  missing:
    - "Add TUI snapshot test for format_tokens with large values"
  debug_session: ""

- truth: "Token counts accumulate across iterations without jumping/resetting"
  status: failed
  reason: "User reported: The numbers are jumping I don't think they are accumulated. Write a test for this yourself and check the snapshot"
  severity: major
  test: 4
  root_cause: "Same as Test 1 - assignment instead of accumulation in AppEvent::TokenUsage handler"
  artifacts:
    - path: "src/tui/app.rs"
      issue: "AppEvent::TokenUsage handler uses assignment instead of accumulation"
      lines: "439-442"
  missing:
    - "Change = to += for token accumulation"
    - "Add TUI snapshot test for multi-iteration token accumulation"
  debug_session: ""
