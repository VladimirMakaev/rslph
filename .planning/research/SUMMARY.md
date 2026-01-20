# Research Summary: v1.2 Context Engineering

**Project:** rslph v1.2 "Context Engineering"
**Domain:** Eval system, token tracking, test-driven agent loops
**Researched:** 2026-01-20
**Confidence:** HIGH

---

## Executive Summary

The v1.2 Context Engineering milestone is architecturally straightforward. The existing codebase already contains 90% of the infrastructure needed: the `Usage` struct in `stream_json.rs` captures all token fields, `std::time::Instant` is already used for timing, and the `tempfile` crate is present as a dev-dependency. The primary work is wiring existing capabilities together, not building new infrastructure.

The recommended approach is to layer token tracking into the existing iteration loop, then build the eval command as an orchestrator that reuses the existing plan and build commands. GSD patterns (structured XML tasks, deviation rules, goal-backward verification) should be adopted incrementally to improve agent steering. The eval system should use embedded projects via `include_str!` for portability.

Key risks center on eval non-determinism and test data contamination. Run multiple trials with statistical acceptance criteria, store hidden tests outside the project directory, and verify test file integrity after each run. The prompt engineering pitfall of "lost in middle" instruction decay can be mitigated by placing critical constraints at both the start and end of prompts.

---

## Key Findings

### Stack Additions

**Minimal changes required:**

| Change | Rationale |
|--------|-----------|
| Promote `tempfile = "3"` from dev-deps to deps | Eval command needs temp directories at runtime |

Everything else exists: `serde_json` for JSON output, `std::time::Instant/Duration` for timing, `chrono` for timestamps. Token tracking infrastructure is already implemented in `StreamResponse` and `Usage` structs.

**What NOT to add:**
- No benchmarking crates (criterion, hyperfine) - simple `Instant::elapsed()` suffices
- No database (sqlite, sled) - JSON files are adequate for v1.x
- No metrics/tracing crates - capture at run end, write to JSON

### Feature Table Stakes

**Must-have (table stakes):**
- Token consumption tracking per iteration
- Controlled project execution in isolated workspaces
- Hidden test suite separate from visible tests
- Pass/fail and partial credit tracking
- Run comparison (JSON storage)
- Multiple built-in eval projects (2-3 minimum)

**Should-have (differentiators):**
- Prompt A/B testing support
- Iteration-level metrics (not just final result)
- Failure mode categorization
- Cost efficiency ratio (success per token)

**Defer to post-v1.2:**
- TDD iteration flow (significant iteration structure change)
- Checkpoint protocol (adds complexity)
- Full GSD-style verification agent

### Architecture Approach

The eval command orchestrates existing commands rather than duplicating logic. `EvalRunner` creates a temp workspace, extracts embedded project files, runs plan+build, then executes hidden tests. Token tracking hooks into `StreamEvent` parsing in the iteration loop via a new `IterationTokens` accumulator in `BuildContext`.

**New modules:**
- `src/tokens.rs` - `TokenUsage`, `IterationTokens`, `EvalTokenStats`
- `src/eval/` - `command.rs`, `runner.rs`, `result.rs`, `project.rs`, `builtin.rs`, `prompts.rs`
- `evals/` - Embedded eval projects (fizzbuzz, etc.)

**Modifications:**
- `build/iteration.rs` - Token accumulation
- `build/state.rs` - Add `iteration_tokens: Vec<IterationTokens>`
- `cli.rs` - Add `Commands::Eval`

### GSD Patterns to Adopt

From GSD skill file analysis, adopt these patterns:

| Pattern | Priority | Quick Win? |
|---------|----------|------------|
| Deviation rules in build prompt | High | Yes - low complexity, immediate improvement |
| Substantive completion summaries | Medium | Yes - just prompt text |
| Structured XML task format | High | No - requires progress file changes |
| Goal-backward verification | High | No - requires verification agent |
| TDD iteration structure | High | No - significant iteration changes |

**Quick win prompt additions:**
```markdown
## Deviation Handling
1. BUGS: Fix immediately, note in Recent Attempts
2. MISSING DEPS: Install and continue
3. BLOCKING ISSUES: Work around and document
4. MAJOR CHANGES: Document decision, proceed with best judgment
```

### Watch Out For

**Critical pitfalls:**

1. **Non-deterministic eval results** - Run 3-5 trials per benchmark, report mean and variance, use temperature=0
2. **Data contamination / benchmark leakage** - Store hidden tests OUTSIDE project directory, never in Claude's working path
3. **Reward hacking / test modification** - Hash test files before/after, fail if modified
4. **Usage data only on final events** - Capture usage from multiple event types, handle partial data on timeout
5. **Fake Claude binary event mismatch** - Periodically verify fake output matches real Claude CLI format

**Phase-mapped prevention:**

| Phase | Must Address |
|-------|--------------|
| Eval framework design | Non-determinism, data contamination, test integrity |
| Token tracking | Usage capture robustness, cache token accounting |
| Eval project bundling | Version mismatch, path assumptions, state leakage |
| Prompt engineering | Instruction decay, TDD ambiguity, format drift |

---

## Recommended Phase Order

Based on dependency analysis and research findings:

