# Feature Landscape: Eval System and Context Engineering

**Domain:** AI agent evaluation systems, test-driven agent loops, prompt engineering patterns
**Researched:** 2026-01-20
**Confidence:** HIGH (based on GSD skill files, SWE-bench patterns, and HumanEval structure)

## Executive Summary

This research investigates three interconnected areas for rslph's v1.2 Context Engineering milestone:

1. **Eval/Benchmark Features** - What does a proper eval command need to be useful? Based on SWE-bench, HumanEval, and modern AI agent evaluation patterns.

2. **Test-Driven Agent Loops** - How should rslph structure iterations around testing? Based on GSD's TDD patterns and verification workflows.

3. **GSD Prompt Engineering Patterns** - What techniques from GSD would improve rslph's autonomous execution? Deep analysis of GSD's skill files reveals highly effective patterns.

---

## Part 1: Eval System Features

### Table Stakes

Features users expect from an eval system. Missing = eval command feels incomplete.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Controlled project execution** | Core purpose - run plan+build on known projects | Medium | Isolated workspace per run |
| **Hidden test suite** | Prevents agent from "cheating" by reading tests | Low | Tests separate from project |
| **Pass/fail tracking** | Binary success metric per test | Low | Parse test output |
| **Time tracking** | Know how long runs take | Low | Already have iteration timing |
| **Token consumption tracking** | Critical cost metric for agent efficiency | Medium | Parse stream-json from Claude CLI |
| **Run comparison** | See if changes improve/regress performance | Medium | Store run history in JSON/SQLite |
| **Multiple eval projects** | Single project insufficient for validation | Medium | Built-in project library |
| **Reproducible runs** | Same project = same starting state | Low | Reset workspace before each run |

### Differentiators

Features that set rslph eval apart from basic benchmarking.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Prompt A/B testing** | Compare different prompt strategies on same projects | Medium | Key for context engineering |
| **Iteration-level metrics** | Track progress per iteration, not just final result | Low | Already have iteration log |
| **Test-before vs test-after comparison** | Measure TDD vs traditional flow | Medium | Different iteration strategies |
| **Partial credit scoring** | Some tests pass = partial success | Low | Count passing tests, not just all-or-nothing |
| **Failure analysis** | Categorize WHY runs fail (timeout, wrong output, compile error) | Medium | Parse failure modes |
| **Cost efficiency ratio** | Success per token spent | Low | Derived metric |
| **Prompt diff tracking** | Know exactly which prompt changes affected results | Low | Store prompt versions |

### Anti-Features

Features to explicitly NOT build for eval system.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **Leaderboard/ranking system** | This is internal tooling, not a public benchmark | Simple run history with comparison |
| **Complex statistical analysis** | Overkill for small sample sizes | Simple pass rate and averages |
| **Cloud storage for runs** | Out of scope (local-only operation) | Local JSON/SQLite storage |
| **Real-time monitoring dashboard** | TUI already provides this during execution | Post-run summary |
| **Integration with external benchmarks** | Different structure, different goals | Built-in eval projects |
| **Multi-model comparison** | Claude-only constraint | Focus on prompt/iteration optimization |

---

## Part 2: Test-Driven Agent Loops

### TDD Iteration Structure (from GSD)

GSD's TDD pattern is directly applicable to rslph's iteration loop. Key insight: **TDD produces 2-3 commits per feature (RED, GREEN, REFACTOR)**.

#### GSD TDD Flow
```
RED - Write failing test:
1. Write test describing expected behavior
2. Run test - MUST fail
3. Commit: test(phase-plan): add failing test for [feature]

GREEN - Implement to pass:
1. Write minimal code to make test pass
2. Run test - MUST pass
3. Commit: feat(phase-plan): implement [feature]

REFACTOR (if needed):
1. Clean up implementation
2. Run tests - MUST still pass
3. Commit: refactor(phase-plan): clean up [feature]
```

#### Adaptation for rslph

| GSD Pattern | rslph Adaptation | Notes |
|-------------|------------------|-------|
| Separate RED/GREEN/REFACTOR phases | Iteration-level separation | Each iteration is one phase |
| "Can you write expect(fn(input)).toBe(output) before fn?" | Task-level heuristic | Planner detects TDD candidates |
| Feature-per-TDD-plan | Feature-per-iteration-group | 2-3 iterations per feature |

#### Proposed Test-Driven Iteration Flow

**Mode: Standard (current)**
```
Iteration 1: Task A implementation
Iteration 2: Task B implementation
Iteration 3: Task C implementation
...
```

**Mode: TDD (new)**
```
Iteration 1: Write failing tests for Task A (RED)
Iteration 2: Implement Task A to pass tests (GREEN)
Iteration 3: Refactor Task A if needed (REFACTOR)
Iteration 4: Write failing tests for Task B (RED)
...
```

**Detection:** Planner marks tasks as `tdd="true"` when they have clear input/output behavior.

