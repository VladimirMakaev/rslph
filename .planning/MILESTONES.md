# Project Milestones: rslph

## v1.3 Hardening (Active)

**Started:** 2026-02-01

**Goal:** Consolidate and harden existing features, remove dead code paths, and establish GSD-style multi-persona workflow while ensuring robust TUI input handling.

**Phases:** 16-20 (5 phases, 26 requirements)

**Key deliverables:**
- TUI-only mode for all commands (remove non-TUI code paths)
- Remove gsd_tdd mode
- Basic mode aligned with portableralph reference
- Multiline text input with cursor for interactive Q&A
- GSD multi-persona flow (executor, verifier, researcher, planner)
- Comprehensive E2E tests for planning with 0-3 question rounds
- TUI input prompt tests with ratatui-testlib

---

## v1.2 Context Engineering (Shipped: 2026-02-01)

**Delivered:** Evaluation framework for measuring agent performance with token tracking, multi-trial statistics, parallel evaluation, enhanced TUI with Claude Code visual parity, and interactive planning with session resume.

**Phases completed:** 8-15 (plus 13.1) — 34 plans total

**Key accomplishments:**
- Token consumption tracking in plan/build via stream-json
- `rslph eval` command for controlled benchmarks in isolated environments
- Built-in eval projects (calculator, fizzbuzz) with hidden test suites
- Prompt modes (basic, gsd, gsd_tdd) with TDD flow and deviation handling
- Multi-trial evaluation with mean/variance statistics
- Parallel eval across modes with TUI dashboard
- Enhanced TUI with full LLM conversation display
- Claude Code visual parity (brand colors, box-drawn elements, spinner)
- Interactive planning Q&A via session resume
- Published to crates.io as rslph v0.1.0

**Stats:**
- 34 plans completed
- Average duration: 4m per plan
- Total execution time: 136m 51s

**Git range:** Phase 8 → Phase 15

---

## v1.1 Testing Enhancement (Shipped: 2026-01-19)

**Delivered:** Comprehensive E2E testing infrastructure with fake Claude simulation for deterministic testing, plus TUI snapshot tests using TestBackend and insta.

**Phases completed:** 7-7.1 (6 plans total)

**Key accomplishments:**

- Fake Claude binary with deterministic stream-json responses
- ScenarioBuilder fluent API for configuring test scenarios
- WorkspaceBuilder with fluent API for test fixture setup
- Tool call simulation (Read, Write, Edit, Bash)
- 26 E2E tests (infrastructure + integration + edge cases)
- TUI snapshot testing with TestBackend + insta (8 tests, 7 snapshots)

**Stats:**

- 2,349 lines of Rust added
- 2 phases, 6 plans
- 1 day from start to ship (2026-01-19)

**Git range:** `docs(07)` → `chore: complete v1.1`

**What's next:** TBD

---

## v1.0 MVP (Shipped: 2026-01-19)

**Delivered:** Complete autonomous AI coding agent with CLI/TUI that reads plans, breaks them into tasks, and iteratively executes them using Claude with fresh context per iteration.

**Phases completed:** 1-6 (17 plans total)

**Key accomplishments:**

- TOML config system with layered precedence (defaults < file < env < CLI)
- Claude CLI subprocess management with streaming, timeout, and signal handling
- Basic and adaptive planning modes with stack detection and persona-based clarification
- Autonomous build loop with fresh context, RALPH_DONE detection, and failure memory
- Git and Sapling VCS integration with auto-commit per iteration
- Rich TUI with status bar, live output, collapsible threads, and keyboard navigation

**Stats:**

- 8,762 lines of Rust
- 6 phases, 17 plans, ~135 tests
- 3 days from start to ship (2026-01-17 → 2026-01-19)

**Git range:** `feat(01-01)` → `feat(06-04)`

**What's next:** v1.1 "Prompt Engineering" with E2E testing framework, verification agent, notifications, and prompt overrides.

---