### Phase 1: Token Tracking Foundation
**Rationale:** Prerequisite for eval metrics - must exist before eval command can record meaningful data
**Delivers:** Per-iteration token accumulation, display in iteration log
**Implements:**
- `src/tokens.rs` with `TokenUsage`, `IterationTokens`
- Modification to `build/iteration.rs` for token accumulation
- Extension to `BuildContext` with `iteration_tokens` field
**Avoids:** Cache token misattribution (pitfall 3.3), cumulative vs per-iteration confusion (pitfall 3.2)

### Phase 2: Eval Infrastructure
**Rationale:** Core scaffolding before any eval projects - provides the framework
**Delivers:** `rslph eval` command shell, `EvalRunner` orchestration, `EvalResult` structure
**Implements:**
- `src/eval/mod.rs` module structure
- `src/eval/command.rs` entry point
- `src/eval/runner.rs` workspace and orchestration
- `src/eval/result.rs` with `TestResults`
- CLI integration (`Commands::Eval`)
**Avoids:** Workspace state leakage (pitfall 4.2) via fresh TempDir per run

### Phase 3: Built-in Eval Projects
**Rationale:** Need at least 2-3 eval projects to validate the framework
**Delivers:** Embedded eval projects (fizzbuzz, cli-todo), project extraction, hidden test execution
**Implements:**
- `evals/` directory with project files
- `src/eval/builtin.rs` with `include_str!` embedding
- `src/eval/project.rs` for extraction
- Promote `tempfile` to regular dependency
**Avoids:** Embedded test data version mismatch (pitfall 4.1), path assumptions (pitfall 4.3)

### Phase 4: Eval Prompt Engineering
**Rationale:** Improve agent steering for eval runs after infrastructure is stable
**Delivers:** Test-driven prompts, deviation rules, substantive summary enforcement
**Implements:**
- `src/eval/prompts.rs` with `get_eval_build_prompt()`
- Deviation handling instructions
- Test-driven iteration guidance
**Avoids:** Instruction decay (pitfall 5.1), TDD ambiguity (pitfall 5.2)

### Phase 5: Results and Comparison
**Rationale:** Polish phase - make eval results useful
**Delivers:** JSON output, run comparison, partial credit metrics
**Implements:**
- JSON serialization of `EvalResult`
- Historical run storage
- Comparison views
**Avoids:** Success metric conflation (pitfall 1.4)

### Phase Ordering Rationale

- **Token tracking first:** All subsequent phases depend on having token metrics
- **Infrastructure before projects:** Framework must exist to test eval projects
- **Projects before prompts:** Need working evals to validate prompt improvements
- **Results last:** Polish after core functionality works

---

## Research Flags

**Phases likely needing deeper research during planning:**
- **Phase 4 (Prompt Engineering):** GSD patterns need adaptation for rslph's specific structure; test TDD prompt variations

**Phases with well-documented patterns:**
- **Phase 1 (Token Tracking):** Straightforward - hook into existing `StreamEvent` parsing
- **Phase 2 (Eval Infrastructure):** Follows existing command patterns exactly
- **Phase 3 (Built-in Projects):** Standard `include_str!` embedding

---

## Open Questions

1. **Hidden test storage location:** Store outside project dir, but where exactly? `~/.rslph/eval-data/`? Embedded in binary separately?
2. **Temperature=0 support:** Does Claude CLI support forcing deterministic output? Verify flag availability.
3. **Multiple trial execution:** Should `rslph eval` run multiple trials automatically, or require wrapper script?
4. **Eval project versioning:** How to track which version of eval projects produced which results?

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Verified existing codebase has 90% of needed infrastructure |
| Features | HIGH | Based on GSD analysis and SWE-bench patterns |
| Architecture | HIGH | Direct codebase analysis, clear integration points |
| Pitfalls | HIGH | Validated by research papers and existing test infrastructure |

**Overall confidence:** HIGH

The v1.2 milestone is well-scoped with minimal external dependencies. The primary work is integration and prompt engineering, not new infrastructure.

### Gaps to Address

- **Cache token pricing:** Exact rates for cache_creation vs cache_read need verification from Anthropic docs
- **Real Claude output verification:** Fake Claude binary should be periodically validated against real CLI output
- **Statistical acceptance criteria:** Define specific thresholds (e.g., "pass if mean > 80% with variance < 10%")

---

## Sources

### Primary (HIGH confidence)
- Existing codebase: `src/subprocess/stream_json.rs`, `src/build/`, `tests/e2e/`
- GSD skill files: `~/.claude/get-shit-done/` (TDD, verification, execution patterns)
- Rust stdlib: `std::time::Instant`, `std::time::Duration`

### Secondary (MEDIUM confidence)
- [HumanEval Benchmark structure](https://klu.ai/glossary/humaneval-benchmark)
- [SWE-bench evaluation patterns](https://epoch.ai/blog/what-skills-does-swe-bench-verified-evaluate)
- [tempfile crate docs](https://docs.rs/tempfile/latest/tempfile/)

### Tertiary (LOW confidence - needs validation)
- [Defeating Non-Determinism in LLMs](https://www.flowhunt.io/blog/defeating-non-determinism-in-llms/)
- [Data Contamination in Benchmarks](https://thegrigorian.medium.com/when-benchmarks-lie-why-contamination-breaks-llm-evaluation-1fa335706f32)

---

*Research completed: 2026-01-20*
*Ready for roadmap: yes*
