# Phase 11: Prompt Engineering — Context

## Vision

Implement multiple prompting structures as distinct modes, each representing a different approach to autonomous agent guidance. Users select a mode that applies coherently to both plan and build commands.

## Key Decisions

### Prompt Modes Architecture

**Three modes:**
| Mode | Description |
|------|-------------|
| `basic` | Exact copy of portableralph PROMPT_plan.md and PROMPT_build.md |
| `gsd` | Adapted from GSD patterns — XML structure, must-haves verification, state tracking |
| `gsd-tdd` | GSD foundation + strict test-driven development flow |

**Mode selection:**
- Config file: `prompt_mode = "basic"` in rslph.toml
- CLI flag: `--mode=gsd` overrides config
- Both plan and build use the same mode (coherent pair)

### Basic Mode

- Exact copy of portableralph prompts (https://www.portableralph.com/)
- No modifications or "cleanup" — faithful reproduction
- Serves as baseline for comparison

### GSD Mode

**Research source:** Local GSD files in `~/.claude/get-shit-done/`

**Patterns to adopt:**

| Pattern | Implementation |
|---------|----------------|
| @ file references | Manual resolution in prompt assembly — parse `@progress.md`, inline content |
| XML task structure | `<task>`, `<objective>`, `<verify>`, `<must_haves>` tags |
| STATE.md structure | Progress.md with YAML frontmatter + structured sections |
| Must-haves verification | Goal-backward success criteria in progress file |

**Patterns ruled out:**

| Pattern | Reason |
|---------|--------|
| Multi-file phase plans | rslph uses single progress.md as only memory; can't split |
| Wave parallelization | rslph executes sequentially, one Claude per iteration |
| Human checkpoints | rslph is fully autonomous during execution |
| Separate PLAN.md per phase | Fresh context reads only progress.md — keep self-contained |

### XML Task Structure (GSD Pattern)

Prompts should use XML for clarity:

```xml
<objective>What this iteration should accomplish</objective>

<context>
@progress.md     <!-- Prior learnings -->
</context>

<task type="auto">
  <name>Task name</name>
  <action>Implementation instructions</action>
  <verify>Verification command (e.g., cargo test)</verify>
  <done>Acceptance criteria</done>
</task>

<must_haves>
  <truths>Observable behaviors that must be true</truths>
  <artifacts>Files that must exist with real implementation</artifacts>
  <key_links>Critical connections between components</key_links>
</must_haves>

<success_criteria>
- [ ] All tasks completed
- [ ] All verification checks pass
</success_criteria>
```

**Note:** Drop `<checkpoint:*>` types from GSD — no human in loop.

### @ File Reference Resolution

GSD uses `@path/to/file.md` in prompts. Claude CLI doesn't auto-resolve these.

**Implementation:**
1. Parse prompt templates for `@` references
2. Read referenced files
3. Inline content before sending to Claude
4. Support: `@progress.md`, `@~/.rslph/prompts/build.md`

**Trade-off considered:** Linking separate PLAN.md files via @ syntax was explored but ruled out. Fresh-context Claude would need complex multi-file resolution. Single self-contained progress.md is simpler and proven.

### Progress File Structure (STATE.md Pattern)

Adopt GSD's STATE.md structure for progress.md:

```yaml
---
phase: current_phase_number
status: in_progress | complete
iterations_completed: N
total_tokens: X
last_updated: ISO8601
---

# [Task Title]

## Current Position
Phase: N - Name
Last activity: [date] — [what was accomplished]

## Accumulated Learnings
[Key learnings from iterations — prevents re-trying failures]

## Decisions Made
| ID | Decision | Choice | Rationale |
|----|----------|--------|-----------|

## Must-Haves (Success Criteria)
- [ ] Observable behavior 1
- [ ] Artifact exists: path/to/file
- [ ] Key link: A connects to B

## Blockers/Concerns
[What's stuck]

## Next Action
[What to do in next iteration]
```

### GSD-TDD Mode

**TDD strictness:**
- Strict test-first — write failing tests before implementation
- Exception: Test impossible to write OR agent failed 3 iterations in a row
- If escaped: Clear loop to continue, capture failure, notify user at build end

**Test type selection:**
1. Built-in heuristics based on project type (unit vs e2e)
2. Adaptive questioning can override heuristics
3. If no `--adaptive` flag, use heuristics only

**TDD attempt tracking:** Use progress.md frontmatter:
```yaml
tdd_attempts: 2
tdd_escaped: false
tdd_escape_reason: null
```

### Executor/Verifier Separation (Future Enhancement)

GSD spawns separate verifier agents post-execution. For rslph, could implement as iteration types:

- **Build iterations:** Execute tasks, commit atomically
- **Verify iterations:** Run tests, check must_haves, report gaps
- **Gap iterations:** Address failures found by verifier

**Deferred to v1.3** — Phase 11 focuses on prompt structure, not iteration type separation.

### Adaptive Flag Enhancement

When `--adaptive` is used:
- Existing: Questions about project complexity/clarifications
- New: Questions about testing strategy (what types of tests, what frameworks)
- Testing info included in progress file for all iterations

## Research Findings (Resolved)

### Q1: Which GSD patterns are applicable to rslph's loop model?

**Applicable:**
- XML task structure (`<task>`, `<objective>`, `<verify>`, `<must_haves>`)
- STATE.md pattern (progress.md as short-term memory)
- @ file references (manual resolution)
- Must-haves verification (goal-backward success criteria)
- Atomic per-task commits (already doing)

**Not applicable:**
- Wave-based parallelization (sequential execution)
- Human checkpoints (fully autonomous)
- Multi-file STATE/PROJECT/ROADMAP (single progress.md)
- Planning workflows (tasks pre-planned)

### Q2: What heuristics determine unit test vs e2e test selection?

**Heuristics to implement:**
- Cargo.toml with `[[bin]]` → e2e tests (CLI behavior)
- Cargo.toml with `[lib]` only → unit tests
- `tests/` directory exists → e2e preferred
- `#[cfg(test)]` modules → unit tests exist
- package.json with `jest`/`vitest` → unit tests
- package.json with `playwright`/`cypress` → e2e tests

### Q3: How does GSD track state across iterations?

GSD uses STATE.md read at every workflow start:
- YAML frontmatter with phase, status, metrics
- Accumulated Context section (decisions, blockers, learnings)
- Session Continuity section

**For rslph:** Progress.md mirrors this structure. Read at iteration start, write at iteration end.

### Q4: What format should test strategy info take in progress file?

```yaml
---
test_strategy:
  type: unit | e2e | mixed
  framework: cargo-test | jest | pytest
  run_command: "cargo test --lib"
  coverage_target: 80
tdd_attempts: 0
tdd_escaped: false
---
```

## Deferred Ideas

- **Executor/Verifier separation** — v1.3, not Phase 11
- **Parallel trials** — v2.0 Multi-Trial Results phase

---
*Created: 2026-01-21*
*Updated: 2026-01-21 — Added GSD research findings*
