# Roadmap: rslph

## Milestones

- v1.0 MVP -- Phases 1-6 (shipped 2026-01-19)
- v1.1 Testing Enhancement -- Phases 7-7.1 (shipped 2026-01-19)
- **v1.2 Context Engineering** -- Phases 8-12 (active)

## Overview

Current milestone: v1.2 "Context Engineering" — Build an evaluation framework to measure agent performance and improve the context engineering (prompts, iteration structure, test-driven flow) that drives autonomous execution.

---

## v1.2 Context Engineering

### Phase 8: Token Tracking

**Goal:** Users can observe token consumption during plan/build execution

**Dependencies:** None (foundational)

**Requirements:**
| ID | Requirement |
|----|-------------|
| TOK-01 | Track input/output tokens per iteration from stream-json |
| TOK-02 | Track cache tokens (creation and read) per iteration |
| TOK-03 | Sum total tokens consumed across all iterations |
| TOK-04 | Store token metrics in build state for persistence |

**Success Criteria:**
1. User sees per-iteration token counts (input, output, cache_creation, cache_read) in build output
2. User sees cumulative token totals at end of build command
3. Token counts survive iteration boundaries via BuildState persistence
4. Plan command reports token consumption for planning phase

**Plans:** 3 plans

Plans:
- [x] 08-01-PLAN.md — Core token tracking infrastructure (types, accumulation, event routing)
- [x] 08-02-PLAN.md — TUI display and plan command token reporting
- [x] 08-03-PLAN.md — E2E and TUI tests with configurable fake Claude tokens

---

### Phase 9: Eval Command Foundation

**Goal:** Users can run controlled benchmarks in isolated environments

**Dependencies:** Phase 8 (Token Tracking)

**Requirements:**
| ID | Requirement |
|----|-------------|
| EVAL-01 | `rslph eval <project>` command runs plan+build in isolated temp directory |
| EVAL-04 | Track total execution time |
| EVAL-05 | Track total token consumption across plan+build |

**Success Criteria:**
1. User runs `rslph eval calculator` and sees plan+build execute in a fresh temp directory
2. User sees total execution time reported at completion
3. User sees total token consumption (from Phase 8) aggregated across plan and build phases
4. Temp directory is cleaned up after eval completes (or preserved with flag)

---

### Phase 10: Eval Projects and Testing

**Goal:** Users can evaluate agent performance against built-in projects with hidden tests

**Dependencies:** Phase 9 (Eval Command Foundation)

**Requirements:**
| ID | Requirement |
|----|-------------|
| PROJ-01 | Calculator eval project with starting prompt |
| PROJ-02 | Test runner script (language-agnostic, checks stdin/stdout pairs) |
| PROJ-03 | Test data file with input/expected output pairs (hidden from agent) |
| PROJ-04 | Second eval project of medium difficulty (TBD scope) |
| EVAL-02 | Execute hidden test runner after build completes (black-box input/output testing) |
| EVAL-03 | Track pass rate (passing/total test cases) |

**Success Criteria:**
1. User runs `rslph eval calculator` and agent attempts to implement a calculator from the prompt
2. After build completes, hidden tests execute automatically against the built artifact
3. User sees pass rate displayed (e.g., "Tests: 8/10 passed (80%)")
4. User can list available eval projects with `rslph eval --list`
5. Hidden test data is NOT visible to the agent during build (verified by location outside project dir)

---

### Phase 11: Prompt Engineering

**Goal:** Agent follows test-driven development with clear iteration guidance

**Dependencies:** Phase 10 (Eval Projects) - need working evals to validate prompt improvements

**Requirements:**
| ID | Requirement |
|----|-------------|
| PROMPT-01 | Add deviation handling rules to build prompt |
| PROMPT-02 | Add substantive completion summary format |
| PROMPT-03 | TDD iteration flow (write tests -> implement -> refactor) |
| PROMPT-04 | Configurable TDD mode (enable/disable via config flag) |
| PROMPT-05 | Research and adopt GSD patterns (phases, research structure) |

**Success Criteria:**
1. Build prompt includes deviation handling rules (bugs, missing deps, blocking issues)
2. Iteration completion summaries include substantive details (what changed, what's next)
3. With TDD mode enabled, agent writes failing tests before implementation
4. User can enable/disable TDD mode via `rslph.toml` configuration
5. Agent iteration structure reflects GSD patterns for structured task execution

---

### Phase 12: Multi-Trial Results

**Goal:** Users can run multiple trials and compare results across runs

**Dependencies:** Phase 10 (Eval Projects) - need test execution for meaningful results

**Requirements:**
| ID | Requirement |
|----|-------------|
| EVAL-06 | Support multiple trial runs with configurable count |
| EVAL-07 | Report mean/variance across trials |
| EVAL-08 | Store results in JSON file |
| EVAL-09 | Compare results between different runs |

**Success Criteria:**
1. User runs `rslph eval calculator --trials 5` and sees 5 independent runs execute
2. User sees statistical summary (mean pass rate, variance, min/max) after trials complete
3. Results are saved to JSON file (e.g., `eval-results-calculator-2026-01-20.json`)
4. User can compare two result files with `rslph eval compare file1.json file2.json`
5. Comparison shows deltas in pass rate, token consumption, and execution time

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
| 8. Token Tracking | v1.2 | 3/3 | Complete | 2026-01-20 |
| 9. Eval Command Foundation | v1.2 | 0/? | Pending | — |
| 10. Eval Projects and Testing | v1.2 | 0/? | Pending | — |
| 11. Prompt Engineering | v1.2 | 0/? | Pending | — |
| 12. Multi-Trial Results | v1.2 | 0/? | Pending | — |

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

---

## Decisions Log

*Populated during execution*

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| — | — | — |

---

*Last updated: 2026-01-20*
