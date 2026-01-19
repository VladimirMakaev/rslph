# Roadmap: rslph

## Overview

Build rslph from foundation to polished CLI/TUI in 8 phases. Start with configuration and progress file parsing, add subprocess management for Claude CLI, implement the plan and build commands, add VCS integration, layer on the TUI, complete verification capabilities, and finish with notifications and prompt customization. Each phase delivers independently testable functionality.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Foundation** - Config loading, CLI skeleton, progress file format
- [x] **Phase 2: Subprocess Management** - Claude CLI spawning, streaming output, process control
- [x] **Phase 3: Planning Command** - `rslph plan` with basic and adaptive modes
- [x] **Phase 4: Core Build Loop** - Iteration logic, completion detection, fresh context
- [x] **Phase 5: VCS Integration** - Git and Sapling auto-commit per iteration
- [x] **Phase 6: TUI Interface** - Status bar, live output, collapsible threads
- [ ] **Phase 7: Verification** - Test agent and independent verification
- [ ] **Phase 8: Notifications and Polish** - Notify scripts, prompt overrides, error refinement

## Phase Details

### Phase 1: Foundation
**Goal**: Core infrastructure exists — config loads correctly, CLI parses commands, progress files can be read and written atomically
**Depends on**: Nothing (first phase)
**Requirements**: CFG-01, CFG-02, CFG-03, CFG-04, CFG-05, CFG-06, CFG-07, CFG-08, CMD-01, CMD-02, PROG-01, PROG-02, PROG-03, PROG-04, PROG-05, PROG-06, PROG-07
**Success Criteria** (what must be TRUE):
  1. User can run `rslph --help` and see plan/build subcommands
  2. User can create `~/.config/rslph/config.toml` and settings are loaded
  3. CLI arguments override config file values (precedence works correctly)
  4. Progress file with all sections (status, analysis, tasks, testing, attempts, log) can be parsed and written
  5. Progress file writes are atomic (crash-safe via temp file + rename)
**Plans**: 3 plans in 2 waves

Plans:
- [x] 01-01-PLAN.md — Config system (TOML loading, defaults, validation) [Wave 1]
- [x] 01-02-PLAN.md — CLI parser (clap, subcommands, argument merging) [Wave 2, depends on 01-01]
- [x] 01-03-PLAN.md — Progress file parser/writer (markdown format, atomic writes) [Wave 1]

### Phase 2: Subprocess Management
**Goal**: Claude CLI can be spawned, output streamed in real-time, and process lifecycle managed safely
**Depends on**: Phase 1
**Requirements**: PROC-01, PROC-02, PROC-03, PROC-04
**Success Criteria** (what must be TRUE):
  1. Claude CLI runs as subprocess with piped stdout/stderr
  2. Output streams line-by-line in real-time (no buffer deadlock)
  3. Ctrl+C gracefully terminates Claude and saves current state
  4. Stuck Claude invocations timeout after configurable duration
  5. No zombie processes accumulate across iterations
**Plans**: 2 plans in 2 waves

Plans:
- [x] 02-01-PLAN.md - Subprocess spawning and streaming (ClaudeRunner struct, piped I/O, concurrent stream reading) [Wave 1]
- [x] 02-02-PLAN.md - Signal handling and timeout management (CancellationToken, graceful shutdown, timeout) [Wave 2, depends on 02-01]

### Phase 3: Planning Command
**Goal**: `rslph plan` transforms ideas into structured progress files, with optional adaptive mode for vague inputs
**Depends on**: Phase 2
**Requirements**: CMD-03, CMD-04, PLAN-01, PLAN-02, PLAN-03, PLAN-04, PLAN-05, PLAN-06, PROMPT-01, PROMPT-02
**Success Criteria** (what must be TRUE):
  1. User can run `rslph plan "build a todo app"` and get a progress.md file
  2. Basic mode (default) produces structured tasks without asking questions
  3. Adaptive mode (`--adaptive`) detects vagueness and asks clarifying questions
  4. Project stack is auto-detected and testing strategy included in output
  5. PROMPT_plan is baked into binary but can be overridden via config
**Plans**: 2 plans in 2 waves

Plans:
- [x] 03-01-PLAN.md — Basic planning mode (prompt system, stack detection, Claude execution) [Wave 1]
- [x] 03-02-PLAN.md — Adaptive mode (vagueness detection, personas, multi-turn conversation) [Wave 2, depends on 03-01]

### Phase 4: Core Build Loop
**Goal**: `rslph build` executes tasks iteratively with fresh context, completion detection, and configurable limits
**Depends on**: Phase 3
**Requirements**: LOOP-01, LOOP-02, LOOP-03, LOOP-04, LOOP-05, LOOP-06, LOOP-07, LOOP-08, LOOP-09, PROMPT-03
**Success Criteria** (what must be TRUE):
  1. User can run `rslph build progress.md` and tasks execute autonomously
  2. Each iteration starts with fresh Claude context (no context pollution)
  3. Progress persists — interrupted runs resume from last checkpoint
  4. RALPH_DONE marker in progress file stops the loop early
  5. Loop stops at max iterations (configurable, sensible default)
  6. `--once` runs single iteration, `--dry-run` previews without executing
  7. Recent attempts section accumulates failure memory across iterations
