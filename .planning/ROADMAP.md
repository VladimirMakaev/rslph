# Roadmap: rslph

## Milestones

- v1.0 MVP -- Phases 1-6 (shipped 2026-01-19)
- v1.1 Testing Enhancement -- Phases 7-7.1 (shipped 2026-01-19)
- v1.2 Context Engineering -- Phases 8-15 (shipped 2026-02-01)
- **v1.3 Hardening** -- Phases 16-20 (active)

## Overview

Current milestone: v1.3 "Hardening" — Consolidate and harden existing features, remove dead code paths, and establish GSD-style multi-persona workflow while ensuring robust TUI input handling.

---

## v1.3 Hardening

### Phase 16: Cleanup

**Goal:** Remove deprecated code paths leaving only TUI-based execution and supported prompt modes

**Dependencies:** None (foundational for v1.3)

**Requirements:**
| ID | Requirement |
|----|-------------|
| MODE-01 | Remove gsd_tdd mode — Delete gsd_tdd mode entirely from codebase (prompts, CLI args, tests) |
| TUI-01 | TUI-only mode for all commands — Remove all non-TUI code paths from plan, build, and eval commands |

**Success Criteria:**
1. User runs `rslph plan` and it always launches TUI mode (no `--no-tui` flag exists)
2. User runs `rslph build` and it always launches TUI mode
3. User runs `rslph --mode gsd_tdd` and receives an error (mode does not exist)
4. Codebase search for "gsd_tdd" returns zero matches
5. All E2E tests pass without non-TUI code paths

**Plans:** TBD

---

### Phase 17: Basic Mode Alignment

**Goal:** Basic mode behavior matches portableralph reference implementation exactly

**Dependencies:** Phase 16 (Cleanup)

**Requirements:**
| ID | Requirement |
|----|-------------|
| MODE-02 | Basic mode prompts match portableralph — Use exact prompt structure from snarktank/ralph reference implementation |
| MODE-03 | Basic mode loop behavior identical — Fresh context per iteration, one task per iteration, RALPH_DONE termination |
| MODE-04 | Basic mode progress file format — Match portableralph progress.txt structure for task tracking and learnings |
| MODE-05 | Basic mode commit behavior — Commit after each completed task (matching ralph.sh) |

**Success Criteria:**
1. User runs `rslph plan --mode basic "goal"` and gets progress file matching portableralph structure
2. Build loop executes exactly one task per iteration with fresh context
3. RALPH_DONE marker in status section terminates build loop
4. Each completed task triggers a VCS commit before next iteration
5. Progress file structure matches portableralph: status, tasks, learnings sections

**Plans:** TBD

---

### Phase 18: TUI Enhancement

**Goal:** Users can interact with Claude via multiline text input with proper cursor navigation and streaming display

**Dependencies:** Phase 17 (Basic Mode Alignment)

**Requirements:**
| ID | Requirement |
|----|-------------|
| TUI-02 | Multiline text input with cursor — Use tui-textarea for proper cursor navigation, word movement, and undo/redo |
| TUI-03 | Full dialog display — Show complete Claude conversation in TUI including streaming output, questions, and user responses |
| TUI-04 | Input widget at bottom — Following claude_dialog.py pattern, input docked at bottom with conversation history above |
| TUI-05 | Streaming text handling — Buffer text until newlines, display complete lines as they arrive (port claude_dialog.py pattern) |

**Success Criteria:**
1. User can navigate multiline input with arrow keys, Ctrl+A/E (line start/end), Alt+F/B (word movement)
2. User can undo/redo text input with Ctrl+U/R
3. Conversation history scrolls above fixed input widget at bottom of screen
4. Streaming text from Claude displays line-by-line as complete lines arrive
5. User responses appear in conversation history after submission

**Plans:** TBD

---

### Phase 19: GSD Personas

**Goal:** Ralph Loop supports persona-driven execution with executor, verifier, and checkpoint capabilities

**Dependencies:** Phase 18 (TUI Enhancement)

**Requirements:**
| ID | Requirement |
|----|-------------|
| GSD-01 | Progress file persona field — Add `## Next Persona` section (executor/verifier/researcher/planner) that determines system prompt for next iteration |
| GSD-02 | Persona prompt library — Define system prompts for each persona with distinct behaviors and constraints |
| GSD-03 | Executor persona — Standard build execution with deviation rules (auto-fix bugs, ask about architecture changes) |
| GSD-04 | Verifier persona — Goal-backward verification (check truths, artifacts, wiring) after all tasks complete |
| GSD-05 | Checkpoint section in progress — Add `## Checkpoint` section with type (human-verify/decision/human-action), awaiting message, resume task |
| GSD-06 | Checkpoint iteration result — Add Checkpoint variant to IterationResult enum that pauses loop for user input |
| GSD-07 | TUI checkpoint display — Show checkpoint information and await user response via input widget |
| GSD-08 | Compact decisions in progress — Add decisions to Recent Attempts with format: "Decision: [what] — [why]" |
| GSD-09 | Persona auto-transition — After all tasks complete, set next_persona to "verifier" for goal-backward check |

