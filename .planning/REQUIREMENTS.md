# Requirements: rslph

**Defined:** 2026-01-17
**Core Value:** Autonomous task execution with fresh context per iteration and accumulated learnings

## v1 Requirements

### Commands

- [ ] **CMD-01**: `rslph plan <plan>` command transforms idea/plan into structured progress file
- [ ] **CMD-02**: `rslph build <plan>` command executes tasks iteratively with fresh context
- [ ] **CMD-03**: Basic planning mode (default) — best-effort structuring without user questions
- [ ] **CMD-04**: Adaptive planning mode (`--adaptive`) — clarifying questions + testing strategy personas

### Core Loop

- [ ] **LOOP-01**: Autonomous iteration loop with configurable max iterations
- [ ] **LOOP-02**: Progress persistence — resume after interruption
- [ ] **LOOP-03**: Task/story tracking — done vs pending status with checkboxes
- [ ] **LOOP-04**: Completion detection via RALPH_DONE marker in progress file
- [ ] **LOOP-05**: Max iteration limits with sensible default (stop when exhausted)
- [ ] **LOOP-06**: Single iteration mode (`--once`) for step-by-step execution
- [ ] **LOOP-07**: Dry-run mode (`--dry-run`) to preview without executing
- [ ] **LOOP-08**: Fresh context per iteration — each Claude invocation starts clean
- [ ] **LOOP-09**: Recent attempts section (configurable depth) for failure memory

### VCS Integration

- [ ] **VCS-01**: Git auto-commit per iteration for rollback safety
- [ ] **VCS-02**: Sapling (sl) support as alternative to git
- [ ] **VCS-03**: Auto-detect which VCS is in use (git vs sl)

### TUI Interface

- [ ] **TUI-01**: Top status bar with iteration X/Y remaining
- [ ] **TUI-02**: Top status bar with task X/Y remaining
- [ ] **TUI-03**: Link/path to full conversation log
- [ ] **TUI-04**: Model display (e.g., "Opus 4.5")
- [ ] **TUI-05**: Folder/project name display
- [ ] **TUI-06**: Context usage progress bar (visual percentage)
- [ ] **TUI-07**: Live Claude output stream in main area
- [ ] **TUI-08**: Collapsible conversation threads
- [ ] **TUI-09**: Configurable number of recent threads to display
- [ ] **TUI-10**: Keyboard navigation for thread expansion/collapse

### Configuration

- [ ] **CFG-01**: TOML config file support (`~/.config/rslph/config.toml`)
- [ ] **CFG-02**: CLI argument overrides for all config options
- [ ] **CFG-03**: Override Claude command path
- [ ] **CFG-04**: Override system prompts (plan + build) via file paths
- [ ] **CFG-05**: Override shell for notify script execution
- [ ] **CFG-06**: Max iterations setting
- [ ] **CFG-07**: Recent threads count setting
- [ ] **CFG-08**: Notification interval setting (every X iterations)

### System Prompts

- [ ] **PROMPT-01**: Baked-in default prompts compiled into binary
- [ ] **PROMPT-02**: PROMPT_plan — instructions for task decomposition and clarification
- [ ] **PROMPT-03**: PROMPT_build — instructions for task execution and progress updates
- [ ] **PROMPT-04**: User-overridable via `~/.config/rslph/` with paths in config

### Notifications

- [ ] **NOTIF-01**: Notify after 1st iteration
- [ ] **NOTIF-02**: Notify every X iterations (configurable, default 10)
- [ ] **NOTIF-03**: Notify on completion
- [ ] **NOTIF-04**: Notify on failure
- [ ] **NOTIF-05**: Notify via user-provided script executed with configurable shell

### Planning Phase

- [ ] **PLAN-01**: Detect input vagueness (for adaptive mode)
- [ ] **PLAN-02**: Requirements clarifier persona — surfaces ambiguity (adaptive only)
- [ ] **PLAN-03**: Testing strategist persona — determines verification approach (adaptive only)
- [ ] **PLAN-04**: Auto-detect project stack for testing strategy
- [ ] **PLAN-05**: Multi-layer testing strategy (unit, type-check, static analysis, e2e)
- [ ] **PLAN-06**: Output structured progress file ready for build phase

### Verification

- [ ] **VERIF-01**: Testing strategy captured during planning phase
- [ ] **VERIF-02**: Separate test agent persona (not embedded in build loop)
- [ ] **VERIF-03**: Test agent runs verification plan independently

### Progress File Format

- [ ] **PROG-01**: Status section with RALPH_DONE marker
- [ ] **PROG-02**: Analysis/research section
- [ ] **PROG-03**: Task list with phases and checkboxes
- [ ] **PROG-04**: Testing strategy section
- [ ] **PROG-05**: Completed This Iteration section
- [ ] **PROG-06**: Recent Attempts section (last N iterations for memory)
- [ ] **PROG-07**: Iteration log for historical record

### Subprocess Management

- [ ] **PROC-01**: Pilot Claude CLI as subprocess
- [ ] **PROC-02**: Async subprocess with streaming output capture
- [ ] **PROC-03**: Graceful Ctrl+C handling (save state, preserve progress)
- [ ] **PROC-04**: Timeout handling for stuck Claude invocations

