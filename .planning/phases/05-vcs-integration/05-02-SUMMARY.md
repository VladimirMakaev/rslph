---
phase: 05-vcs-integration
plan: 02
subsystem: vcs
tags: [bugfix, gap-closure, commit-message, sapling]

requires:
  - phase: 05-vcs-integration
    provides: VCS trait, GitVcs, SaplingVcs, build loop integration

provides:
  - Correct project name in VCS commit messages
  - Sapling commit hash retrieval via sl log

affects: []

tech-stack:
  added: []
  patterns:
    - Store invariant data in context at construction

key-files:
  modified:
    - src/build/state.rs
    - src/build/iteration.rs
    - src/vcs/sapling.rs

key-decisions:
  - "CONTEXT-CAPTURED-NAME: Store project_name in BuildContext at construction, not rely on Claude response"
  - "SL-LOG-HASH: Use sl log -l 1 --template '{node|short}' to get commit hash after sl commit"

patterns-established:
  - "Context capture: Store invariant data at context construction for reliable access"

duration: 5min
completed: 2026-01-18
---

# Phase 5 Plan 2: VCS Bug Fixes Summary

**Gap closure: Fix empty project name and unknown Sapling hash**

## Performance

- **Duration:** 5 min
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments

- Added `project_name: String` field to BuildContext, captured at construction
- Changed commit message to use `ctx.project_name` instead of `updated_progress.name`
- Fixed Sapling commit hash retrieval: run `sl log` after commit instead of parsing empty stdout

## Task Commits

1. **All tasks** - `6ffc39b` (fix)

## Files Modified

- `src/build/state.rs` - Add project_name field to BuildContext
- `src/build/iteration.rs` - Use ctx.project_name for commit message
- `src/vcs/sapling.rs` - Get hash via sl log after commit

## Decisions Made

1. **Store project name in context (CONTEXT-CAPTURED-NAME):** The project name should be captured at BuildContext construction from the original progress file. This is more reliable than depending on Claude to echo the name back correctly in its response.

2. **Sapling hash via sl log (SL-LOG-HASH):** Sapling's `sl commit` command produces no stdout on success (unlike Git). After a successful commit, run `sl log -l 1 --template '{node|short}'` to retrieve the commit hash.

## Deviations from Original Plan

The original diagnosis was incorrect - it suggested using `ctx.progress.name` which is re-read each iteration. The correct fix is storing the name at construction in a dedicated field.

## Issues Resolved

- UAT Test 2: Empty project name in commit message → Fixed
- UAT Test 3: Sapling hash shows "unknown" → Fixed

---
*Phase: 05-vcs-integration*
*Gap closure plan completed: 2026-01-18*
