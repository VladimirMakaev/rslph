# rslph

## What This Is

A Rust CLI application implementing the Ralph Wiggum Loop — an autonomous AI coding agent that reads a plan, breaks it into tasks, and iteratively executes them using Claude. Each iteration starts with fresh context, preventing context pollution while preserving learnings through a progress file. Features a modern TUI for monitoring execution.

## Current State

**Version:** v1.1 "Testing Enhancement" (shipped 2026-01-19)
**Lines of code:** ~11,000 Rust
**Tech stack:** Rust, ratatui (TUI), figment (config), clap (CLI), tokio (async)

**What works:**
- `rslph plan "idea"` — generates structured progress file with adaptive mode
- `rslph build progress.md` — iterates autonomously with fresh context, auto-commits
- Rich TUI with status bar, live output, collapsible threads, keyboard navigation
- Git and Sapling VCS support with auto-detection
- Fake Claude binary for deterministic E2E testing
- ScenarioBuilder/WorkspaceBuilder fluent APIs for test fixtures
- 26 E2E tests + TUI snapshot testing

## Current Milestone: v1.3 Hardening

**Goal:** Consolidate and harden existing features, remove dead code paths, and establish GSD-style multi-persona workflow while ensuring robust TUI input handling.

**Target features:**
- TUI-only mode for all commands (remove non-TUI code paths)
- Proper multiline text input with cursor for interactive Q&A
- `--basic` mode aligned with portableralph reference
- Remove `gsd_tdd` mode, replace with GSD multi-persona flow
- GSD personas (research, planning, execution, verification) compatible with Ralph Loop
- Comprehensive E2E tests for planning with 0-3 question rounds
- TUI input prompt tests with ratatui-testlib

## Core Value

Autonomous task execution with fresh context per iteration and accumulated learnings — enabling Claude to complete complex multi-step plans without human intervention while avoiding context window exhaustion.

## Requirements

### Validated (v1.0, v1.1, v1.2)

- ✓ `rslph plan <plan>` command with basic and adaptive modes — v1.0
- ✓ `rslph build <plan>` command with autonomous iteration — v1.0
- ✓ Fresh context per iteration with progress file persistence — v1.0
- ✓ RALPH_DONE marker detection for early termination — v1.0
- ✓ Git and Sapling VCS auto-commit per iteration — v1.0
- ✓ Rich TUI with status bar, live output, collapsible threads — v1.0
- ✓ TOML config with layered precedence (defaults < file < env < CLI) — v1.0
- ✓ Ctrl+C graceful handling and timeout support — v1.0
- ✓ E2E Testing Framework with fake Claude simulation — v1.1
- ✓ TUI snapshot testing with TestBackend + insta — v1.1
- ✓ Token consumption tracking in plan/build via stream-json — v1.2
- ✓ `rslph eval` command for controlled benchmarks — v1.2
- ✓ Built-in eval projects with hidden test suites — v1.2
- ✓ Prompt modes (basic, gsd, gsd_tdd) — v1.2
- ✓ Multi-trial evaluation with statistics — v1.2
- ✓ Parallel eval TUI with dashboard — v1.2
- ✓ Interactive planning Q&A via session resume — v1.2

### Active (v1.3)

- [ ] TUI-only mode for all commands (remove non-TUI code)
- [ ] Multiline text input with cursor for Q&A
- [ ] `--basic` mode aligned with portableralph
- [ ] Remove `gsd_tdd` mode
- [ ] GSD multi-persona flow (research, plan, execute, verify)
- [ ] E2E tests for planning with 0-3 question rounds
- [ ] TUI input tests with ratatui-testlib

### Future

- [ ] Verification agent (separate from build loop)
- [ ] Notification system (completion, failure, intervals)
- [ ] User-overridable prompts via config file paths

### Out of Scope

- Claude API direct integration — using CLI subprocess only (simpler, leverages existing auth)
- GUI application — CLI/TUI only
- Multi-model support — Claude only via Claude CLI
- Plugin system — configuration via TOML/CLI sufficient for v1
- Cloud sync — local-only operation

## Context

**Pattern origin:** The Ralph Wiggum Loop originated with developer Geoffrey Huntley in late 2025. Named after The Simpsons character who persists despite setbacks, it emphasizes iterative self-correction over single-pass perfection.

**Reference implementation:** [portableralph](https://github.com/aaron777collins/portableralph) — Bash-based implementation with PROMPT_plan.md and PROMPT_build.md structure.

**Key insight:** The progress file IS the memory. Each iteration Claude reads accumulated learnings, sees what was tried, and avoids repeating failures. Fresh context + persistent file = best of both worlds.

## Constraints

- **Language**: Rust — self-contained binary, no runtime dependencies
- **Claude integration**: CLI subprocess only — shell out to `claude` command
- **TUI library**: ratatui
- **Config format**: TOML for configuration files
- **Platform**: Cross-platform (macOS, Linux primarily)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Claude CLI subprocess over API | Simpler auth, leverages existing claude CLI setup | ✓ Good |
| TOML for config | Standard Rust ecosystem choice, human-readable | ✓ Good |
| Adaptive planning over always-ask | Better UX for detailed plans, only interrupt when value-add | ✓ Good |
| Progress file as memory | Proven pattern from reference implementations, survives context resets | ✓ Good |
| Two-phase prompts (plan/build) | Separation of concerns, different personas for different tasks | ✓ Good |
| TUI stderr backend | Keeps stdout available for non-TUI output | ✓ Good |
| VCS shell out over git2 crate | Simpler, no C dependency, supports both Git and Sapling | ✓ Good |

## Technical Debt

- **STREAM-JSON-RESEARCH**: Need to research Claude CLI `--output-format stream-json` and `--json-schema` flags for correct usage

---
*Last updated: 2026-02-01 — v1.3 Hardening milestone started*