**Success Criteria:**
1. Progress file contains `## Next Persona` section with valid persona (executor, verifier, researcher, planner)
2. Build loop selects different system prompt based on next_persona value
3. Executor persona follows deviation rules: auto-fixes bugs, asks about architecture changes
4. After all tasks complete, next_persona automatically changes to "verifier"
5. Verifier persona checks goal achievement via truths/artifacts/wiring analysis
6. Checkpoint in progress file pauses build loop and displays awaiting message in TUI
7. User can respond to checkpoint via TUI input, resuming execution at specified task
8. Decisions appear in Recent Attempts section with "Decision: [what] - [why]" format

**Plans:** TBD

---

### Phase 20: E2E Tests

**Goal:** Comprehensive E2E test coverage for planning scenarios and TUI input behavior

**Dependencies:** Phase 19 (GSD Personas)

**Requirements:**
| ID | Requirement |
|----|-------------|
| TEST-01 | Planning with 0 question rounds — E2E test where Claude produces plan without asking questions |
| TEST-02 | Planning with 1 question round — E2E test where Claude asks 1 round of clarifying questions |
| TEST-03 | Planning with 2 question rounds — E2E test where Claude asks 2 rounds of questions |
| TEST-04 | Planning with 3 question rounds — E2E test where Claude asks 3 rounds of questions |
| TEST-05 | TUI input prompt with cursor — Snapshot test showing input widget with visible cursor |
| TEST-06 | Multi-iteration build with tokens — E2E test verifying token accumulation across iterations |
| TEST-07 | Tests use ratatui-testlib and fake claude — All TUI tests use established TestBackend + insta pattern |

**Success Criteria:**
1. E2E test for 0-question planning produces valid progress.md without interaction
2. E2E test for 1-question planning collects answer and resumes to produce progress.md
3. E2E test for 2-question planning handles two Q&A rounds correctly
4. E2E test for 3-question planning handles three Q&A rounds correctly
5. TUI snapshot captures input widget with visible cursor position
6. Multi-iteration build test verifies token totals accumulate correctly across iterations
7. All TUI tests use TestBackend + insta pattern for deterministic snapshots

**Plans:** TBD

---

## Progress

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
| 8. Token Tracking | v1.2 | 4/4 | Complete | 2026-01-20 |
| 9. Eval Command Foundation | v1.2 | 3/3 | Complete | 2026-01-20 |
| 10. Eval Projects and Testing | v1.2 | 4/4 | Complete | 2026-01-20 |
| 11. Prompt Engineering | v1.2 | 4/4 | Complete | 2026-01-21 |
| 12. Multi-Trial Results | v1.2 | 5/5 | Complete | 2026-01-22 |
| 13. Parallel Eval TUI | v1.2 | 9/9 | Complete | 2026-01-22 |
| 14. TUI Visual Parity | v1.2 | 6/6 | Complete | 2026-01-23 |
| 13.1 Clippy & Crates.io | v1.2 | 1/1 | Complete | 2026-01-30 |
| 15. Interactive Planning | v1.2 | 7/7 | Complete | 2026-02-01 |
| 16. Cleanup | v1.3 | 0/? | Pending | — |
| 17. Basic Mode Alignment | v1.3 | 0/? | Pending | — |
| 18. TUI Enhancement | v1.3 | 0/? | Pending | — |
| 19. GSD Personas | v1.3 | 0/? | Pending | — |
| 20. E2E Tests | v1.3 | 0/? | Pending | — |

---

## Previous Milestones

<details>
<summary>v1.0 MVP (Phases 1-6) — SHIPPED 2026-01-19</summary>

See `.planning/milestones/v1.0-ROADMAP.md` for full details.

- [x] Phase 1: Foundation (3/3 plans)
- [x] Phase 2: Subprocess Management (2/2 plans)
- [x] Phase 3: Planning Command (2/2 plans)
- [x] Phase 4: Core Build Loop (4/4 plans)
- [x] Phase 5: VCS Integration (2/2 plans)
- [x] Phase 6: TUI Interface (4/4 plans)

</details>

<details>
<summary>v1.1 Testing Enhancement (Phases 7-7.1) — SHIPPED 2026-01-19</summary>

See `.planning/milestones/v1.1-ROADMAP.md` for full details.

- [x] Phase 7: E2E Testing Framework (5/5 plans)
- [x] Phase 7.1: TUI Testing (1/1 plan)

</details>

<details>
<summary>v1.2 Context Engineering (Phases 8-15) — SHIPPED 2026-02-01</summary>

- [x] Phase 8: Token Tracking (4/4 plans)
- [x] Phase 9: Eval Command Foundation (3/3 plans)
- [x] Phase 10: Eval Projects and Testing (4/4 plans)
- [x] Phase 11: Prompt Engineering (4/4 plans)
- [x] Phase 12: Multi-Trial Results (5/5 plans)
- [x] Phase 13: Parallel Eval TUI (9/9 plans)
- [x] Phase 14: TUI Visual Parity (6/6 plans)
- [x] Phase 13.1: Clippy & Crates.io (1/1 plan)
- [x] Phase 15: Interactive Planning (7/7 plans)

</details>

---

## Decisions Log

*Populated during execution*

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| — | — | — |

---

*Last updated: 2026-02-01 — v1.3 Hardening roadmap created*
