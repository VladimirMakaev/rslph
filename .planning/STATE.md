# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-01-17)

**Core value:** Autonomous task execution with fresh context per iteration and accumulated learnings
**Current focus:** Phase 6 - TUI Interface (Complete)

## Current Position

Phase: 6 of 8 (TUI Interface)
Plan: 4 of 4 in current phase (COMPLETE)
Status: Phase complete
Last activity: 2026-01-19 - Completed 06-04-PLAN.md

Progress: [██████████] 100% (Phase 6 complete)

## Performance Metrics

**Velocity:**
- Total plans completed: 16
- Average duration: 5m 31s
- Total execution time: 1.47 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01-foundation | 3/3 | 22m 5s | 7m 22s |
| 02-subprocess-management | 2/2 | 6m 29s | 3m 15s |
| 03-planning-command | 2/2 | 16m | 8m |
| 04-core-build-loop | 4/4 | 22m 41s | 5m 40s |
| 05-vcs-integration | 1/1 | 4m | 4m |
| 06-tui-interface | 4/4 | 17m 4s | 4m 16s |

**Recent Trend:**
- Last 5 plans: 06-01 (2m 27s), 06-02 (2m 18s), 06-03 (4m 19s), 06-04 (8m)
- Trend: Consistent

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- **CFG-ENV-LOWERCASE**: Use lowercase env var mapping without split for flat config structure
- **TABLE-HEAD-CLEAR**: Must clear table_row on End(TableHead) to avoid mixing header and data cells
- **TASK-MARKER-PRIORITY**: Check current_task_checked before in_list_item in text handler
- **CLI-VALUE-SOURCE**: Use clap value_source() to distinguish explicit CLI values from defaults
- **TOKIO-UTIL-NO-SYNC**: tokio-util does not have a sync feature, CancellationToken is in base crate
- **EOF-STATE-STRUCT**: Track stdout_done/stderr_done in struct, not local to next_output()
- **PROCESS-GROUP-SIGTERM**: Send SIGTERM to negative PID to signal entire process group
- **BIASED-SELECT-CANCEL**: Use biased select! to check cancellation before output
- **PROMPT-INCLUDE-STR**: Use include_str! for compile-time prompt embedding (zero runtime cost)
- **CLAUDE-HEADLESS-P**: Use -p flag for headless Claude CLI execution
- **STACK-PRIORITY-ORDER**: Check Cargo.toml before package.json before pyproject.toml before go.mod
- **BOX-FIGMENT-ERROR**: Box figment::Error in RslphError to reduce enum size
- **VAGUENESS-THRESHOLD-055**: Use +0.55 for very short inputs to ensure score > 0.5 triggers clarification
- **DOUBLE-ENTER-STDIN**: Use two consecutive empty lines to terminate multi-line input
- **STREAM-JSON-FORMAT**: Use `--output-format stream-json` with `--verbose` for Claude CLI (required when using -p mode)
- **CHRONO-TIMESTAMP**: Use chrono for iteration timestamps in log
- **STDERR-BUILD-LOGS**: Use eprintln with [BUILD] prefix for iteration status logs
- **DRY-RUN-VALIDATE-PROMPT**: Validate prompt loading in dry-run to catch config errors early
- **WHICH-FALLBACK**: Use `which` to resolve relative command names to absolute paths at config load time
- **EMPTY-PARENT-PATH**: Filter empty parent paths when getting working_dir from progress_path (Path::parent returns Some("") for bare filenames)
- **SAPLING-SL-ROOT-DETECT**: Detect Sapling via `sl root` command success, not `.sl` directory (Sapling is a client that works with Git/Mercurial repos, doesn't create its own directory)
- **VCS-SHELL-OUT**: Shell out to git/sl CLI rather than using git2 crate (simpler, no C dependency)
- **VCS-SAPLING-FIRST**: Detect Sapling via sl root before Git via .git directory
- **VCS-WARN-NOT-FAIL**: VCS errors are logged as warnings, do not fail the build
- **VCS-ITER-COMMIT**: Commit after iteration completion, not per-task
- **CONTEXT-CAPTURED-NAME**: Store project_name in BuildContext at construction, not rely on Claude response parsing
- **SL-LOG-HASH**: Use `sl log -l 1 --template '{node|short}'` to get commit hash after sl commit (sl commit produces no stdout)
- **TUI-STDERR-BACKEND**: Use stderr for terminal backend to keep stdout available for non-TUI output
- **TUI-PANIC-HOOK-CHAIN**: Chain panic hooks instead of replacing to preserve existing panic behavior
- **TUI-UNBOUNDED-CHANNEL**: Use unbounded channels for event handling to avoid backpressure with fast Claude output
- **OUTPUT-ROLE-PREFIX**: Format messages as 'role: content' with indentation for multiline
- **SCROLL-CLAMP-VIEWPORT**: Use viewport_height and content_height for scroll bounds
- **TUI-LOG-ROUTING**: Route logs through TUI channel when active to prevent stderr corruption
- **BUILDCONTEXT-TUI-TX**: Add optional tui_tx sender to BuildContext for log routing
- **LOG-AS-SYSTEM-MESSAGE**: Display log messages as 'system' role in thread view

### Pending Todos

- **CLAUDE-INTERNET-FLAG**: Remove `--internet` workaround flag from Claude CLI invocations once the underlying issue causing Claude CLI to hang without it is resolved. See `src/planning/command.rs`.
- **CLAUDE-CLI-OUTPUT-FLAGS**: Research Claude CLI `--output-format stream-json` and `--json-schema` flags for correct usage. See `.planning/todos/pending/2026-01-18-research-claude-cli-stream-json-and-json-schema.md`.

### Blockers/Concerns

None.

## Session Continuity

Last session: 2026-01-19 04:00 UTC
Stopped at: Completed 06-04-PLAN.md (Thread View, Keybindings, & Integration)
Resume file: None
