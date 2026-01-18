---
phase: 05-vcs-integration
plan: 01
subsystem: vcs
tags: [git, sapling, auto-commit, rollback]

# Dependency graph
requires:
  - phase: 04-core-build-loop
    provides: BuildContext, iteration execution, progress file management
provides:
  - VCS trait abstraction for Git and Sapling
  - Auto-detection of VCS type (Sapling-first, Git-fallback)
  - Auto-commit after iterations with completed tasks
  - Rollback safety via atomic commits per iteration
affects: [06-test-infrastructure, 07-polish, 08-release]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Shell out to VCS CLI commands (no git2 crate)
    - Vcs trait abstraction for multi-VCS support
    - Option<Box<dyn Vcs>> in BuildContext for optional VCS

key-files:
  created:
    - src/vcs/mod.rs
    - src/vcs/git.rs
    - src/vcs/sapling.rs
  modified:
    - src/error.rs
    - src/lib.rs
    - src/build/state.rs
    - src/build/iteration.rs

key-decisions:
  - "VCS-SHELL-OUT: Shell out to git/sl CLI rather than using git2 crate (simpler, no C dependency)"
  - "VCS-SAPLING-FIRST: Detect Sapling via sl root before Git via .git directory"
  - "VCS-WARN-NOT-FAIL: VCS errors are logged as warnings, do not fail the build"
  - "VCS-ITER-COMMIT: Commit after iteration completion, not per-task"

patterns-established:
  - "Vcs trait pattern: has_changes -> stage_all -> commit, with commit_all default"
  - "VCS detection pattern: try command first (sl root), fall back to directory check (.git)"
  - "Optional integration: Option<Box<dyn Trait>> for features that may not be available"

# Metrics
duration: 4min
completed: 2026-01-18
---

# Phase 5 Plan 1: VCS Integration Summary

**VCS auto-commit via Vcs trait abstraction with Git/Sapling implementations, Sapling-first detection, and build loop integration**

## Performance

- **Duration:** 4 min
- **Started:** 2026-01-18T22:09:27Z
- **Completed:** 2026-01-18T22:13:30Z
- **Tasks:** 3
- **Files modified:** 7

## Accomplishments

- Created VCS module with Vcs trait, VcsType enum, and VcsError type
- Implemented GitVcs and SaplingVcs with shell-out pattern to CLI commands
- Added detect_vcs function that prefers Sapling (sl root) over Git (.git directory)
- Integrated VCS auto-commit into BuildContext and iteration loop
- Added 8 unit tests covering detection, change tracking, and commit operations

## Task Commits

Each task was committed atomically:

1. **Task 1: Create VCS module** - `21c8e06` (feat)
2. **Task 2: Integrate VCS into build loop** - `dd866ec` (feat)
3. **Task 3: Add unit tests** - (included in Task 1 commit)

## Files Created/Modified

- `src/vcs/mod.rs` - VCS trait, VcsType enum, detect_vcs(), create_vcs()
- `src/vcs/git.rs` - GitVcs implementation with git CLI shell-out
- `src/vcs/sapling.rs` - SaplingVcs implementation with sl CLI shell-out
- `src/error.rs` - VcsError enum with CommandFailed, NothingToCommit, CommitFailed, Detection
- `src/lib.rs` - Export vcs module
- `src/build/state.rs` - Add vcs field to BuildContext, detect VCS on construction
- `src/build/iteration.rs` - Add VCS commit after iterations with completed tasks

## Decisions Made

1. **Shell out to CLI (VCS-SHELL-OUT):** Use git/sl CLI commands rather than git2 crate. This avoids C library dependencies, uses user's configured git with all their settings, and is simpler for basic operations. The jj-vcs project deprecated git2 for similar reasons.

2. **Sapling-first detection (VCS-SAPLING-FIRST):** Check `sl root` before looking for `.git` directory. Sapling works as a smart client with Git repos, so users who have sl installed likely prefer it.

3. **Warn not fail (VCS-WARN-NOT-FAIL):** VCS errors are logged as warnings and do not fail the build. VCS is a convenience feature for rollback, not critical path.

4. **Iteration commit (VCS-ITER-COMMIT):** Commit once after each iteration that completes tasks, not per-task. This matches the "fresh context per iteration" architecture.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required. VCS detection is automatic based on repository type.

## Next Phase Readiness

- VCS integration complete with Git and Sapling support
- Ready for Phase 6 (Test Infrastructure) - existing tests pass (101 tests)
- VCS commits will now appear in history during rslph build runs
- Users can rollback iterations with standard git/sl commands

---
*Phase: 05-vcs-integration*
*Completed: 2026-01-18*