## v2 Requirements

### Advanced Hooks

- **HOOK-01**: onStart hook — run scripts before loop begins
- **HOOK-02**: onIteration hook — custom logic each iteration
- **HOOK-03**: Webhook notifications (POST to URL with status JSON)

### Session Management

- **SESS-01**: Session persistence — resume previous sessions
- **SESS-02**: Auto-compaction — handle context window limits

### Multi-Agent

- **AGENT-01**: Parallel research agents for faster PRD generation
- **AGENT-02**: Git worktree support for parallel feature development

### Advanced TUI

- **TUI-11**: Multi-pane layout (task list, output, progress summary)
- **TUI-12**: Vim-like keyboard bindings
- **TUI-13**: tmux integration for background operation

### Cost Tracking

- **COST-01**: Track tokens/iterations
- **COST-02**: Estimated cost display

## Out of Scope

| Feature | Reason |
|---------|--------|
| Claude API direct integration | CLI subprocess only — simpler auth, leverages existing claude CLI |
| GUI application | CLI/TUI only — different product category |
| Multi-model support | Claude only via Claude CLI — focus on one integration |
| Plugin system | Configuration via TOML/CLI sufficient for v1 |
| Cloud sync | Local-only operation |
| Auto-push to remote | Safety — user explicitly pushes |
| Unlimited iterations by default | Cost explosion risk — require explicit limit |
| Interactive chat mode | Different product — this is autonomous loop |
| Browser/web UI | Scope creep — stay CLI/TUI focused |
| Project scaffolding | Not core value — use existing project structures |
| File watching/hot reload | Iteration-based, not reactive |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| CMD-01 | Phase 1 | Complete |
| CMD-02 | Phase 1 | Complete |
| CMD-03 | Phase 3 | Pending |
| CMD-04 | Phase 3 | Pending |
| LOOP-01 | Phase 4 | Pending |
| LOOP-02 | Phase 4 | Pending |
| LOOP-03 | Phase 4 | Pending |
| LOOP-04 | Phase 4 | Pending |
| LOOP-05 | Phase 4 | Pending |
| LOOP-06 | Phase 4 | Pending |
| LOOP-07 | Phase 4 | Pending |
| LOOP-08 | Phase 4 | Pending |
| LOOP-09 | Phase 4 | Pending |
| VCS-01 | Phase 5 | Pending |
| VCS-02 | Phase 5 | Pending |
| VCS-03 | Phase 5 | Pending |
| TUI-01 | Phase 6 | Pending |
| TUI-02 | Phase 6 | Pending |
| TUI-03 | Phase 6 | Pending |
| TUI-04 | Phase 6 | Pending |
| TUI-05 | Phase 6 | Pending |
| TUI-06 | Phase 6 | Pending |
| TUI-07 | Phase 6 | Pending |
| TUI-08 | Phase 6 | Pending |
| TUI-09 | Phase 6 | Pending |
| TUI-10 | Phase 6 | Pending |
| CFG-01 | Phase 1 | Complete |
| CFG-02 | Phase 1 | Complete |
| CFG-03 | Phase 1 | Complete |
| CFG-04 | Phase 1 | Complete |
| CFG-05 | Phase 1 | Complete |
| CFG-06 | Phase 1 | Complete |
| CFG-07 | Phase 1 | Complete |
| CFG-08 | Phase 1 | Complete |
| PROMPT-01 | Phase 3 | Pending |
| PROMPT-02 | Phase 3 | Pending |
| PROMPT-03 | Phase 4 | Pending |
| PROMPT-04 | Phase 8 | Pending |
| NOTIF-01 | Phase 8 | Pending |
| NOTIF-02 | Phase 8 | Pending |
| NOTIF-03 | Phase 8 | Pending |
| NOTIF-04 | Phase 8 | Pending |
| NOTIF-05 | Phase 8 | Pending |
| PLAN-01 | Phase 3 | Pending |
| PLAN-02 | Phase 3 | Pending |
| PLAN-03 | Phase 3 | Pending |
| PLAN-04 | Phase 3 | Pending |
| PLAN-05 | Phase 3 | Pending |
| PLAN-06 | Phase 3 | Pending |
| VERIF-01 | Phase 7 | Pending |
| VERIF-02 | Phase 7 | Pending |
| VERIF-03 | Phase 7 | Pending |
| PROG-01 | Phase 1 | Complete |
| PROG-02 | Phase 1 | Complete |
| PROG-03 | Phase 1 | Complete |
| PROG-04 | Phase 1 | Complete |
| PROG-05 | Phase 1 | Complete |
| PROG-06 | Phase 1 | Complete |
| PROG-07 | Phase 1 | Complete |
| PROC-01 | Phase 2 | Complete |
| PROC-02 | Phase 2 | Complete |
| PROC-03 | Phase 2 | Complete |
| PROC-04 | Phase 2 | Complete |

**Coverage:**
- v1 requirements: 54 total
- Mapped to phases: 54
- Unmapped: 0

---
*Requirements defined: 2026-01-17*
*Last updated: 2026-01-17 after roadmap creation*
