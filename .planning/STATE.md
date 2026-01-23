# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-01-20)

**Core value:** Autonomous task execution with fresh context per iteration and accumulated learnings
**Current focus:** v1.2 Context Engineering — eval system + test-driven flow

## Current Position

Phase: 14 - TUI Visual Parity
Plan: 1 of 6 complete
Status: In progress
Last activity: 2026-01-23 - Completed 14-01 centralized theme module

Progress: [##########] 100% v1.0 | [##########] 100% v1.1 | [##########] 95% v1.2

## Phase Summary (v1.2)

| Phase | Goal | Requirements | Status |
|-------|------|--------------|--------|
| 8 - Token Tracking | Users can observe token consumption | TOK-01, TOK-02, TOK-03, TOK-04 | Complete |
| 9 - Eval Foundation | Controlled benchmarks in isolation | EVAL-01, EVAL-04, EVAL-05 | Complete |
| 10 - Eval Projects | Evaluate against built-in projects | PROJ-01-04, EVAL-02, EVAL-03 | Complete |
| 11 - Prompt Engineering | TDD with clear iteration guidance | PROMPT-01 to PROMPT-05 | Complete |
| 12 - Multi-Trial Results | Multiple trials, compare results | EVAL-06 to EVAL-09 | Complete |
| 13 - Parallel Eval TUI | Parallel evals with live TUI | PARA-01 to PARA-04 | Complete |
| 14 - TUI Visual Parity | Claude Code-style TUI design | TUI-01 to TUI-06 | In Progress (1/6) |

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
- Total plans completed: 26
- Average duration: 3m 19s
- Total execution time: 92m 51s

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
| 10-eval-projects-and-testing | 4/4 | 11m 5s | 2m 46s |
| 11-prompt-engineering | 4/4 | 14m 16s | 3m 34s |
| 12-multi-trial-results | 5/5 | 13m | 2m 36s |
| 13-parallel-eval-tui | 9/9 | 27m | 3m |
| 14-tui-visual-parity | 1/6 | 11m | 11m |

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
| test-runner-sync | Test execution pattern | Use std::process::Command (sync) not tokio since tests run post-build |
| output-comparison | Whitespace handling | Trim both expected and actual output for comparison |
| jsonl-error-handling | Parse error strategy | Skip malformed lines via filter_map, don't fail on parse errors |
| debug-binary-preference | Binary selection order | Debug binary preferred over release in find_built_program |
| list-flag-optional-project | CLI argument handling | --list flag makes project argument optional via required_unless_present |
| test-phase-timing | Test execution timing | Test execution happens after build, before workspace cleanup |
| e2e-test-module-structure | E2E test organization | Add tests to existing eval_command.rs rather than standalone file |
| fizzbuzz-test-coverage | Test case range | 8 cases covering 1-20 range with progressive complexity |

**v1.2 Decisions (Phase 11):**

| ID | Decision | Choice |
|----|----------|--------|
| prompt-mode-variants | PromptMode enum variants | Basic/Gsd/GsdTdd as the three prompt modes |
| prompt-mode-default | Default prompt mode | Basic for backward compatibility |
| prompt-mode-serialization | String serialization format | snake_case for both strum and serde (basic, gsd, gsd_tdd) |
| tdd-state-structure | TDD state tracking format | tdd_state block in YAML frontmatter with phase, consecutive_failures, escaped fields |
| tdd-escape-threshold | TDD escape hatch threshold | 3 consecutive failures triggers escape hatch (PROMPT-03) |
| tdd-task-types | TDD task type variants | Three task types: test, implement, refactor for TDD phases |
| basic-mode-content | Basic mode prompt content | Use current rslph prompts (not PortableRalph) for backward compatibility |
| mode-file-precedence | File override precedence | File overrides > mode selection for power users |

**v1.2 Decisions (Phase 12):**

| ID | Decision | Choice |
|----|----------|--------|
| variance-correction | Sample variance formula | Bessel's correction (n-1) for unbiased estimator |
| empty-stats-handling | Empty slice handling | Return zeros for all fields (count=0) |
| single-value-variance | Single value variance | Return 0.0 (no variation with one sample) |
| trial-result-return | run_eval_command return value | Return last trial's EvalResult for backward compatibility |
| pass-rate-normalization | Pass rate internal format | 0.0-1.0 internally, displayed as percentage |
| json-filename-pattern | Multi-trial JSON filename | eval-results-{project}-{YYYY-MM-DD}.json |
| json-deserialize | Deserialize derive | Added for future compare command loading |

**v1.2 Decisions (Phase 13):**

| ID | Decision | Choice |
|----|----------|--------|
| parallel-limit | Concurrent trial limit | Semaphore::new(3) for rate limiting |
| event-channel-type | TrialEvent channel | mpsc::unbounded_channel for async event communication |
| hash-derive | PromptMode Hash | Added Hash to PromptMode for HashMap key usage |
| multimode-filename | Multi-mode JSON filename | eval-results-{project}-multimode-{timestamp}.json |

**v1.2 Decisions (Phase 14):**

| ID | Decision | Choice |
|----|----------|--------|
| brand-color-encoding | Claude brand color format | RGB values from brand guidelines (CRAIL=#C15F3C, CLOUDY=#B1ADA1, PAMPAS=#F4F3EE) |
| model-tier-detection | Model tier detection logic | Case-insensitive substring matching for "opus", "sonnet" |
| style-composition | Style function pattern | Each style function returns complete Style object with colors and modifiers |

**v1.2 Decisions (Phase 13 - Continued):**

| ID | Decision | Choice |
|----|----------|--------|
| conversation-max-items | Ring buffer limit | 1000 items for memory efficiency |
| conversation-toggle-key | Conversation toggle key | 'c' key toggles split conversation view |
| conversation-scroll-keys | Conversation scroll keys | PageUp/PageDown scroll by 10 items |
| split-view-ratio | Split view layout | 50/50 horizontal split for conversation and main view |
| plan-tui-pattern | Plan TUI architecture | Separate TUI task with mpsc channel for event forwarding |
| plan-tui-auto-scroll | Plan TUI scrolling | Auto-scroll to bottom as new content arrives |

### Pending Todos

- **CLAUDE-INTERNET-FLAG**: Remove `--internet` workaround flag from Claude CLI invocations once the underlying issue causing Claude CLI to hang without it is resolved. See `src/planning/command.rs`.
- **CLAUDE-CLI-OUTPUT-FLAGS**: Research Claude CLI `--output-format stream-json` and `--json-schema` flags for correct usage.

### Future Features (v1.3 Candidates)

*Note: EVAL-PARALLEL-MODES, EVAL-TUI-DASHBOARD, TUI-LLM-OUTPUT, and PLAN-TUI are now addressed in Phase 13.*

None currently.

### Blockers/Concerns

None.

## Session Continuity

Last session: 2026-01-23
Stopped at: Completed 14-01 centralized theme module
Resume file: None

### Roadmap Evolution

- Phase 13 added: Parallel Eval TUI (parallel modes, eval dashboard, enhanced TUI, plan TUI)
- Phase 13 planned: 4 plans in 2 waves (wave 1: 13-01, wave 2: 13-02, 13-03, 13-04)
- 13-01 complete: Parallel infrastructure with --modes flag and JoinSet execution
- 13-02 complete: Dashboard TUI for parallel eval with multi-pane grid layout
- 13-03 complete: Enhanced conversation view in build TUI with 'c' toggle
- 13-04 complete: Plan command TUI mode with streaming LLM output
- Phase 13 UAT: 3 gaps found, closed with plans 13-05, 13-06, 13-07
- Milestone audit: 2 critical gaps found (PARA-01 partial, PARA-02 partial)
- Gap closure plans created: 13-08 (dashboard iteration progress), 13-09 (mode passthrough)
- 13-09 complete: Mode passthrough from eval trials to plan/build commands
- 13-08 complete: Iteration progress wired to dashboard TUI via ProgressCallback
- Phase 13 complete: All 9 plans done, all PARA requirements fulfilled
- Phase 14 added: TUI Visual Parity with Claude Code (brand colors, box-drawn elements, spinner, status bar)
- 14-01 complete: Centralized theme module with Claude brand colors, model tier symbols, and style functions