**Plans**: 4 plans in 2 waves

Plans:
- [x] 04-01-PLAN.md — Build module foundation (state machine, PROMPT_build, iteration logic) [Wave 1]
- [x] 04-02-PLAN.md — Completion detection and loop termination (RALPH_DONE, max iterations, attempts) [Wave 2, depends on 04-01]
- [x] 04-03-PLAN.md — Single iteration and dry-run modes (--once, --dry-run) [Wave 2, depends on 04-01]
- [x] 04-04-PLAN.md — Fix claude_path resolution for subprocess spawn (gap closure) [Wave 1]

### Phase 5: VCS Integration
**Goal**: Each iteration auto-commits for rollback safety, supporting both Git and Sapling
**Depends on**: Phase 4
**Requirements**: VCS-01, VCS-02, VCS-03
**Success Criteria** (what must be TRUE):
  1. Git repositories auto-commit after each iteration with descriptive message
  2. Sapling (sl) repositories auto-commit identically to Git
  3. VCS type is auto-detected (no user configuration required)
  4. User can roll back to any iteration via standard VCS commands
**Plans**: 2 plans in 1 wave

Plans:
- [x] 05-01-PLAN.md — VCS module with Git/Sapling implementations, auto-detection, build loop integration [Wave 1]
- [x] 05-02-PLAN.md — Fix commit message project name and Sapling hash retrieval (gap closure) [Wave 1]

### Phase 6: TUI Interface
**Goal**: Rich terminal UI displays status, live output, and collapsible conversation threads
**Depends on**: Phase 4
**Requirements**: TUI-01, TUI-02, TUI-03, TUI-04, TUI-05, TUI-06, TUI-07, TUI-08, TUI-09, TUI-10
**Success Criteria** (what must be TRUE):
  1. Status bar shows iteration X/Y remaining and task X/Y remaining
  2. Model name and folder/project name displayed in header
  3. Context usage progress bar shows visual percentage
  4. Live Claude output streams in main area without blocking
  5. Conversation threads are collapsible with configurable recent count
  6. Keyboard navigation works (scroll, expand/collapse, quit)
  7. Link/path to full log is accessible
**Plans**: 4 plans in 3 waves

Plans:
- [x] 06-01-PLAN.md — Terminal setup, panic hooks, app state, event handler [Wave 1]
- [x] 06-02-PLAN.md — Status bar header and progress bar widgets [Wave 2, depends on 06-01]
- [x] 06-03-PLAN.md — Live output view with scrolling and ClaudeRunner channel integration [Wave 2, depends on 06-01]
- [x] 06-04-PLAN.md — Thread view, keyboard navigation, build command integration [Wave 3, depends on 06-02, 06-03]

### Phase 7: Verification
**Goal**: Test agent runs independently to verify build results
**Depends on**: Phase 4
**Requirements**: VERIF-01, VERIF-02, VERIF-03
**Success Criteria** (what must be TRUE):
  1. Testing strategy from planning phase is captured and available
  2. Separate test agent persona (not embedded in build loop) can execute
  3. Verification runs independently and reports pass/fail
**Plans**: TBD

Plans:
- [ ] 07-01: Test agent implementation and verification runner

### Phase 8: Notifications and Polish
**Goal**: User receives notifications at key points, prompts are fully customizable
**Depends on**: Phase 6, Phase 7
**Requirements**: NOTIF-01, NOTIF-02, NOTIF-03, NOTIF-04, NOTIF-05, PROMPT-04
**Success Criteria** (what must be TRUE):
  1. User-provided notify script runs after 1st iteration
  2. Notify script runs every X iterations (configurable)
  3. Notify script runs on completion and on failure
  4. Script executes via configurable shell with context (status, counts)
  5. User can override PROMPT_plan and PROMPT_build via config file paths
**Plans**: TBD

Plans:
- [ ] 08-01: Notification system
- [ ] 08-02: Prompt override system and final polish

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 3/3 | Complete | 2026-01-17 |
| 2. Subprocess Management | 2/2 | Complete | 2026-01-17 |
| 3. Planning Command | 2/2 | Complete | 2026-01-18 |
| 4. Core Build Loop | 4/4 | Complete | 2026-01-18 |
| 5. VCS Integration | 2/2 | Complete | 2026-01-18 |
| 6. TUI Interface | 4/4 | Complete | 2026-01-19 |
| 7. Verification | 0/1 | Not started | - |
| 8. Notifications and Polish | 0/2 | Not started | - |
