---
phase: 04-core-build-loop
plan: 04
subsystem: subprocess
tags: [path-resolution, which, error-handling, diagnostics]

# Dependency graph
requires:
  - phase: 04-core-build-loop/03
    provides: Build loop with dry-run and once modes
provides:
  - resolve_command_path function using `which` fallback
  - Enhanced spawn error with PATH diagnostic
affects: [05-vcs-integration]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "which-based command resolution for cross-env compatibility"
    - "PATH diagnostic in error messages for troubleshooting"

key-files:
  created: []
  modified:
    - src/config.rs
    - src/build/iteration.rs

key-decisions:
  - "WHICH-FALLBACK: Use `which` to resolve relative command names to absolute paths at config load time"

patterns-established:
  - "Command path resolution: Always resolve relative paths via which before subprocess spawn"
  - "Diagnostic errors: Include environment context (PATH) in spawn failure messages"

# Metrics
duration: 2m 15s
completed: 2026-01-18
---

# Phase 04 Plan 04: Gap Closure - Claude Path Resolution Summary

**Resolve relative claude command to absolute path using `which`, with enhanced error diagnostics showing PATH context**

## Performance

- **Duration:** 2 min 15 sec
- **Started:** 2026-01-18T05:00:00Z
- **Completed:** 2026-01-18T05:02:15Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Config loading now resolves "claude" to absolute path via `which` command
- Spawn failures produce actionable error messages with PATH context
- All existing tests continue to pass (93 tests)
- UAT gap for "Failed to spawn claude: No such file or directory" closed

## Task Commits

Each task was committed atomically:

1. **Task 1: Add resolve_command_path function and apply to claude_path** - `b77a256` (feat)
2. **Task 2: Enhance spawn error message with PATH diagnostic** - `b4f169c` (feat)

## Files Created/Modified
- `src/config.rs` - Added resolve_command_path() function, applied to claude_path in Config::load() and Config::load_with_overrides(), added 3 tests
- `src/build/iteration.rs` - Enhanced spawn error message to include command path, PATH environment, and actionable guidance

## Decisions Made
- **WHICH-FALLBACK**: Use `which` command to resolve relative command names to absolute paths. This handles the case where claude is in shell PATH but subprocess environments may differ from interactive shells.

## Deviations from Plan
None - plan executed exactly as written.

## Issues Encountered
None - implementation matched plan specification.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 4 gap closure complete
- All UAT tests should now pass (build spawns claude successfully)
- Ready for Phase 5: VCS Integration

---
*Phase: 04-core-build-loop*
*Completed: 2026-01-18*
