# Ralph (rslph)

## What This Is

A Rust CLI application implementing the Ralph Wiggum Loop — an autonomous AI coding agent that reads a plan, breaks it into tasks, and iteratively executes them using Claude. Each iteration starts with fresh context, preventing context pollution while preserving learnings through a progress file. Features a modern TUI for monitoring execution.

## Core Value

Autonomous task execution with fresh context per iteration and accumulated learnings — enabling Claude to complete complex multi-step plans without human intervention while avoiding context window exhaustion.

## Requirements

### Validated

(None yet — ship to validate)

### Active

#### Commands
- [ ] `ralph plan <plan>` — Transform idea/plan into structured progress file with tasks
- [ ] `ralph build <plan>` — Execute tasks iteratively, piloting Claude CLI headlessly

#### Planning Phase
- [ ] Adaptive vagueness detection — ask clarifying questions only when input is vague
- [ ] Requirements clarifier persona — surfaces ambiguity before structuring
- [ ] Testing strategist persona — determines verification approach for the project
- [ ] Auto-detect project stack and confirm testing setup with user
- [ ] Multi-layer testing strategy (unit, type-check, static analysis, e2e)
- [ ] Output structured progress file ready for build phase

#### Build Phase
- [ ] Headless Claude CLI execution as subprocess
- [ ] Fresh context per iteration (prevents context pollution)
- [ ] Configurable max iterations — stop when exhausted regardless of completion
- [ ] Progress file accumulates learnings between iterations
- [ ] Recent attempts section (configurable depth) for failure memory
- [ ] Task completion via checkbox marking (`- [x]` / `- [ ]`)
- [ ] RALPH_DONE marker detection for early termination

#### TUI Interface
- [ ] Top status bar: iteration X/Y remaining, task X/Y remaining, log link
- [ ] Model display (e.g., "Opus 4.5")
- [ ] Folder/project name display
- [ ] Context usage progress bar (visual percentage)
- [ ] Live Claude output stream in main area
- [ ] Collapsible conversation threads
- [ ] Configurable number of recent threads to display

#### Configuration
- [ ] TOML config file support (`~/.config/ralph/config.toml`)
- [ ] CLI argument overrides for all config options
- [ ] Override Claude command path
- [ ] Override system prompts (plan + build) via file paths
- [ ] Override shell for notify script execution
- [ ] Max iterations setting
- [ ] Recent threads count setting

#### System Prompts
- [ ] Baked-in default prompts compiled into binary
- [ ] PROMPT_plan — instructions for task decomposition and clarification
- [ ] PROMPT_build — instructions for task execution and progress updates
- [ ] User-overridable via `~/.config/ralph/` with paths in config

#### Notifications
- [ ] Notify script execution on completion/failure
- [ ] Script called via configurable shell
- [ ] User provides their own notification logic (desktop, sound, webhook, etc.)

#### Progress File Format
- [ ] Status section with RALPH_DONE marker
- [ ] Analysis/research section
- [ ] Task list with phases and checkboxes
- [ ] Testing strategy section
- [ ] Completed This Iteration section
- [ ] Recent Attempts section (last N iterations for memory)
- [ ] Iteration log for historical record

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

**Research sources:**
- [kylemclaren/ralph](https://github.com/kylemclaren/ralph) — CLI with maxIterations, progress.txt, prd.json pattern
- [PJFP Guide](https://pjfp.com/what-is-the-ralph-wiggum-loop-in-programming-ultimate-guide-to-ai-powered-iterative-coding/) — Comprehensive overview of the pattern

## Constraints

- **Language**: Rust — self-contained binary, no runtime dependencies
- **Claude integration**: CLI subprocess only — shell out to `claude` command
- **TUI library**: Modern Rust TUI (ratatui or similar)
- **Config format**: TOML for configuration files
- **Platform**: Cross-platform (macOS, Linux primarily)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Claude CLI subprocess over API | Simpler auth, leverages existing claude CLI setup, user already authenticated | — Pending |
| TOML for config | Standard Rust ecosystem choice, human-readable | — Pending |
| Adaptive planning over always-ask | Better UX for detailed plans, only interrupt when value-add | — Pending |
| Progress file as memory | Proven pattern from reference implementations, survives context resets | — Pending |
| Two-phase prompts (plan/build) | Separation of concerns, different personas for different tasks | — Pending |

---
*Last updated: 2026-01-17 after initialization*
