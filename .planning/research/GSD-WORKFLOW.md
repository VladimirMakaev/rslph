# GSD Workflow Research

**Researched:** 2026-02-01
**Source:** `~/.claude/get-shit-done/` directory
**Confidence:** HIGH (direct source code analysis)

## Executive Summary

GSD implements a multi-stage workflow with distinct personas (agents) that orchestrate project execution from vision to shipping. The workflow is designed around fresh context per operation, with explicit state files that survive context window resets.

**Key insight for Ralph Loop integration:** GSD's architecture assumes subagent spawning (Task tool) which Ralph Loop doesn't have. However, the stage transitions, state management patterns, and checkpoint protocols translate directly to Ralph Loop's fresh-context-per-iteration model.

---

## GSD Personas (Agents)

GSD defines specialized agents with specific roles. Each agent type is optimized for its task and can run at different quality levels (opus/sonnet/haiku).

### 1. Orchestrator (Main Context)

**Role:** Coordination layer. Stays lean, delegates work.

**Responsibilities:**
- Route to appropriate workflows
- Spawn and coordinate subagents
- Handle checkpoints and user interaction
- Present next steps

**Tools:** All tools, but primarily coordinates

**State consumed:** STATE.md, config.json, ROADMAP.md

**Not a subagent** - this is the main Claude instance coordinating everything.

---

### 2. gsd-planner

**Role:** Create executable PLAN.md files for phases.

**Responsibilities:**
- Decompose phase goals into tasks
- Assign dependencies and waves for parallel execution
- Define must_haves for goal-backward verification
- Determine TDD vs standard tasks
- Create frontmatter with metadata

**Output:** `{phase}-{plan}-PLAN.md` files in phase directory

**Context consumed:**
- ROADMAP.md (phase goals)
- CONTEXT.md (user decisions for this phase)
- RESEARCH.md (technical findings)
- REQUIREMENTS.md (what must be delivered)
- STATE.md (project position, decisions)

**Model profiles:**
| Profile | Model |
|---------|-------|
| quality | opus |
| balanced | opus |
| budget | sonnet |

---

### 3. gsd-executor

**Role:** Execute PLAN.md tasks autonomously.

**Responsibilities:**
- Read and execute plan tasks in order
- Handle deviation rules (auto-fix bugs, ask about architectural changes)
- Commit each task atomically
- Handle authentication gates
- Create SUMMARY.md on completion
- Update STATE.md

**Output:** Code changes, commits, `{phase}-{plan}-SUMMARY.md`

**Context consumed:**
- PLAN.md (what to execute)
- STATE.md (project context)
- config.json (commit behavior)

**Key behaviors:**
- Fresh 200k context per plan
- Commits after each task (atomic)
- Creates dynamic checkpoints for auth gates
- Tracks deviations for summary

**Model profiles:**
| Profile | Model |
|---------|-------|
| quality | opus |
| balanced | sonnet |
| budget | sonnet |

---

### 4. gsd-verifier

**Role:** Goal-backward verification of phase completion.

**Responsibilities:**
- Derive must_haves from phase goal
- Check truths (observable behaviors)
- Check artifacts (files exist, substantive, not stubs)
- Check wiring (components connected)
- Scan for anti-patterns
- Identify human verification needs
- Generate gap closure recommendations

**Output:** `{phase}-VERIFICATION.md` with status (passed/gaps_found/human_needed)

**Context consumed:**
- PLAN.md must_haves
- Phase goal from ROADMAP.md
- SUMMARY.md files (claims to verify)
- Actual codebase files

**Model profiles:**
| Profile | Model |
|---------|-------|
| quality | opus |
| balanced | sonnet |
| budget | haiku |

---

### 5. gsd-debugger

**Role:** Investigate issues and find root causes.

**Responsibilities:**
- Gather symptoms (pre-filled from UAT or user-provided)
- Form and test hypotheses
- Read relevant code
- Identify root cause with evidence
- Suggest fix direction

**Output:** `.planning/debug/{slug}.md` with investigation trace

**Context consumed:**
- Symptoms (expected vs actual)
- Codebase files for investigation

**Model profiles:**
| Profile | Model |
|---------|-------|
| quality | opus |
| balanced | sonnet |
| budget | sonnet |

---

### 6. gsd-phase-researcher

**Role:** Research for a specific phase (focused investigation).

**Responsibilities:**
- Query Context7 for library docs
- Verify with official documentation
- Cross-reference findings
- Produce RESEARCH.md with recommendations

**Output:** `{phase}-RESEARCH.md` in phase directory