### Table Stakes for Test-Driven Flow

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **TDD task detection** | Planner identifies TDD candidates | Low | Heuristic in planning prompt |
| **RED iteration marker** | Agent knows it's writing tests only | Low | Instruction in progress file |
| **Test-first enforcement** | RED iteration must produce failing tests | Medium | Verify test failure before GREEN |
| **GREEN iteration marker** | Agent knows it's implementing only | Low | Instruction in progress file |

### Differentiators for Test-Driven Flow

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Automatic test framework detection** | Configure jest/pytest/cargo test based on stack | Low | Already detect stack |
| **Test failure verification** | Confirm tests actually fail in RED phase | Medium | Parse test output |
| **Test coverage tracking** | Measure coverage improvement per iteration | High | Requires coverage tools |

---

## Part 3: GSD Prompt Engineering Patterns

### Analysis of GSD Skill Files

GSD uses sophisticated prompt engineering techniques. Here are the key patterns discovered from analyzing `~/.claude/get-shit-done/`:

#### Pattern 1: Structured XML for Claude Parsing

GSD uses XML structure extensively for machine-readable sections.

**GSD Example:**
```xml
<task type="auto">
  <name>Task 1: Create User model</name>
  <files>src/features/user/model.ts</files>
  <action>Define User type with id, email, name, createdAt.</action>
  <verify>tsc --noEmit passes</verify>
  <done>User type exported and usable</done>
</task>
```

**Why It Works:**
- Clear structure Claude can parse reliably
- Each element has specific purpose
- Verification is explicit
- Success criteria is measurable

**rslph Application:**
Progress file could use structured task format instead of plain markdown checkboxes.

#### Pattern 2: Verification-First Design

GSD's core principle: **Task completion does NOT equal goal achievement**.

**GSD Approach:**
```yaml
must_haves:
  truths:
    - "User can see existing messages"
    - "User can send a message"
  artifacts:
    - path: "src/components/Chat.tsx"
      provides: "Message list rendering"
  key_links:
    - from: "Chat.tsx"
      to: "api/chat"
      via: "fetch in useEffect"
```

**Why It Works:**
- Defines observable behaviors (truths)
- Lists required artifacts
- Specifies critical connections (key_links)
- Enables automated verification

**rslph Application:**
Progress file could include `must_haves` section that verification agent checks after all tasks.

#### Pattern 3: Goal-Backward Derivation

GSD derives verification criteria by working backwards from the goal.

**Process:**
1. State the goal
2. Ask "What must be TRUE for this goal to be achieved?"
3. For each truth, ask "What must EXIST?"
4. For each artifact, ask "What must be CONNECTED?"

**rslph Application:**
Planner could generate verification criteria alongside tasks:
```markdown
## Goal: Add user authentication

## Truths (what must be true)
1. User can log in with email/password
2. Protected routes reject unauthenticated users
3. Session persists across page refreshes

## Tasks
- [ ] Create login endpoint
- [ ] Add auth middleware
- [ ] Create login UI
```

#### Pattern 4: Deviation Rules (Automatic Handling)

GSD embeds deviation rules in prompts so Claude handles unexpected situations autonomously.

**GSD Rules:**
1. **Auto-fix bugs** - Fix immediately, track for summary
2. **Auto-add missing critical functionality** - Add without asking
3. **Auto-fix blocking issues** - Unblock and continue
4. **Ask about architectural changes** - STOP and ask user

**Why It Works:**
- Reduces friction during execution
- Defines clear boundaries for autonomy
- Preserves user control over major decisions

**rslph Application:**
Build prompt could include deviation rules:
```markdown
## Deviation Handling

If you encounter unexpected issues:
1. BUGS: Fix immediately, note in Recent Attempts
2. MISSING DEPS: Install and continue
3. BLOCKING ISSUES: Work around and document
4. MAJOR CHANGES: Document decision in Recent Attempts, proceed with best judgment
```

#### Pattern 5: Context Segmentation

GSD divides work into "waves" and "segments" for parallel execution and context management.

**GSD Approach:**
- Wave 1: Independent tasks (run in parallel)
- Wave 2: Tasks depending on Wave 1
- Segments: Autonomous work between checkpoints

**Why It Works:**
- Maximizes parallelism
- Fresh context for each segment
- Clear dependency management

**rslph Application:**
Progress file could include phase dependencies:
```markdown
### Phase 1: Foundation (wave 1)
- [ ] Create database schema
- [ ] Set up API routes

### Phase 2: Features (wave 2, depends on Phase 1)
- [ ] Implement user registration
- [ ] Add authentication
```

#### Pattern 6: Substantive Summaries

GSD requires summaries to be substantive, not just "complete".

**Bad:** "Phase complete"
**Good:** "JWT auth with refresh rotation using jose library"

**Why It Works:**
- Forces real documentation
- Enables context for future iterations
- Supports knowledge transfer between runs

**rslph Application:**
After completion, require substantive status:
```markdown
## Status
RALPH_DONE
Implemented password hashing with bcrypt, login/logout endpoints with JWT,
and protected route middleware checking token validity.
```

#### Pattern 7: Checkpoint Protocol

GSD uses explicit checkpoints for human interaction.

**Types:**
- `checkpoint:human-verify` - Confirm work is correct
- `checkpoint:decision` - Make implementation choice
- `checkpoint:human-action` - Rare, truly unavoidable manual steps

