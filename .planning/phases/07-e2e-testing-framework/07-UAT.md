---
status: testing
phase: 07-e2e-testing-framework
source: [07-01-SUMMARY.md, 07-02-SUMMARY.md, 07-03-SUMMARY.md, 07-04-SUMMARY.md, 07-05-SUMMARY.md]
started: 2026-01-19T22:30:00Z
updated: 2026-01-19T22:30:00Z
---

## Current Test

number: 1
name: Run E2E Tests
expected: |
  Run `cargo test --test e2e` and all 48 tests pass without failures.
  Output shows tests for fake Claude, workspace fixtures, and rslph integration.
awaiting: user response

## Tests

### 1. Run E2E Tests
expected: Run `cargo test --test e2e` and all 48 tests pass without failures. Output shows tests for fake Claude, workspace fixtures, and rslph integration.
result: [pending]

### 2. ScenarioBuilder Creates Valid JSON
expected: Run `cargo test test_scenario_simple_text_response --test e2e -- --nocapture`. Test passes and output shows valid stream-json format with message_start, content_block_start, content_block_delta, content_block_stop, message_stop events.
result: [pending]

### 3. Fake Claude Binary Executes
expected: Build the fake claude binary with `cargo build --tests`, then verify binary exists at `target/debug/deps/fake_claude*`. The binary should be executable.
result: [pending]

### 4. Multi-Iteration Scenario
expected: Run `cargo test test_multi_invocation_scenario --test e2e -- --nocapture`. Test passes showing invocation counter increments across calls (invocation 1, 2, etc. in output).
result: [pending]

### 5. Edge Case - Crash Simulation
expected: Run `cargo test test_crash_after_events --test e2e -- --nocapture`. Test passes showing fake Claude exits early with code 1 after configured number of events.
result: [pending]

### 6. WorkspaceBuilder Creates Structure
expected: Run `cargo test test_workspace_creates_temp_dir --test e2e -- --nocapture`. Test passes showing temp workspace created with PROGRESS.md, git repo initialized, and cleanup on drop.
result: [pending]

### 7. Rslph Integration - Basic Run
expected: Run `cargo test test_rslph_build_single_iteration_success --test e2e -- --nocapture`. Test passes showing rslph binary invoked with fake Claude and completes without error.
result: [pending]

### 8. Rslph Integration - Multi Iteration
expected: Run `cargo test test_rslph_build_multi_iteration_invokes_claude_multiple_times --test e2e -- --nocapture`. Test passes showing rslph invokes fake Claude multiple times in loop.
result: [pending]

### 9. Tool Call Simulation
expected: Run `cargo test test_tool_call_events --test e2e -- --nocapture`. Test passes showing tool_use events for Read, Write, Edit, Bash tools with proper JSON structure.
result: [pending]

### 10. Assertion Helpers Work
expected: Run `cargo test test_assert_task_complete --test e2e -- --nocapture`. Test passes showing assertion helper correctly identifies complete vs incomplete tasks in progress file.
result: [pending]

## Summary

total: 10
passed: 0
issues: 0
pending: 10
skipped: 0

## Gaps

[none yet]
