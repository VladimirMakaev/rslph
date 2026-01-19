# Roadmap: rslph

## Milestones

- ✅ **v1.0 MVP** — Phases 1-6 (shipped 2026-01-19)
- 🚧 **v1.1 Prompt Engineering** — Phases 7-9 (in progress)

## Overview

v1.1 "Prompt Engineering" adds comprehensive E2E testing with fake Claude simulation, verification agent, notifications, and prompt customization.

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1-6) — SHIPPED 2026-01-19</summary>

See `.planning/milestones/v1.0-ROADMAP.md` for full details.

- [x] Phase 1: Foundation (3/3 plans)
- [x] Phase 2: Subprocess Management (2/2 plans)
- [x] Phase 3: Planning Command (2/2 plans)
- [x] Phase 4: Core Build Loop (4/4 plans)
- [x] Phase 5: VCS Integration (2/2 plans)
- [x] Phase 6: TUI Interface (4/4 plans)

</details>

### 🚧 v1.1 Prompt Engineering (In Progress)

- [ ] **Phase 7: E2E Testing Framework** - Fake Claude, scenario API, workspace fixtures
- [ ] **Phase 8: Verification** - Test agent and independent verification
- [ ] **Phase 9: Notifications and Polish** - Notify scripts, prompt overrides, error refinement

## Phase Details

### Phase 7: E2E Testing Framework
**Goal**: Comprehensive testing infrastructure with fake Claude simulation for deterministic, reproducible testing of the Ralph Wiggum Loop
**Depends on**: Phase 6 (TUI complete)
**Requirements**: TEST-01 through TEST-12

**Success Criteria** (what must be TRUE):
  1. Python fake-claude package exists with fluent scenario API
  2. Fake Claude produces stream-json format matching real Claude CLI
  3. Tool calls (Read, Write, Edit, Bash) can be simulated with configurable results
  4. Progress file manipulation (mark complete, RALPH_DONE) is testable
  5. Edge cases are covered (timeout, crash, malformed output, fast output)
  6. Workspace fixture manages temp directories with config and source files
  7. Verifier helpers assert on task completion, file content, git commits
  8. UAT demo scenarios exist for visual testing
  9. Pytest integration with fixtures works
  10. Multi-invocation support for testing retry/failure memory

**Plans**: 4 plans in 3 waves

Plans:
- [ ] 07-01-PLAN.md — Core fake-claude package with stream-json generators
- [ ] 07-02-PLAN.md — Workspace fixtures and pytest infrastructure
- [ ] 07-03-PLAN.md — Edge case scenarios and multi-invocation support
- [ ] 07-04-PLAN.md — Verifier helpers and UAT demo scenarios

### Phase 8: Verification
**Goal**: Test agent runs independently to verify build results
**Depends on**: Phase 7 (testing framework for verification testing)
**Requirements**: VERIF-01, VERIF-02, VERIF-03

**Success Criteria** (what must be TRUE):
  1. Testing strategy from planning phase is captured and available
  2. Separate test agent persona (not embedded in build loop) can execute
  3. Verification runs independently and reports pass/fail

**Plans**: TBD (1-2 plans estimated)

Plans:
- [ ] 08-01: Test agent implementation and verification runner

### Phase 9: Notifications and Polish
**Goal**: User receives notifications at key points, prompts are fully customizable
**Depends on**: Phase 6
**Requirements**: NOTIF-01, NOTIF-02, NOTIF-03, NOTIF-04, NOTIF-05, PROMPT-04

**Success Criteria** (what must be TRUE):
  1. User-provided notify script runs after 1st iteration
  2. Notify script runs every X iterations (configurable)
  3. Notify script runs on completion and on failure
  4. Script executes via configurable shell with context (status, counts)
  5. User can override PROMPT_plan and PROMPT_build via config file paths

**Plans**: TBD (2 plans estimated)

Plans:
- [ ] 09-01: Notification system
- [ ] 09-02: Prompt override system and final polish

## Progress

**Execution Order:**
Phases execute in numeric order: 7 -> 8 -> 9

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Foundation | v1.0 | 3/3 | Complete | 2026-01-17 |
| 2. Subprocess Management | v1.0 | 2/2 | Complete | 2026-01-17 |
| 3. Planning Command | v1.0 | 2/2 | Complete | 2026-01-18 |
| 4. Core Build Loop | v1.0 | 4/4 | Complete | 2026-01-18 |
| 5. VCS Integration | v1.0 | 2/2 | Complete | 2026-01-18 |
| 6. TUI Interface | v1.0 | 4/4 | Complete | 2026-01-19 |
| 7. E2E Testing Framework | v1.1 | 0/4 | Planned | - |
| 8. Verification | v1.1 | 0/1 | Not started | - |
| 9. Notifications and Polish | v1.1 | 0/2 | Not started | - |