**Context consumed:**
- CONTEXT.md (what user decided - informs research focus)
- Phase goal from ROADMAP.md

**Model profiles:**
| Profile | Model |
|---------|-------|
| quality | opus |
| balanced | sonnet |
| budget | haiku |

---

### 7. gsd-project-researcher

**Role:** Broad ecosystem research for new project/milestone.

**Responsibilities:**
- Survey technology landscape
- Map feature categories
- Document architecture patterns
- Catalog domain pitfalls

**Output:** Multiple files in `.planning/research/`

**Model profiles:**
| Profile | Model |
|---------|-------|
| quality | opus |
| balanced | sonnet |
| budget | haiku |

---

### 8. gsd-plan-checker

**Role:** Verify PLAN.md files before execution.

**Responsibilities:**
- Check plans have valid frontmatter
- Verify tasks are actionable
- Check dependencies are correct
- Ensure must_haves are verifiable

**Output:** VERIFICATION PASSED or ISSUES FOUND

**Model profiles:**
| Profile | Model |
|---------|-------|
| quality | opus |
| balanced | sonnet |
| budget | haiku |

---

### 9. gsd-codebase-mapper

**Role:** Analyze codebase structure.

**Responsibilities:**
- Read-only exploration
- Extract patterns and structure
- Generate codebase documentation

**Output:** Files in `.planning/codebase/`

**Model profiles:**
| Profile | Model |
|---------|-------|
| quality | opus |
| balanced | haiku |
| budget | haiku |

---

## Stage Transitions

GSD has a defined progression through stages. Each stage produces artifacts consumed by the next.

```
NEW PROJECT/MILESTONE
    |
    v
+-------------------+
| 1. QUESTIONING    | <-- gather user's vision
+-------------------+
    |
    v
+-------------------+
| 2. RESEARCH       | <-- survey ecosystem
+-------------------+
    |
    v
+-------------------+
| 3. REQUIREMENTS   | <-- define what to build
+-------------------+
    |
    v
+-------------------+
| 4. ROADMAP        | <-- structure into phases
+-------------------+
    |
    |  For each phase:
    v
    +-------------------+
    | 5. DISCUSS PHASE  | <-- capture implementation decisions
    +-------------------+
        |
        v
    +-------------------+
    | 6. PHASE RESEARCH | <-- investigate specifics
    +-------------------+
        |
        v
    +-------------------+
    | 7. PLAN PHASE     | <-- create PLAN.md files
    +-------------------+
        |
        v
    +-------------------+
    | 8. EXECUTE PHASE  | <-- run plans, create code
    +-------------------+
        |
        v
    +-------------------+
    | 9. VERIFY PHASE   | <-- goal-backward verification
    +-------------------+
        |
        v  (gaps found? -> plan-phase --gaps -> execute -> verify again)
        |
    +-------------------+
    | 10. UAT (VERIFY   | <-- human testing
    |     WORK)         |
    +-------------------+
        |
        v  (issues? -> diagnose -> plan --gaps -> execute -> UAT again)
        |
    +-------------------+
    | 11. TRANSITION    | <-- mark phase complete, next phase
    +-------------------+
    |
    v  (more phases? -> repeat 5-11)
    |
+-------------------+
| 12. COMPLETE      | <-- archive milestone, tag release
|     MILESTONE     |
+-------------------+
```

---

## State Management

GSD uses explicit files to maintain state across context windows.

### STATE.md - Living Memory

**Purpose:** Project's short-term memory spanning all sessions.

**Key sections:**
- **Project Reference**: Core value, current focus
- **Current Position**: Phase X of Y, Plan A of B, Status
- **Progress**: Visual progress bar
- **Accumulated Context**: Recent decisions, blockers
- **Session Continuity**: Last session, resume file

**Size constraint:** Under 100 lines (digest, not archive)

**Updated:** After every significant action

**Read:** First step of every workflow

---

### PROJECT.md - Project Vision

**Purpose:** What we're building, requirements, decisions.

**Key sections:**
- What This Is (accurate description)
- Core Value (the ONE thing)
- Requirements (Validated/Active/Out of Scope)
- Key Decisions (with outcomes)
- Constraints

**Evolves:** At milestone completion and phase transitions

---

### ROADMAP.md - Phase Structure

**Purpose:** What phases exist and their status.

**Key sections:**
- Milestones overview
- Phase list with goals
- Progress table

**Updated:** At phase transitions, milestone completion

---

### config.json - Behavior Configuration

**Purpose:** Control workflow behavior.

