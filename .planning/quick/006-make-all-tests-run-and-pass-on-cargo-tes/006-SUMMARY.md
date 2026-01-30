---
phase: quick-006
plan: 01
subsystem: testing
tags: [tests, config, figment, env-vars]

# Dependency graph
requires:
  - phase: quick-005
    provides: "RSLPH_CLAUDE_CMD env var support"
provides:
  - "All 294 unit tests passing"
  - "Config env var filtering for skipped fields"
  - "Test isolation preventing mutex poisoning"
affects: [ci, testing, config]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Figment env provider filtering for serde(skip) fields"
    - "ClaudeCommand struct in test setup"

key-files:
  created: []
  modified:
    - "src/config.rs"
    - "src/planning/command.rs"

key-decisions:
  - "Filter claude_cmd from Figment Env provider to prevent deserialization conflict"
  - "Use ClaudeCommand directly in tests instead of deprecated claude_path"

patterns-established:
  - "Env provider filtering pattern: Env::prefixed(\"RSLPH_\").lowercase(true).filter(|key| key != \"claude_cmd\")"

# Metrics
duration: 2m 4s
completed: 2026-01-30
---

# Quick Task 006: Test Suite Fixes Summary

**Fixed 5 failing tests by filtering claude_cmd from Figment Env provider and updating deprecated test patterns**

## Performance

- **Duration:** 2m 4s
- **Started:** 2026-01-30T21:56:26Z
- **Completed:** 2026-01-30T21:58:30Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments
- All 294 unit tests passing with 0 failures
- Fixed Figment env var deserialization conflict with serde(skip) fields
- Updated tests to use modern ClaudeCommand struct instead of deprecated claude_path
- Eliminated mutex poisoning cascade from config test failures

## Task Commits

Each task was committed atomically:

1. **Task 1: Filter claude_cmd from Figment Env provider** - `27aa068` (fix)
2. **Task 2: Fix test_run_plan_command_nonexistent_command to use claude_cmd** - `02b6063` (fix)
3. **Task 3: Verify all tests pass** - ✓ (verification only, no commit)

## Files Created/Modified
- `src/config.rs` - Added .filter(|key| key != "claude_cmd") to Env providers
- `src/planning/command.rs` - Updated test to use ClaudeCommand directly

## Decisions Made

**Filter claude_cmd from Figment Env provider**
- The `claude_cmd` field is marked `#[serde(skip)]` because we handle RSLPH_CLAUDE_CMD manually after extraction (parsing it into command + base_args)
- When Figment's Env provider sees RSLPH_CLAUDE_CMD, it tries to deserialize it into the skipped `claude_cmd` field, causing extraction failure
- Solution: Filter out "claude_cmd" key from Env provider before deserialization
- Applied to both `load()` and `load_with_overrides()` methods

**Use ClaudeCommand directly in tests**
- Tests should use the modern `claude_cmd` field directly instead of relying on `claude_path` conversion
- Provides more explicit control over test scenarios
- Prevents reliance on deprecated field behavior

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

**Root cause 1: Figment Env provider conflict**
- Figment was attempting to deserialize RSLPH_CLAUDE_CMD into the `claude_cmd` field
- The field is marked `#[serde(skip)]` because manual parsing is required
- This caused 4 config tests to fail, with mutex poisoning cascading to other tests
- Fixed by filtering the key before deserialization

**Root cause 2: Deprecated test pattern**
- Test `test_run_plan_command_nonexistent_command` was setting `claude_path` field
- The code actually uses `config.claude_cmd.command` to spawn subprocess
- Setting `claude_path` via Default doesn't propagate to `claude_cmd` without going through the full `Config::load` machinery
- Fixed by setting `claude_cmd` directly using ClaudeCommand struct

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All tests passing, ready for continuous integration
- CI pipeline can now run `cargo test` with confidence
- Test isolation is working correctly (no mutex poisoning)
- RSLPH_CLAUDE_CMD env var integration is fully tested and working

---
*Phase: quick-006*
*Completed: 2026-01-30*