**GSD Display Format:**
```
+-------------------------------------------------------+
|  CHECKPOINT: Verification Required                     |
+-------------------------------------------------------+

Progress: 5/8 tasks complete
Task: Responsive dashboard layout

Built: Responsive dashboard at /dashboard

How to verify:
  1. Run: npm run dev
  2. Visit: http://localhost:3000/dashboard
  3. Desktop: Verify sidebar visible

-------------------------------------------------------
> YOUR ACTION: Type "approved" or describe issues
-------------------------------------------------------
```

**rslph Application:**
Could add checkpoint tasks that pause iteration and wait for user input.

### Summary: GSD Patterns for rslph Adoption

| Pattern | Priority | Complexity | Impact |
|---------|----------|------------|--------|
| Structured XML task format | High | Medium | Major improvement to task parsing |
| Goal-backward verification | High | Medium | Enables verification agent |
| Deviation rules in prompt | High | Low | Reduces iteration friction |
| Substantive summaries | Medium | Low | Better completion documentation |
| TDD iteration structure | High | Medium | Enables test-driven flow |
| Checkpoint protocol | Low | High | Useful for complex tasks |
| Context segmentation | Low | Medium | Future enhancement |

---

## Part 4: Eval System Architecture

Based on SWE-bench and HumanEval patterns.

### SWE-bench Structure (HIGH confidence - official docs)

Each instance provides:
- Repository state before the fix
- Issue description
- Two test sets:
  - **Fail-to-Pass**: Tests that should pass after fix
  - **Pass-to-Pass**: Tests that should remain passing

**Key insight:** SWE-bench uses hidden tests to verify correctness without leaking answers.

### HumanEval Structure (HIGH confidence - official docs)

- 164 hand-crafted problems
- Function signature + docstring provided
- Multiple unit tests per problem (avg 7.7)
- **pass@k** metric: Success if at least one of k samples passes

### Recommended rslph Eval Structure

```
eval-projects/
  calculator/
    README.md           # Problem description
    src/                # Starter code
    tests/              # Hidden test suite (not in src/)
    solution/           # Reference solution (for validation)
  todo-app/
    README.md
    src/
    tests/
    solution/
```

**Eval run process:**
1. Copy project to temp workspace
2. Run `rslph plan` on problem description
3. Run `rslph build` with iteration limit
4. Run hidden test suite
5. Record metrics (pass rate, time, tokens)
6. Clean up workspace

### Metrics to Track

| Metric | Type | Purpose |
|--------|------|---------|
| pass_rate | float | Primary success metric |
| tests_passed | int | Partial credit |
| tests_total | int | Context for pass_rate |
| iterations | int | Efficiency measure |
| time_seconds | float | Total execution time |
| tokens_input | int | Token consumption |
| tokens_output | int | Token consumption |
| failure_mode | enum | Why it failed (timeout, error, wrong output) |

---

## Feature Dependencies

```
Token Tracking (prerequisite for eval)
        |
        v
Eval Command (runs projects with metrics)
        |
        +---> Prompt A/B Testing (compare prompts)
        |
        v
TDD Iteration Flow (improves success rate)
        |
        v
GSD Patterns in Prompts (improves steering)
```

---

## MVP Recommendation

For v1.2, prioritize:

1. **Token tracking** - Required for eval metrics, low complexity
2. **Eval command with 2-3 built-in projects** - Core evaluation capability
3. **Run comparison** - See if changes help
4. **Deviation rules in build prompt** - Quick win for steering
5. **Substantive completion summaries** - Quick win for documentation

Defer to post-v1.2:
- TDD iteration flow (significant change to iteration structure)
- Checkpoint protocol (adds complexity)
- Full GSD-style verification agent (separate milestone)

---

## Sources

### Primary Sources (HIGH confidence)
- GSD Skill Files: `~/.claude/get-shit-done/` (full analysis above)
  - `references/tdd.md` - TDD patterns
  - `references/verification-patterns.md` - Verification techniques
  - `workflows/execute-plan.md` - Execution flow
  - `workflows/verify-phase.md` - Goal-backward verification
  - `templates/phase-prompt.md` - Task structure
  - `references/checkpoints.md` - Checkpoint protocol

### Secondary Sources (MEDIUM confidence)
- [HumanEval Benchmark](https://klu.ai/glossary/humaneval-benchmark) - pass@k metrics, test structure
- [SWE-bench Skills Evaluation](https://epoch.ai/blog/what-skills-does-swe-bench-verified-evaluate) - Instance structure, test harness
- [Lakera Prompt Engineering Guide](https://www.lakera.ai/blog/prompt-engineering-guide) - Prompt engineering techniques

### Tertiary Sources (LOW confidence - not verified)
- [Benchmarking AI Agents 2025](https://metadesignsolutions.com/benchmarking-ai-agents-in-2025-top-tools-metrics-performance-testing-strategies/)
- [TDD with AI](https://medium.com/@rupeshit/tdd-in-the-age-of-vibe-coding-pairing-red-green-refactor-with-ai-65af8ed32ae8)
