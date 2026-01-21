# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-01-20)

**Core value:** Autonomous task execution with fresh context per iteration and accumulated learnings
**Current focus:** v1.2 Context Engineering — eval system + test-driven flow

## Current Position

Phase: 10 - Eval Projects and Testing
Plan: 01 of 4
Status: In progress
Last activity: 2026-01-20 — Completed 10-01-PLAN.md (Built-in Eval Projects)

Progress: [##########] 100% v1.0 | [##########] 100% v1.1 | [######░░░░] 60% v1.2

## Phase Summary (v1.2)

| Phase | Goal | Requirements | Status |
|-------|------|--------------|--------|
| 8 - Token Tracking | Users can observe token consumption | TOK-01, TOK-02, TOK-03, TOK-04 | Complete ✓ |
| 9 - Eval Foundation | Controlled benchmarks in isolation | EVAL-01, EVAL-04, EVAL-05 | Complete ✓ |
| 10 - Eval Projects | Evaluate against built-in projects | PROJ-01-04, EVAL-02, EVAL-03 | In progress (1/4) |
| 11 - Prompt Engineering | TDD with clear iteration guidance | PROMPT-01 to PROMPT-05 | Pending |
| 12 - Multi-Trial Results | Multiple trials, compare results | EVAL-06 to EVAL-09 | Pending |

## Performance Metrics

**v1.0 Velocity:**
- Total plans completed: 17
- Average duration: 5m 31s
- Total execution time: 1.47 hours
- Shipped: 2026-01-19 (3 days from start)

**v1.1 Velocity:**
- Total plans completed: 6
- Average duration: 4m 53s
- Total execution time: 29m 18s
- Shipped: 2026-01-19 (same day)

**v1.2 Velocity:**
- Total plans completed: 8
- Average duration: 3m 52s
- Total execution time: 31m 1s

**By Phase (v1.0):**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01-foundation | 3/3 | 22m 5s | 7m 22s |
| 02-subprocess-management | 2/2 | 6m 29s | 3m 15s |
| 03-planning-command | 2/2 | 16m | 8m |
| 04-core-build-loop | 4/4 | 22m 41s | 5m 40s |
| 05-vcs-integration | 2/2 | 8m | 4m |
| 06-tui-interface | 4/4 | 17m 4s | 4m 16s |

**By Phase (v1.1):**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 07-e2e-testing-framework | 5/5 | 26m | 5m 12s |
| 07.1-tui-testing | 1/1 | 3m 18s | 3m 18s |

**By Phase (v1.2):**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 08-token-tracking | 4/4 | 18m | 4m 30s |
| 09-eval-command-foundation | 3/3 | 9m 46s | 3m 15s |
| 10-eval-projects-and-testing | 1/4 | 3m 15s | 3m 15s |

*Updated after each plan completion*

## Accumulated Context

### Decisions

All decisions are archived in milestone roadmap files:
- `.planning/milestones/v1.0-ROADMAP.md`
- `.planning/milestones/v1.1-ROADMAP.md`

**v1.2 Decisions (Phase 8):**

| ID | Decision | Choice |
|----|----------|--------|
| token-display-format | Status bar token format | "In: X \| Out: Y \| CacheW: Z \| CacheR: W" |
| token-formatting-lib | Number formatting library | human_format crate for SI suffixes (5.2k, 1.2M) |
| token-accumulation | Token accumulation approach | Use += to accumulate across all messages and iterations |
| token-field-tracking | Fields to track | All 4: input, output, cache_creation, cache_read |
| token-config-location | Fake Claude token config | TokenConfig in fake_claude_lib with ScenarioBuilder integration |

**v1.2 Decisions (Phase 9):**

| ID | Decision | Choice |
|----|----------|--------|
| eval-module-structure | Eval module pattern | Mirrors build module: mod.rs exports types, command.rs contains handler |
| eval-stub-approach | Initial implementation | Stub returns placeholder EvalResult for incremental development |
| eval-token-return-types | Plan/Build return types | run_plan_command returns (PathBuf, TokenUsage), run_build_command returns TokenUsage |
| eval-token-aggregation | Token aggregation | total_tokens = plan_tokens + build_tokens |
| eval-prompt-detection | Prompt file priority | prompt.txt > README.md > PROMPT.md |
| eval-test-scope | E2E test focus | CLI parsing and validation, not full execution |

**v1.2 Decisions (Phase 10):**

| ID | Decision | Choice |
|----|----------|--------|
| include-dir-paths | File path handling | include_dir stores files with project prefix (e.g., "calculator/tests.jsonl") |
| test-data-separation | Hidden test data | extract_project_files excludes tests.jsonl; get_test_data provides access |

### Pending Todos

- **CLAUDE-INTERNET-FLAG**: Remove `--internet` workaround flag from Claude CLI invocations once the underlying issue causing Claude CLI to hang without it is resolved. See `src/planning/command.rs`.
- **CLAUDE-CLI-OUTPUT-FLAGS**: Research Claude CLI `--output-format stream-json` and `--json-schema` flags for correct usage.

### Blockers/Concerns

None.

## Session Continuity

Last session: 2026-01-20
Stopped at: Completed 10-01-PLAN.md (Built-in Eval Projects)
Resume file: None
