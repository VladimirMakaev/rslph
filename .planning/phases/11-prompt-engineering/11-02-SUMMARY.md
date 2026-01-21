---
phase: 11-prompt-engineering
plan: 02
subsystem: prompts
tags: [prompt-engineering, gsd, xml-structure, deviation-handling]

dependency_graph:
  requires:
    - 11-01 (basic mode prompts)
  provides:
    - GSD planning prompt with XML task structure
    - GSD build prompt with deviation handling
  affects:
    - 11-03 (GSD-TDD mode will extend these patterns)

tech_stack:
  added: []
  patterns:
    - XML task structure (<task>, <name>, <action>, <verify>, <done>)
    - Must-haves verification (truths, artifacts, key links)
    - Deviation handling rules (4-rule system)
    - Substantive completion summaries

key_files:
  created:
    - prompts/gsd/PROMPT_plan.md
    - prompts/gsd/PROMPT_build.md
  modified: []

decisions:
  - id: gsd-xml-structure
    choice: "Use <task>, <name>, <action>, <verify>, <done> tags"
    rationale: "Clear structure for autonomous agent parsing"
  - id: deviation-rules
    choice: "4-rule system: bug, missing-critical, blocking, architectural"
    rationale: "Matches GSD pattern, provides clear priority order"
  - id: must-haves-format
    choice: "Truths, Artifacts, Key Links sections with checkboxes"
    rationale: "Goal-backward verification enables autonomous completeness check"

metrics:
  duration: 3m 10s
  completed: 2026-01-21
---

# Phase 11 Plan 02: GSD Mode Prompts Summary

GSD planning and build prompts with XML task structure, deviation handling, and must-haves verification for goal-backward success criteria.

## What Was Built

### 1. GSD Planning Prompt (`prompts/gsd/PROMPT_plan.md`)

Created planning prompt (375 lines) with GSD-adapted patterns:

**XML Task Structure:**
```xml
<task>
  <name>Task name</name>
  <action>Implementation instructions</action>
  <verify>Verification command</verify>
  <done>Completion criteria</done>
</task>
```

**Must-Haves Section:**
- Truths: Observable behaviors that must be true
- Artifacts: Files that must exist with real implementation
- Key Links: Critical connections between components

**Progress File Format:**
- YAML frontmatter for state tracking (phase, status, iterations, tokens)
- Current Position section
- Accumulated Learnings for iteration memory
- Decisions Made table

**Testing Philosophy:**
- Interleaved testing (not batched at end)
- Test type heuristics based on project stack
- Verification levels (Exists, Substantive, Wired, Functional)

### 2. GSD Build Prompt (`prompts/gsd/PROMPT_build.md`)

Created build prompt (370 lines) implementing PROMPT-01, PROMPT-02, and PROMPT-05:

**Deviation Handling (PROMPT-01):**
- Rule 1: Auto-fix bugs in code just written
- Rule 2: Add critical functionality (error handling, validation, security)
- Rule 3: Fix blocking issues (missing deps, config errors)
- Rule 4: Ask about architecture (stop and document for major changes)

**Substantive Completion Summary (PROMPT-02):**
```markdown
### Task: [Task name]

**What Changed:**
- Created/modified: [files with description]
- Key implementation: [approach summary]

**Verification:**
- Ran: [command]
- Result: [pass/fail with details]

**Deviations Applied:**
- [Rule N - Type] [description]

**Next:** [What next task needs to know]
```

**Must-Haves Verification (PROMPT-05):**
- Check all truths before RALPH_DONE
- Verify all artifacts exist with real implementation
- Confirm all key links are wired and functional

## Key Patterns Established

| Pattern | Plan Prompt | Build Prompt |
|---------|-------------|--------------|
| XML task structure | Defines format | Parses and marks complete |
| Must-haves | Creates from goal-backward | Verifies before RALPH_DONE |
| Progress file | Defines structure | Updates each iteration |
| Testing interleaved | Plans with tests | Executes tests inline |
| Verification levels | Documents requirements | Applies before marking |

## Commits

| Hash | Description |
|------|-------------|
| 9b20514 | feat(11-02): create GSD planning prompt with XML task structure |
| 6a2c6eb | feat(11-02): create GSD build prompt with deviation handling |

## Deviations from Plan

None - plan executed exactly as written.

## Requirements Coverage

| Requirement | Status |
|-------------|--------|
| PROMPT-01 (Deviation handling) | Covered - 4-rule system in build prompt |
| PROMPT-02 (Completion summaries) | Covered - substantive format defined |
| PROMPT-05 (GSD patterns) | Covered - must-haves in both prompts |

## Next Phase Readiness

Ready for 11-03 (GSD-TDD mode):
- GSD base prompts established as foundation
- TDD-specific additions can extend these patterns:
  - TDD State section in progress file
  - RED-GREEN-REFACTOR cycle instructions
  - TDD escape hatch after 3 failed attempts
