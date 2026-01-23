# Roadmap: rslph

## Milestones

- v1.0 MVP -- Phases 1-6 (shipped 2026-01-19)
- v1.1 Testing Enhancement -- Phases 7-7.1 (shipped 2026-01-19)
- **v1.2 Context Engineering** -- Phases 8-14 (active)

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

**Plans:** 4 plans

Plans:
- [x] 08-01-PLAN.md — Core token tracking infrastructure (types, accumulation, event routing)
- [x] 08-02-PLAN.md — TUI display and plan command token reporting
- [x] 08-03-PLAN.md — E2E and TUI tests with configurable fake Claude tokens
- [x] 08-04-PLAN.md — Gap closure: fix token accumulation bug and add snapshot tests

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

**Plans:** 3 plans

Plans:
- [x] 09-01-PLAN.md — Core eval module and CLI subcommand
- [x] 09-02-PLAN.md — Eval command implementation (orchestrate plan+build)
- [x] 09-03-PLAN.md — E2E tests and verification

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

**Plans:** 4 plans

Plans:
- [x] 10-01-PLAN.md — Project registry and calculator eval project with include_dir embedding
- [x] 10-02-PLAN.md — Stdin/stdout test runner implementation
- [x] 10-03-PLAN.md — Integrate test runner into eval command and add --list flag
- [x] 10-04-PLAN.md — FizzBuzz eval project and E2E tests

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

**Plans:** 4 plans

Plans:
- [x] 11-01-PLAN.md — Add PromptMode enum, strum dependency, update config
- [x] 11-02-PLAN.md — Create GSD mode prompts (deviation handling, summaries, must-haves)
- [x] 11-03-PLAN.md — Create GSD-TDD mode prompts (TDD flow with escape hatch)
- [x] 11-04-PLAN.md — Basic mode prompts and CLI integration

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
4. User can compare two result files with `rslph compare file1.json file2.json`
5. Comparison shows deltas in pass rate, token consumption, and execution time

**Plans:** 5 plans

Plans:
- [x] 12-01-PLAN.md — Add --trials CLI flag and statistics module
- [x] 12-02-PLAN.md — Multi-trial loop and statistics aggregation
- [x] 12-03-PLAN.md — Multi-trial JSON result serialization
- [x] 12-04-PLAN.md — Compare command with delta display
- [x] 12-05-PLAN.md — E2E tests for multi-trial and compare features

---

### Phase 13: Parallel Eval TUI

**Goal:** Users can run parallel evals across modes with live TUI dashboard and enhanced conversation display

**Dependencies:** Phase 12 (Multi-Trial Results)

**Requirements:**
| ID | Requirement |
|----|-------------|
| PARA-01 | Parallel eval runs across different modes (basic, gsd, gsd_tdd) with --modes flag |
| PARA-02 | TUI dashboard for parallel eval execution showing real-time progress of each trial/mode |
| PARA-03 | Enhanced TUI showing full LLM conversation output (thinking, tool calls, messages) like Claude Code UI |
| PARA-04 | TUI mode for `plan` command matching enhanced TUI style (streaming output, tool calls, thinking blocks) |

**Success Criteria:**
1. User runs `rslph eval calculator --modes basic,gsd,gsd_tdd --trials 3` and sees 9 total trials (3 per mode) run in parallel
2. TUI dashboard shows real-time progress: mode, trial number, current iteration, pass rate, elapsed time
3. Results are grouped by mode in JSON output for comparison
4. User can view full LLM conversation in TUI (thinking blocks, tool calls, text output) in a scrollable view
5. Enhanced TUI applies to `build` command with full conversation display
6. `plan` command has TUI mode showing streaming LLM output, tool calls, thinking blocks, and generated plan preview

**Plans:** 9 plans

Plans:
- [x] 13-01-PLAN.md — --modes flag and parallel execution infrastructure
- [x] 13-02-PLAN.md — Parallel eval dashboard TUI
- [x] 13-03-PLAN.md — Enhanced conversation display
- [x] 13-04-PLAN.md — Plan command TUI mode
- [x] 13-05-PLAN.md — Make plan TUI default with --no-tui to disable (UAT Gap 1)
- [x] 13-06-PLAN.md — Fix task description truncation in progress parser (UAT Gap 2)
- [x] 13-07-PLAN.md — Wire StreamEvent to conversation view (UAT Gap 3)
- [x] 13-08-PLAN.md — Wire iteration progress to dashboard TUI (Audit Gap 0 - blocker)
- [x] 13-09-PLAN.md — Mode passthrough to plan/build commands (Audit Gap 1)

---

### Phase 14: TUI Visual Parity with Claude Code

**Goal:** Align TUI visual design with Claude Code's interface for consistent user experience

**Dependencies:** Phase 13 (Parallel Eval TUI)

**Requirements:**
| ID | Requirement |
|----|-------------|
| TUI-01 | Claude brand color palette (Crail #C15F3C, Cloudy #B1ADA1) with centralized theme |
| TUI-02 | Box-drawn thinking blocks with collapse/expand toggle |
| TUI-03 | Box-drawn tool call containers with tool name header and indented parameters |
| TUI-04 | Animated braille spinner during LLM streaming |
| TUI-05 | Enhanced status bar with model tier indicator (◆◇○), token bar, session timer |
| TUI-06 | Box-drawn message borders with type-specific styling |

**Success Criteria:**
1. TUI uses Claude brand colors via centralized `theme.rs` module
2. Thinking blocks display with box borders and are collapsible
3. Tool calls show as box containers with indented content
4. Animated spinner shows during LLM response streaming
5. Status bar shows model indicator, token usage bar, and session timer
6. Messages have distinct box-drawn borders per type (assistant, tool, system)

**Plans:** 6 plans

Plans:
- [x] 14-01-PLAN.md — Centralized theme module with Claude brand colors (TUI-01)
- [x] 14-02-PLAN.md — Animated braille spinner widget (TUI-04)
- [x] 14-03-PLAN.md — Enhanced status bar with model tier and timer (TUI-05)
- [x] 14-04-PLAN.md — Box-drawn thinking blocks and tool containers (TUI-02, TUI-03)
- [x] 14-05-PLAN.md — Themed message borders in thread view (TUI-06)
- [x] 14-06-PLAN.md — Integration and key bindings for visual features

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

*Last updated: 2026-01-23 — Phase 14 complete (6/6 plans)*