**Key settings:**
- `model_profile`: quality/balanced/budget
- `commit_docs`: true/false
- `mode`: yolo/interactive/custom
- Gates for confirmation prompts

---

### .continue-here.md - Mid-Plan Resume

**Purpose:** Resume mid-plan after context window reset.

**Key sections:**
- Current state
- Completed work
- Remaining work
- Decisions made
- Next action

**Lifecycle:** Created on interruption, deleted on resume

---

## Wave-Based Parallel Execution

Plans within a phase can run in parallel based on dependencies.

**How it works:**

1. **During planning:** Planner assigns `wave: N` to each plan
2. **During execution:** Orchestrator groups plans by wave
3. **Execution order:**
   - Wave 1: All plans spawn in parallel
   - Wait for Wave 1 to complete
   - Wave 2: All plans spawn in parallel
   - Repeat until all waves done

**Wave assignment rules:**
- `depends_on: []` -> Wave 1
- `depends_on: [01-01]` -> Wave 2 (after 01-01's wave + 1)
- Plans with no dependencies in same wave run in parallel

**Benefits:**
- Fresh context per subagent
- Parallel execution where possible
- Clear dependency management

---

## Checkpoint Protocol

Checkpoints are formalized human-in-the-loop points.

### Types

| Type | Usage | Example |
|------|-------|---------|
| `human-verify` | 90% - Claude built it, human confirms visually | Check deployed app works |
| `decision` | 9% - Human chooses between options | Select auth provider |
| `human-action` | 1% - Truly unavoidable manual step | Click email verification link |

### Authentication Gates

**Special case:** When CLI returns auth error, executor creates dynamic `human-action` checkpoint.

**Flow:**
1. Claude runs CLI command
2. Gets auth error
3. Creates checkpoint asking user to authenticate
4. User authenticates
5. Claude retries command
6. Continues execution

**Key principle:** Claude automates everything with CLI/API. Checkpoints are for verification and decisions, not manual work.

---

## Subagent Spawning Pattern

GSD uses Task tool to spawn fresh subagents.

```
Orchestrator (main context)
    |
    +-- Task(subagent_type="gsd-executor") --> Fresh 200k context
    |
    +-- Task(subagent_type="gsd-verifier") --> Fresh 200k context
    |
    +-- Task(subagent_type="gsd-planner") --> Fresh 200k context
```

**Benefits:**
- Each agent starts with full 200k tokens
- No context pollution between agents
- Orchestrator stays lean (~10-15% context)
- Quality maintained throughout long projects

**How subagents return:**
- Structured markdown with status
- SUMMARY.md or VERIFICATION.md created
- Commit made
- Orchestrator reads result files

---

## Gap Closure Loop

When verification finds gaps or UAT finds issues:

```
VERIFICATION finds gaps
    |
    v
/gsd:plan-phase {X} --gaps
    |
    v
Creates additional PLAN.md files (with gap_closure: true)
    |
    v
/gsd:execute-phase {X} --gaps-only
    |
    v
Executes only gap closure plans
    |
    v
VERIFICATION runs again
    |
    v
(repeat until passed)
```

---

## UAT (User Acceptance Testing)

Interactive testing workflow with persistent state.

**Philosophy:** Show expected, ask if reality matches.

**Flow:**
1. Extract testable deliverables from SUMMARY.md
2. Create UAT.md with all tests
3. Present tests one at a time
4. User responds: pass/issue/skip
5. Infer severity from natural language
6. If issues: auto-diagnose root causes
7. If issues: auto-plan fixes
8. Verify fix plans
9. Ready for --gaps-only execution

**State persistence:** UAT.md survives /clear, shows progress and gaps.

---

## Compatibility with Ralph Loop

### What Translates Directly

| GSD Pattern | Ralph Loop Equivalent |
|-------------|----------------------|
| Fresh context per subagent | Fresh context per iteration |
| STATE.md persistence | progress file persistence |
| PLAN.md as execution instructions | phase plans as iteration prompts |
| Checkpoint protocol | Q&A flow with AskUserQuestion |
| Wave-based ordering | Dependency-ordered iteration |
| Deviation rules | Build command's auto-fix behavior |

### What Needs Adaptation

| GSD Pattern | Ralph Loop Consideration |
|-------------|-------------------------|
| Task tool spawning | Ralph Loop doesn't spawn subagents - single iteration model |
| Parallel execution | Ralph Loop is sequential - can order by dependency |
| Model selection | Ralph Loop uses configured model, not per-agent selection |
| Orchestrator staying lean | Ralph Loop's TUI is the orchestrator layer |

### Recommended Approach

1. **Persona as prompt context, not separate agents**
   - Instead of spawning gsd-planner, include planner instructions in prompt
   - Each iteration gets fresh context with appropriate persona instructions

2. **Stage as iteration type**
   - `plan --mode=research` -> research persona prompt
   - `plan --mode=plan` -> planner persona prompt
   - `build` -> executor persona prompt

3. **STATE.md equivalent**
   - Ralph Loop's progress file already serves this purpose
   - Extend progress file with GSD-style sections if needed

4. **Checkpoint as Q&A**
   - Ralph Loop already has Q&A flow
   - Map checkpoint types to question formats

---

## Key Patterns for Ralph Loop

### 1. Goal-Backward Verification

From verify-phase.md:

```
Task completion != Goal achievement

Goal-backward verification:
1. What must be TRUE for the goal to be achieved?
2. What must EXIST for those truths to hold?
3. What must be WIRED for those artifacts to function?
```

**Ralph Loop application:** After build iterations, add verification iteration that checks goal achievement, not just task completion.

### 2. Deviation Rules

From execute-plan.md:

```
Rule 1: Auto-fix bugs (fix immediately)
Rule 2: Auto-add missing critical functionality
Rule 3: Auto-fix blocking issues
Rule 4: Ask about architectural changes

Priority: If Rule 4 applies -> STOP and ask
          If Rules 1-3 apply -> Fix automatically
```

**Ralph Loop application:** Include these rules in build prompts.

### 3. Atomic Commits

From execute-plan.md:

```
After each task:
1. Commit immediately
2. Track commit hash for summary
3. Include in iteration result
```

**Ralph Loop application:** Already does this in build command.

### 4. Continuation Format

From continuation-format.md:

```
## Next Up

**{identifier}: {name}** -- {one-line description}

`{command}`

<sub>`/clear` first -> fresh context window</sub>
```

**Ralph Loop application:** Progress file already shows next steps.

---

## Files Created by GSD

| Stage | Files Created | Location |
|-------|--------------|----------|
| Init | PROJECT.md, ROADMAP.md, STATE.md, config.json | .planning/ |
| Research | SUMMARY.md, STACK.md, FEATURES.md, etc. | .planning/research/ |
| Requirements | REQUIREMENTS.md | .planning/ |
| Discuss | {phase}-CONTEXT.md | .planning/phases/XX-name/ |
| Phase Research | {phase}-RESEARCH.md | .planning/phases/XX-name/ |
| Plan | {phase}-{plan}-PLAN.md | .planning/phases/XX-name/ |
| Execute | {phase}-{plan}-SUMMARY.md | .planning/phases/XX-name/ |
| Verify | {phase}-VERIFICATION.md | .planning/phases/XX-name/ |
| UAT | {phase}-UAT.md | .planning/phases/XX-name/ |
| Debug | {slug}.md | .planning/debug/ |
| Milestone | MILESTONES.md, v{X.Y}-*.md | .planning/, .planning/milestones/ |

---

## Implications for Ralph Loop v1.3

### Persona Definition Strategy

Define personas as prompt prefixes, not separate agents:

```rust
enum Persona {
    Researcher,    // Survey ecosystem, produce findings
    Planner,       // Decompose goals into tasks
    Executor,      // Execute tasks, handle deviations
    Verifier,      // Check goal achievement
    Debugger,      // Find root causes
}
```

Each persona gets specific instructions in the iteration prompt.

### Stage Tracking

Track current stage in progress file:

```yaml
workflow:
  stage: planning  # research | planning | executing | verifying
  persona: planner
  checkpoint: null  # or type of pending checkpoint
```

### Transition Logic

Define clear triggers for stage transitions:

- research -> planning: research files created
- planning -> executing: all PLAN.md files created
- executing -> verifying: all SUMMARY.md files exist
- verifying -> UAT: verification passed
- UAT -> done: all tests passed

### Checkpoint Flow

Map to existing Q&A:

```rust
match checkpoint_type {
    HumanVerify => ask_with_options("approved", "describe issues"),
    Decision => ask_with_options(option_list),
    HumanAction => ask_with_confirmation("done"),
}
```

---

## Sources

All findings from direct analysis of:
- `~/.claude/get-shit-done/workflows/*.md`
- `~/.claude/get-shit-done/templates/*.md`
- `~/.claude/get-shit-done/references/*.md`

Confidence: HIGH - primary source analysis, no inference required.
