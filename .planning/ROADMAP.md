# Roadmap: rslph

## Milestones

- ✅ **v1.0 MVP** — Phases 1-6 (shipped 2026-01-19)
- 🚧 **v1.1 Testing Enhancement** — Phases 7-8 (in progress)

## Overview

v1.1 "Testing Enhancement" adds comprehensive E2E testing with fake Claude simulation, verification agent, notifications, and prompt customization.

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

### 🚧 v1.1 Testing Enhancement (In Progress)

- [x] **Phase 7: E2E Testing Framework** - Fake Claude (Rust), scenario API, workspace fixtures ✓
- [x] **Phase 7.1: TUI Testing** - TestBackend + insta snapshot tests for TUI ✓
- [ ] **Phase 8: Verification** - Test agent and independent verification

## Phase Details

### Phase 7: E2E Testing Framework
**Goal**: Comprehensive all-Rust testing infrastructure with fake Claude simulation for deterministic, reproducible testing of the Ralph Wiggum Loop
**Depends on**: Phase 6 (TUI complete)
**Requirements**: TEST-01 through TEST-12

**Success Criteria** (what must be TRUE):
  1. Rust fake-claude binary exists with fluent scenario API
  2. Fake Claude produces stream-json format matching real Claude CLI (reuses existing types)
  3. Tool calls (Read, Write, Edit, Bash) can be simulated with configurable results
  4. Progress file manipulation (mark complete, RALPH_DONE) is testable
  5. Edge cases are covered (timeout, crash, malformed output, fast output)
  6. Workspace fixture manages temp directories with config and source files (Rust)
  7. Verifier helpers assert on task completion, file content, git commits (Rust)
  8. E2E tests verify complete integration
  9. Multi-invocation support for testing retry/failure memory
  10. True E2E integration tests run rslph with fake Claude

**Plans**: 5 plans in 4 waves

Plans:
- [x] 07-01-PLAN.md — Core fake-claude infrastructure with stream-json output
- [x] 07-02-PLAN.md — Workspace fixtures and verifier helpers
- [x] 07-03-PLAN.md — Tool calls, edge cases, multi-invocation support
- [x] 07-04-PLAN.md — Infrastructure verification tests
- [x] 07-05-PLAN.md — True E2E integration tests (rslph with fake Claude)

### Phase 7.1: TUI Testing with TestBackend + insta (INSERTED)
**Goal**: Implement TUI-specific snapshot tests using ratatui TestBackend and insta
**Depends on**: Phase 7 (E2E testing infrastructure)
**Plans**: 1 plan

**Context**: Phase 7 deferred TUI testing. Research determined TestBackend + insta is the right approach (ratatui-testlib is PTY-based overkill). This phase implements TUI rendering and key handling tests.

**Success Criteria** (what must be TRUE):
  1. ratatui-testlib API verified and documented (DONE in RESEARCH.md)
  2. TUI rendering tests exist (widgets, layout, visual output)
  3. Key handling tests exist (navigation, input)
  4. TUI tests integrate with existing E2E infrastructure

Plans:
- [x] 07.1-01-PLAN.md — TUI snapshot tests (rendering + key handling) ✓

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

## Progress

**Execution Order:**
Phases execute in numeric order: 7 -> 7.1 -> 8

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Foundation | v1.0 | 3/3 | Complete | 2026-01-17 |
| 2. Subprocess Management | v1.0 | 2/2 | Complete | 2026-01-17 |
| 3. Planning Command | v1.0 | 2/2 | Complete | 2026-01-18 |
| 4. Core Build Loop | v1.0 | 4/4 | Complete | 2026-01-18 |
| 5. VCS Integration | v1.0 | 2/2 | Complete | 2026-01-18 |
| 6. TUI Interface | v1.0 | 4/4 | Complete | 2026-01-19 |
| 7. E2E Testing Framework | v1.1 | 5/5 | Complete | 2026-01-19 |
| 7.1 TUI Testing | v1.1 | 1/1 | Complete | 2026-01-19 |
| 8. Verification | v1.1 | 0/1 | Not started | - |
