---
phase: 07-e2e-testing-framework
plan: 03
subsystem: testing
tags: [e2e, fake-claude, tool-calls, edge-cases, scenario-builder]

# Dependency graph
requires:
  - phase: 07-01
    provides: Fake Claude binary and ScenarioBuilder base
provides:
  - Tool call simulation (Read, Write, Edit, Bash)
  - Edge case testing (delay, crash, malformed output, exit code)
  - Multi-invocation scenario support
  - Comprehensive unit tests
affects: [07-04]

# Tech tracking
tech-stack:
  added: []
  patterns: [fluent-builder-api, edge-case-simulation]

key-files:
  created:
    - tests/e2e/scenario_tests.rs
  modified:
    - tests/fake_claude_lib/stream_json.rs
    - tests/fake_claude_lib/scenario.rs
    - tests/fake_claude_lib/config.rs
    - tests/fake_claude.rs
    - tests/e2e/main.rs

key-decisions:
  - "Tests placed in e2e test crate rather than fake_claude binary module (harness=false prevents test discovery)"
  - "Added with_delay alias for API consistency with plan specification"
  - "Used path attribute to include fake_claude_lib in e2e tests"

patterns-established:
  - "uses_* methods for common tool call simulation"
  - "send_raw for malformed output testing"
  - "crash_after for process crash simulation"

# Metrics
duration: 5min
completed: 2026-01-19
---

# Phase 7 Plan 3: Extended Scenario Builder Summary

**Tool call simulation, edge cases, and multi-invocation support for fake Claude testing**

## Performance

- **Duration:** 5 min
- **Started:** 2026-01-19T22:03:51Z
- **Completed:** 2026-01-19T22:08:56Z
- **Tasks:** 4
- **Files modified:** 6

## Accomplishments
- Tool call event generation with unique IDs (toolu_XXXX format)
- Tool call helper methods: uses_read, uses_write, uses_edit, uses_bash, uses_tool
- Edge case configuration: with_delay, crash_after, send_raw, with_exit_code
- Fake Claude binary handles raw_lines and exit_code
- 9 comprehensive unit tests for ScenarioBuilder

## Task Commits

Each task was committed atomically:

1. **Task 1: Add tool_use event generation to stream_json** - `9d29f99` (feat)
2. **Task 2: Add tool call helpers to ScenarioBuilder** - `59a6736` (feat)
3. **Task 3: Add edge case configuration methods** - `bea3639` (feat)
4. **Task 4: Add unit tests for tool calls and edge cases** - `12e53d4` (test)

## Files Created/Modified
- `tests/fake_claude_lib/stream_json.rs` - Added TOOL_ID_COUNTER, ContentBlockOutput constructors, StreamEventOutput::tool_use
- `tests/fake_claude_lib/scenario.rs` - Added uses_*, send_raw, with_exit_code, with_delay methods
- `tests/fake_claude_lib/config.rs` - Added raw_lines and exit_code fields to InvocationConfig
- `tests/fake_claude.rs` - Updated binary to handle raw_lines and exit_code
- `tests/e2e/main.rs` - Added fake_claude_lib module and scenario_tests
- `tests/e2e/scenario_tests.rs` - 9 comprehensive unit tests

## Decisions Made
- **Test location:** Placed unit tests in e2e test crate because fake_claude binary uses harness=false which prevents test discovery
- **API naming:** Added with_delay as alias for with_delay_ms to match plan specification
- **Module inclusion:** Used #[path] attribute to include fake_claude_lib in e2e tests rather than duplicating code

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- Initial attempt to run tests in fake_claude.rs failed because harness=false prevents test discovery
- Resolved by moving tests to e2e test crate with path attribute for module inclusion

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Complete scenario builder API ready for E2E test development
- Tool call simulation enables testing of Claude tool use handling
- Edge case testing enables resilience testing (timeouts, crashes, malformed output)
- Plan 04 can now implement full E2E tests using this infrastructure

---
*Phase: 07-e2e-testing-framework*
*Completed: 2026-01-19*
