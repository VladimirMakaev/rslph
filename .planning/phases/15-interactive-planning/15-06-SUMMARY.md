---
phase: 15-interactive-planning
plan: 06
subsystem: prompts
tags: [planning, prompts, AskUserQuestion, adaptive-mode]

# Dependency graph
requires:
  - phase: 15-interactive-planning (plans 01-04)
    provides: Session resume and AskUserQuestion tool detection
provides:
  - Modified planning prompts that allow questions in adaptive mode
  - Consistent guideline across gsd, gsd_tdd, basic, and root prompts
affects: [planning workflow, adaptive mode, user interaction]

# Tech tracking
tech-stack:
  added: []
  patterns: [conditional-behavior-by-mode, adaptive-vs-standard-mode]

key-files:
  created: []
  modified:
    - prompts/gsd/PROMPT_plan.md
    - prompts/gsd_tdd/PROMPT_plan.md
    - prompts/basic/PROMPT_plan.md
    - prompts/PROMPT_plan.md

key-decisions:
  - "Conditional question behavior based on --adaptive flag"
  - "Questions limited to 2-5 max to prevent over-questioning"
  - "Standard mode documents assumptions in Analysis section"

patterns-established:
  - "Mode-conditional behavior: standard mode has one behavior, adaptive mode has another"
  - "AskUserQuestion tool for tech choices, scope decisions, and deployment context"

# Metrics
duration: 3min
completed: 2026-02-01
---

# Phase 15 Plan 06: Allow Clarifying Questions in Adaptive Mode Summary

**Modified all 4 planning prompts to conditionally allow AskUserQuestion tool usage in adaptive mode while preserving default no-questions behavior**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-01T00:00:00Z
- **Completed:** 2026-02-01T00:03:00Z
- **Tasks:** 2/2
- **Files modified:** 4

## Accomplishments
- Replaced "Do NOT ask clarifying questions" with conditional guideline in all prompts
- Standard mode still makes assumptions and documents them in Analysis
- Adaptive mode can now use AskUserQuestion tool for critical questions
- Questions limited to 2-5 max with clear guidance on appropriate question types

## Task Commits

Each task was committed atomically:

1. **Task 1: Modify GSD planning prompt to allow questions** - `fb5b317` (feat)
2. **Task 2: Modify GSD-TDD, Basic, and root planning prompts** - `9d00063` (feat)

## Files Created/Modified
- `prompts/gsd/PROMPT_plan.md` - Updated guideline 7 with conditional question behavior
- `prompts/gsd_tdd/PROMPT_plan.md` - Updated guideline 7 with conditional question behavior
- `prompts/basic/PROMPT_plan.md` - Updated guideline 7 with conditional question behavior
- `prompts/PROMPT_plan.md` - Updated guideline 7 with conditional question behavior

## Decisions Made
- Conditional behavior based on `--adaptive` flag (preserves backward compatibility)
- Questions limited to 2-5 to prevent over-questioning
- Three categories of appropriate questions: tech choices, scope decisions, deployment context
- Standard mode documents assumptions in Analysis section (maintains traceability)

## Deviations from Plan
None - plan executed exactly as written.

## Issues Encountered
None - straightforward text replacement in all 4 files.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All planning prompts now support AskUserQuestion in adaptive mode
- Root cause of UAT failures (prompts forbidding questions) is now fixed
- Ready for UAT re-run to verify all 6 issues are resolved

---
*Phase: 15-interactive-planning*
*Completed: 2026-02-01*
