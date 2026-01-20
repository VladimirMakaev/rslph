# Requirements: v1.2 Context Engineering

**Project:** rslph
**Milestone:** v1.2 "Context Engineering"
**Created:** 2026-01-20
**Status:** Approved

---

## v1.2 Requirements

### Token Tracking (TOK)

| ID | Requirement | Priority |
|----|-------------|----------|
| TOK-01 | Track input/output tokens per iteration from stream-json | Must |
| TOK-02 | Track cache tokens (creation and read) per iteration | Must |
| TOK-03 | Sum total tokens consumed across all iterations | Must |
| TOK-04 | Store token metrics in build state for persistence | Must |

### Eval Command (EVAL)

| ID | Requirement | Priority |
|----|-------------|----------|
| EVAL-01 | `rslph eval <project>` command runs plan+build in isolated temp directory | Must |
| EVAL-02 | Execute hidden test runner after build completes (black-box input/output testing) | Must |
| EVAL-03 | Track pass rate (passing/total test cases) | Must |
| EVAL-04 | Track total execution time | Must |
| EVAL-05 | Track total token consumption across plan+build | Must |
| EVAL-06 | Support multiple trial runs with configurable count | Must |
| EVAL-07 | Report mean/variance across trials | Must |
| EVAL-08 | Store results in JSON file | Must |
| EVAL-09 | Compare results between different runs | Should |

### Eval Projects (PROJ)

| ID | Requirement | Priority |
|----|-------------|----------|
| PROJ-01 | Calculator eval project with starting prompt | Must |
| PROJ-02 | Test runner script (language-agnostic, checks stdin/stdout pairs) | Must |
| PROJ-03 | Test data file with input/expected output pairs (hidden from agent) | Must |
| PROJ-04 | Second eval project of medium difficulty (TBD scope) | Should |

### Prompt Engineering (PROMPT)

| ID | Requirement | Priority |
|----|-------------|----------|
| PROMPT-01 | Add deviation handling rules to build prompt | Must |
| PROMPT-02 | Add substantive completion summary format | Must |
| PROMPT-03 | TDD iteration flow (write tests → implement → refactor) | Must |
| PROMPT-04 | Configurable TDD mode (enable/disable via config flag) | Must |
| PROMPT-05 | Research and adopt GSD patterns (phases, research structure) | Should |

---

## Future Requirements (deferred)

| ID | Requirement | Rationale |
|----|-------------|-----------|
| FUT-01 | Cost estimation from token counts | Nice-to-have, can calculate externally |
| FUT-02 | TUI token display during execution | Polish, not core functionality |
| FUT-03 | Docker-based isolation (like SWE-bench) | Overkill for v1.2, temp dir sufficient |
| FUT-04 | Two-attempt mode (like Aider) | Enhancement after core eval works |
| FUT-05 | Verification agent (separate from build loop) | Defer to v1.3+ |
| FUT-06 | Notification system (completion, failure) | Defer to v1.3+ |
| FUT-07 | User-overridable prompts via config paths | Defer to v1.3+ |

---

## Out of Scope

| Item | Rationale |
|------|-----------|
| Database storage for results | JSON files sufficient for v1.x |
| Web dashboard for results | CLI-focused tool |
| IDE integration | Out of project scope |
| Multi-model support | Claude-only via Claude CLI |

---

## Requirement Traceability

*Filled by roadmapper during phase creation*

| Requirement | Phase |
|-------------|-------|
| TOK-01 | — |
| TOK-02 | — |
| TOK-03 | — |
| TOK-04 | — |
| EVAL-01 | — |
| EVAL-02 | — |
| EVAL-03 | — |
| EVAL-04 | — |
| EVAL-05 | — |
| EVAL-06 | — |
| EVAL-07 | — |
| EVAL-08 | — |
| EVAL-09 | — |
| PROJ-01 | — |
| PROJ-02 | — |
| PROJ-03 | — |
| PROJ-04 | — |
| PROMPT-01 | — |
| PROMPT-02 | — |
| PROMPT-03 | — |
| PROMPT-04 | — |
| PROMPT-05 | — |

---

## Research Sources

- [HumanEval Benchmark](https://klu.ai/glossary/humaneval-benchmark) — pass@k metric, unit test evaluation
- [Aider Polyglot](https://epoch.ai/benchmarks/aider-polyglot) — two-attempt mode, cost tracking
- [SWE-bench Docker Setup](https://www.swebench.com/SWE-bench/guides/docker_setup/) — container isolation patterns
- [2025 Coding LLM Benchmarks Guide](https://www.marktechpost.com/2025/07/31/the-ultimate-2025-guide-to-coding-llm-benchmarks-and-performance-metrics/)
- GSD skill files at `~/.claude/get-shit-done/` — prompt engineering patterns

---

*18 requirements | 4 categories | Ready for roadmap*
