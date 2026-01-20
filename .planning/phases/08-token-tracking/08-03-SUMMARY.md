---
phase: 08-token-tracking
plan: 03
subsystem: testing
tags: [e2e-testing, tui-testing, fake-claude, tokens, insta-snapshots]

requires:
  - 08-01: TokenUsage struct, format_tokens()
  - 08-02: Status bar token display, AppEvent::TokenUsage
provides:
  - ScenarioBuilder.with_token_usage() method
  - E2E tests for token tracking with fake Claude
  - TUI snapshot tests for token display
affects:
  - Future phases testing token-related features

tech-stack:
  added: []
  patterns:
    - Configurable token values in fake Claude infrastructure
    - TUI snapshot testing with deterministic token values

key-files:
  created:
    - tests/e2e/test_token_tracking.rs
    - tests/e2e/snapshots/e2e__tui_tests__status_bar_displays_tokens.snap
    - tests/e2e/snapshots/e2e__tui_tests__status_bar_zero_tokens.snap
    - tests/e2e/snapshots/e2e__tui_tests__status_bar_large_tokens.snap
    - tests/e2e/snapshots/e2e__tui_tests__token_values_across_iterations.snap
  modified:
    - tests/fake_claude_lib/config.rs
    - tests/fake_claude_lib/mod.rs
    - tests/fake_claude_lib/scenario.rs
    - tests/fake_claude_lib/stream_json.rs
    - tests/e2e/main.rs
    - tests/e2e/tui_tests.rs

decisions:
  - id: token-config-location
    choice: "TokenConfig in fake_claude_lib/config.rs with ScenarioBuilder integration"
    reason: "Fluent API matches existing ScenarioBuilder pattern"

metrics:
  duration: "5m 27s"
  completed: "2026-01-20"
---

# Phase 8 Plan 3: Token Tracking Tests Summary

Extended fake Claude infrastructure with token configuration and added E2E/TUI tests for token tracking. ScenarioBuilder now supports `with_token_usage(input, output, cache_create, cache_read)` for deterministic test values.

## Changes Made

### Task 1: Add token configuration to ScenarioBuilder (47c764e)

Added token configuration support to the fake Claude test infrastructure:

1. **TokenConfig struct** in `tests/fake_claude_lib/config.rs`:
   - `input_tokens`, `output_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`
   - Default values: 100 input, 50 output, 0 cache tokens

2. **ScenarioBuilder.with_token_usage()** method:
   ```rust
   ScenarioBuilder::new()
       .with_token_usage(5000, 1500, 2000, 1000)
       .respond_with_text("Response with custom tokens")
       .build();
   ```

3. **StreamEventOutput.assistant_text_with_tokens()** and related methods:
   - All response methods respect configured token values
   - `tool_use_with_tokens()`, `result_with_tokens()`, `assistant_with_blocks_and_tokens()`

4. **UsageOutput.from_config()** helper for converting TokenConfig to UsageOutput

### Task 2: Create E2E tests for token tracking (69581ad)

Added 4 E2E tests in `tests/e2e/test_token_tracking.rs`:

| Test | Purpose |
|------|---------|
| `test_fake_claude_with_custom_tokens` | Verify token config appears in fake Claude output |
| `test_fake_claude_multi_invocation_tokens` | Different tokens per invocation |
| `test_rslph_build_with_token_tracking` | Full build with configured tokens |
| `test_fake_claude_tool_use_with_tokens` | Tool use respects token config |

### Task 3: Create TUI snapshot tests for token display (59cbb6b)

Added 4 TUI snapshot tests in `tests/e2e/tui_tests.rs`:

| Test | Snapshot Content |
|------|-----------------|
| `test_status_bar_displays_tokens` | "In: 5.2k \| Out: 10.9k \| CacheW: 2.1k \| CacheR: 1.5k" |
| `test_status_bar_zero_tokens` | "In: 0 \| Out: 0 \| CacheW: 0 \| CacheR: 0" |
| `test_status_bar_large_tokens` | Million-scale values (1.2M, 567.9k) |
| `test_token_values_across_iterations` | Token updates between iterations |

Updated 7 existing TUI snapshots to reflect status bar token display.

## Test Results

```
Library tests: 168 passed
E2E tests: 64 passed (including 8 new token-related tests)
```

## Deviations from Plan

None - plan executed exactly as written.

## Next Phase Readiness

Token tracking implementation complete:
- Core infrastructure (08-01)
- Status bar display (08-02)
- E2E and TUI tests (08-03)

Ready to proceed with:
- 08-04: Plan command token tracking (if planned)
- Phase 9: Eval Foundation
