---
phase: 10-eval-projects-and-testing
plan: 01
subsystem: eval
tags: [include_dir, embedding, eval, projects, registry]

# Dependency graph
requires:
  - phase: 09-eval-command-foundation
    provides: eval command infrastructure for running benchmarks
provides:
  - Built-in project registry with include_dir embedding
  - Calculator eval project with prompt and hidden tests
  - Functions: get_project, list_projects, is_builtin, extract_project_files, get_test_data
affects: [10-02, 10-03, 10-04, prompt-engineering, multi-trial]

# Tech tracking
tech-stack:
  added: [include_dir 0.7]
  patterns: [compile-time file embedding, hidden test data separation]

key-files:
  created: [src/eval/projects.rs, evals/calculator/prompt.txt, evals/calculator/tests.jsonl]
  modified: [Cargo.toml, src/eval/mod.rs]

key-decisions:
  - "File paths in include_dir include project directory prefix"
  - "extract_project_files excludes tests.jsonl to hide test data from agents"

patterns-established:
  - "Project registry: use include_dir! macro to embed evals/ at compile time"
  - "Test data separation: prompt.txt visible, tests.jsonl hidden via get_test_data"

# Metrics
duration: 3min 15s
completed: 2026-01-20
---

# Phase 10 Plan 01: Built-in Eval Projects Summary

**Calculator eval project embedded with include_dir registry providing prompt visibility and hidden test data separation**

## Performance

- **Duration:** 3 min 15 sec
- **Started:** 2026-01-20T15:18:05Z
- **Completed:** 2026-01-20T15:21:20Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Added include_dir crate for compile-time file embedding
- Created project registry with 5 functions: get_project, list_projects, is_builtin, extract_project_files, get_test_data
- Created calculator eval project with 21-line prompt and 10 test cases
- Verified embedding works via 5 unit tests

## Task Commits

Each task was committed atomically:

1. **Task 1: Add include_dir dependency and create project registry** - `924640a` (feat)
2. **Task 2: Fix file path handling in get_test_data** - `00c6c6f` (fix)

## Files Created/Modified
- `Cargo.toml` - Added include_dir = "0.7" dependency
- `src/eval/mod.rs` - Export projects module functions
- `src/eval/projects.rs` - Built-in project registry with embedding
- `evals/calculator/prompt.txt` - Calculator prompt for agents (visible)
- `evals/calculator/tests.jsonl` - Hidden test cases (10 tests)

## Decisions Made
- File paths in include_dir include the project directory prefix (e.g., "calculator/tests.jsonl" not "tests.jsonl")
- Used project.path().join("tests.jsonl") to construct correct paths

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed include_dir file path handling**
- **Found during:** Task 2 (Verification tests)
- **Issue:** get_test_data returned None because include_dir stores files with full path from embedded root
- **Fix:** Use project.path().join("tests.jsonl") to construct correct path
- **Files modified:** src/eval/projects.rs
- **Verification:** All 5 unit tests pass
- **Committed in:** 00c6c6f

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Bug fix necessary for correct operation. No scope creep.

## Issues Encountered
- include_dir macro requires directory to exist at compile time - created evals/calculator/ structure before cargo check could pass

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Built-in project registry complete and tested
- Ready for 10-02: eval command integration with built-in projects
- Calculator project can be used for end-to-end eval testing

---
*Phase: 10-eval-projects-and-testing*
*Completed: 2026-01-20*
