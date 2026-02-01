# Integration Pitfalls: GSD Multi-Agent Pattern + Ralph Loop

**Research Date:** 2026-02-01
**Context:** v1.3 Hardening - Adding GSD personas to existing Ralph Loop
**Confidence:** HIGH (based on direct codebase analysis)

## Executive Summary

The Ralph Loop and GSD patterns have fundamentally different context management philosophies that create specific integration challenges. Ralph Loop uses a **single progress file as external memory** with fresh context per iteration, while GSD uses **multiple agents with specialized system prompts** coordinated by an orchestrator. The key challenge is preserving persona-specific state across Ralph Loop iterations while maintaining the simplicity of the progress file memory model.

---

## Pattern Analysis

### Ralph Loop Pattern (Current rslph)

**Core Architecture:**
```
Iteration N:
  1. Read progress.md from disk
  2. Spawn fresh Claude process with build prompt + progress content
  3. Claude executes one task, outputs updated progress.md
  4. Parse response, write progress.md atomically
  5. Repeat until RALPH_DONE or max iterations
```

**Key Characteristics:**
- **Stateless iterations:** Each Claude invocation starts with zero context history
- **Progress file as memory:** All state serialized to markdown
- **Single system prompt:** Same `build_prompt` for all iterations
- **Simple state machine:** Starting -> Running -> IterationComplete -> Done
- **No persona switching:** Same role throughout execution

**Relevant Code (src/build/iteration.rs):**
```rust
// Fresh subprocess each iteration
let system_prompt = get_build_prompt_for_mode(ctx.mode);
let user_input = format!(
    "## Current Progress\n\n{}\n\n## Instructions\n\nExecute the next incomplete task.",
    ctx.progress.to_markdown()
);
```

### GSD Multi-Agent Pattern

**Core Architecture:**
```
Orchestrator:
  1. Spawns specialized subagents via Task tool
  2. Each agent has role-specific system prompt
  3. Agents communicate via file artifacts (.planning/)
  4. Orchestrator coordinates handoffs

Agent Types:
  - gsd-executor: Implements code with deviation rules
  - gsd-verifier: Validates phase goal achievement
  - gsd-planner: Creates detailed plan files
  - gsd-researcher: Surveys ecosystem/technology
```

**Key Characteristics:**
- **Persona-specific prompts:** Each agent type has distinct system prompt
- **File-based state:** STATE.md, SUMMARY.md, .continue-here.md
- **Checkpoint mechanism:** Agents can pause and return structured state
- **Wave-based parallelism:** Independent plans run simultaneously
- **Fresh agents, not resumption:** New agent with explicit state vs resume

**Relevant Workflow (execute-plan.md):**
```markdown
## checkpoint_return_for_orchestrator

When spawned by orchestrator and hitting checkpoint:
- Return structured state including completed tasks table
- Orchestrator presents to user
- NEW agent spawned with previous state inlined
```

---

## Key Compatibility Challenges

### Challenge 1: Persona Context Switching

**The Problem:**
Ralph Loop uses a single `build_prompt` across all iterations. GSD personas (requirements clarifier, testing strategist, executor, verifier) each need different system prompts and behavioral constraints.

**Current Ralph Limitation (src/prompts/modes.rs):**
```rust
pub enum PromptMode {
    Basic,
    Gsd,
    GsdTdd,
}
// Mode is fixed at build start, not dynamic per iteration
```

**GSD Expectation:**
Different phases require different personas:
- Planning phase: Need questioning + research personas
- Execution phase: Need executor persona with deviation rules
- Verification phase: Need verifier persona

**Impact:** Cannot switch from executor to verifier mid-build without changing the core iteration model.

---

### Challenge 2: State Serialization Scope

**The Problem:**
GSD agents accumulate context through execution (decisions made, files created, deviations encountered). Ralph Loop only preserves what fits in progress.md.

**GSD State Components:**
1. **Completed tasks with commit hashes** - Ralph has this
2. **Decisions made and rationale** - Ralph has `recent_attempts` but not rich decisions
3. **Authentication gates encountered** - Not in Ralph
4. **Deviation tracking** - Not in Ralph
5. **Checkpoint state** - Not in Ralph
6. **Agent-to-agent handoff context** - Not in Ralph

**Progress File Limitations (src/progress.rs):**
```rust
pub struct ProgressFile {
    pub name: String,
    pub status: String,
    pub analysis: String,
    pub tasks: Vec<TaskPhase>,       // Task completion tracking
    pub testing_strategy: String,
    pub completed_this_iteration: Vec<String>,
    pub recent_attempts: Vec<Attempt>, // Limited failure memory
    pub iteration_log: Vec<IterationEntry>,
}
// No fields for: decisions, deviations, auth gates, persona context
```

**Impact:** Rich GSD context cannot be preserved across Ralph iterations without extending progress file schema.

---

### Challenge 3: Checkpoint Handling

**The Problem:**
GSD checkpoints pause execution for user interaction (verification, decisions, manual actions). Ralph Loop runs until task completion or max iterations.

**GSD Checkpoint Flow:**
```
Agent executes tasks ->
Hits checkpoint ->
Returns structured state to orchestrator ->
User interacts ->
NEW agent spawned with state
```

**Ralph Loop Flow:**
```
Iteration executes ->
Writes progress.md ->
Next iteration starts immediately
```

**Key Difference:** Ralph has no mechanism to pause mid-iteration for user input. The `AskUserQuestion` handling in planning mode (src/planning/command.rs) uses session resume, but build mode (src/build/iteration.rs) has no equivalent.

**Impact:** User-interactive checkpoints require architectural changes to Ralph Loop.

---

### Challenge 4: Orchestrator vs Direct Execution

**The Problem:**
GSD uses an orchestrator pattern where a thin coordinator spawns heavy workers. Ralph Loop IS the worker - there is no orchestrator layer.

**GSD Structure:**
```
User -> Orchestrator (thin context) -> Subagent (full context)
                                    -> Subagent (full context)
```

**Ralph Structure:**
```
User -> Build Loop -> Claude subprocess (full context)
                   -> Claude subprocess (full context)
```

**Impact:** Adding GSD patterns means either:
1. Keeping Ralph as-is but extending progress file (simpler)
2. Adding orchestrator layer around Ralph (more complex, more flexible)

---

### Challenge 5: Fresh Context Philosophy

**The Problem:**
Both patterns use fresh context per execution unit. But they differ on what constitutes the "unit" and how state is passed.

| Aspect | Ralph Loop | GSD |
|--------|-----------|-----|
| Execution unit | Iteration (1 task) | Plan (multiple tasks) |
| State mechanism | progress.md | Multiple files (.planning/) |
| Context lifetime | Single iteration | Until checkpoint or plan complete |
| State format | Markdown (parsed) | Markdown + YAML frontmatter |

**Impact:** GSD personas expect richer context files than progress.md provides.

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Inline Persona Switching

**What it looks like:**
```rust
// BAD: Switching system prompt mid-iteration based on task type
let prompt = match current_task.task_type {
    TaskType::Research => RESEARCH_PERSONA,
    TaskType::Execute => EXECUTOR_PERSONA,
    TaskType::Verify => VERIFIER_PERSONA,
};
```

**Why it fails:**
- Task type detection is unreliable
- Context from previous persona is lost
- System prompt changes confuse Claude mid-conversation

**Better approach:** Persona is determined at iteration start, not mid-execution.

---

### Anti-Pattern 2: Bloated Progress File

**What it looks like:**
```rust
pub struct ProgressFile {
    // Original fields...
    pub gsd_context: GsdContext,
    pub decisions: Vec<Decision>,
    pub deviations: Vec<Deviation>,
    pub auth_gates: Vec<AuthGate>,
    pub checkpoint_state: Option<CheckpointState>,
    pub persona_history: Vec<PersonaTransition>,
    // ...50 more fields
}
```

**Why it fails:**
- Progress file is parsed and included in EVERY iteration prompt
- Large files consume context window
- Schema complexity increases parse failure risk
- Violates "small state, fresh context" philosophy

**Better approach:** Separate files for extended state, only include essentials in progress.md.

---

### Anti-Pattern 3: Stateful Session Resume

**What it looks like:**
```rust
// BAD: Trying to resume Claude session across Ralph iterations
let args = vec![
    "--resume".to_string(),
    previous_session_id.to_string(),
    // ...
];
```

**Why it fails:**
- Session IDs are tied to specific Claude instances
- No guarantee session can be resumed after process termination
- Breaks the fresh context model
- GSD explicitly uses NEW agents, not resume

**Better approach:** Serialize all needed state to files, start fresh each iteration.

---

### Anti-Pattern 4: Implicit Persona Inheritance

**What it looks like:**
```markdown
## Progress
Phase: Verification
// Assume Claude knows to behave as verifier because of phase name
```

**Why it fails:**
- System prompt is what determines Claude behavior
- Phase name in progress file doesn't change system prompt
- Leads to executor behavior during verification phase

**Better approach:** Explicit persona selection at iteration start, based on phase/task metadata.

---

## Recommended Integration Approach

### Approach 1: Extended Progress Schema (Minimal Change)

**Concept:** Add persona and GSD-specific fields to progress file without changing core loop.

**Changes Required:**

```rust
// Extended progress structure
pub struct ProgressFile {
    // Existing fields...

    // NEW: Persona selection for next iteration
    pub next_persona: Option<Persona>,

    // NEW: Decisions made (compact format)
    pub decisions: Vec<CompactDecision>,

    // NEW: Checkpoint if paused
    pub checkpoint: Option<Checkpoint>,
}

pub struct CompactDecision {
    pub iteration: u32,
    pub what: String,      // Brief description
    pub why: String,       // Rationale
}

pub struct Checkpoint {
    pub checkpoint_type: String,  // "human-verify", "decision", "human-action"
    pub awaiting: String,         // What user needs to do
    pub resume_task: u32,         // Which task to resume after
}
```

**Iteration Logic Change:**
```rust
// Before spawning Claude, select system prompt based on persona
let persona = ctx.progress.next_persona.unwrap_or(Persona::Executor);
let system_prompt = get_prompt_for_persona(persona, ctx.mode);
```

**Checkpoint Handling:**
```rust
// After parsing response, check for checkpoint
if let Some(checkpoint) = updated_progress.checkpoint {
    // Pause iteration loop
    return Ok(IterationResult::Checkpoint(checkpoint));
}
```

**Pros:**
- Minimal changes to existing code
- Progress file remains primary state mechanism
- Backward compatible (old files work, new fields optional)

**Cons:**
- Progress file schema grows
- Checkpoint handling requires new IterationResult variant
- User interaction during checkpoints needs TUI support

---

### Approach 2: Companion State Files (More GSD-like)

**Concept:** Keep progress.md simple, add companion files for GSD context.

**File Structure:**
```
project/
  progress.md           # Task tracking (existing)
  .gsd-context.json     # Persona, decisions, deviations
  .gsd-checkpoint.md    # If paused at checkpoint
```

**Progress File (unchanged):**
```markdown
# Progress: Project Name

## Status
In Progress

## Tasks
### Phase 1
- [x] Task 1
- [ ] Task 2
```

**GSD Context File (new):**
```json
{
  "current_persona": "executor",
  "decisions": [
    {"iteration": 3, "what": "Used jose for JWT", "why": "Better TS types"}
  ],
  "deviations": [
    {"rule": 1, "description": "Fixed case-sensitive email check"}
  ],
  "next_persona": null
}
```

**Iteration Logic:**
```rust
// Load both files
let progress = ProgressFile::load(&ctx.progress_path)?;
let gsd_context = GsdContext::load(&ctx.gsd_context_path).unwrap_or_default();

// Build prompt with both
let system_prompt = get_prompt_for_persona(gsd_context.current_persona);
let user_input = format!(
    "## Progress\n{}\n\n## Context\n{}\n\n## Instructions\n...",
    progress.to_markdown(),
    gsd_context.to_summary()
);
```

**Pros:**
- Progress file stays simple and backward compatible
- GSD context can grow without affecting core loop
- Cleaner separation of concerns
- JSON for structured data, Markdown for human-readable state

**Cons:**
- Two files to manage atomically
- Need to keep files in sync
- More complexity in iteration setup

---

### Approach 3: Persona-Specific Prompts (System Prompt Library)

**Concept:** Define persona prompts as a library, select at iteration start.

**Prompt Structure (src/planning/personas.rs extended):**
```rust
pub enum Persona {
    Executor,           // Standard build execution
    ExecutorWithTdd,    // TDD-focused execution
    Verifier,           // Goal verification
    Researcher,         // Ecosystem survey
    Planner,            // Plan generation
}

impl Persona {
    pub fn system_prompt(&self) -> &'static str {
        match self {
            Persona::Executor => EXECUTOR_PROMPT,
            Persona::ExecutorWithTdd => TDD_EXECUTOR_PROMPT,
            Persona::Verifier => VERIFIER_PROMPT,
            Persona::Researcher => RESEARCHER_PROMPT,
            Persona::Planner => PLANNER_PROMPT,
        }
    }

    pub fn from_task(task: &Task) -> Self {
        // Infer persona from task metadata
        if task.description.contains("[verify]") {
            Persona::Verifier
        } else if task.description.contains("[research]") {
            Persona::Researcher
        } else {
            Persona::Executor
        }
    }
}
```

**Progress File Extension:**
```markdown
## Tasks

### Phase 1: Foundation
- [x] Set up project structure
- [ ] [verify] Validate project compiles
- [ ] [research] Survey authentication libraries
- [ ] Implement auth flow
```

**Pros:**
- Task-driven persona selection
- Personas as first-class concept
- Can add new personas without schema changes

**Cons:**
- Task description parsing is fragile
- Metadata in task description is non-standard
- Requires discipline in task formatting

---

## Recommended Path Forward

**For v1.3 Hardening, recommend Approach 1 (Extended Progress Schema) because:**

1. **Minimal architectural change:** Core Ralph Loop stays intact
2. **Incremental adoption:** Can add fields without breaking existing progress files
3. **Proven pattern:** GSD's `.continue-here.md` is similar concept
4. **Clear migration:** Old files work, new features use new fields

**Implementation Priority:**

| Priority | Feature | Rationale |
|----------|---------|-----------|
| P0 | Persona field in progress | Enables prompt switching |
| P0 | Checkpoint result variant | Enables user interaction pauses |
| P1 | Compact decisions list | Preserves decision context |
| P1 | TUI checkpoint display | User sees what's awaited |
| P2 | Deviation tracking | Full GSD compliance |
| P2 | Authentication gate handling | Service integration support |

---

## State Management Patterns That Work

### Pattern 1: Explicit Persona Declaration

**In progress.md:**
```markdown
## Status
In Progress

## Next Persona
verifier

## Tasks
...
```

**In iteration.rs:**
```rust
let persona = progress.next_persona.take(); // Consume, don't persist
let system_prompt = persona.map(|p| p.system_prompt())
    .unwrap_or(get_build_prompt_for_mode(ctx.mode));
```

**Why it works:** Persona is declared once, used once, then cleared. No accumulation.

---

### Pattern 2: Checkpoint as Iteration Result

```rust
pub enum IterationResult {
    Continue { tasks_completed: u32 },
    Done(DoneReason),
    Timeout,
    Checkpoint(CheckpointData),  // NEW
}

pub struct CheckpointData {
    pub checkpoint_type: String,
    pub task_name: String,
    pub awaiting: String,
    pub resume_instructions: String,
}
```

**In build loop:**
```rust
Ok(IterationResult::Checkpoint(data)) => {
    // Display to user
    show_checkpoint_ui(&data, &ctx.tui_tx);

    // Wait for user response
    let response = wait_for_user_response().await;

    // Update progress with response
    ctx.progress.checkpoint_response = Some(response);
    ctx.progress.write(&ctx.progress_path)?;

    // Continue to next iteration (same iteration number)
    BuildState::Running { iteration }
}
```

**Why it works:** Checkpoints are first-class, user interaction is explicit, state is persisted before waiting.

---

### Pattern 3: Compact Decision Log

```rust
pub struct Attempt {
    pub iteration: u32,
    pub tried: String,
    pub result: String,
    pub next: Option<String>,
    pub decision: Option<String>,  // NEW: If this was a decision point
}
```

**In progress.md:**
```markdown
## Recent Attempts

### Iteration 3

- Tried: JWT implementation
- Result: Success
- Decision: Used jose library over jsonwebtoken (better TS types)
```

**Why it works:** Decisions are logged with attempts, not separate structure. Natural evolution of existing schema.

---

## Implementation Checklist

- [ ] Add `next_persona: Option<String>` to ProgressFile
- [ ] Add `checkpoint: Option<Checkpoint>` to ProgressFile
- [ ] Add `Checkpoint` variant to IterationResult
- [ ] Create persona prompt library in src/planning/personas.rs
- [ ] Extend TUI for checkpoint display and user input
- [ ] Add `decision` field to Attempt struct
- [ ] Test persona switching across iterations
- [ ] Test checkpoint pause and resume
- [ ] Document persona selection rules
- [ ] Update progress.md parsing for new fields

---

## Sources

- Direct codebase analysis:
  - `/Users/vmakaev/Non-Work/rslph/src/progress.rs`
  - `/Users/vmakaev/Non-Work/rslph/src/build/iteration.rs`
  - `/Users/vmakaev/Non-Work/rslph/src/build/state.rs`
  - `/Users/vmakaev/Non-Work/rslph/src/build/command.rs`
  - `/Users/vmakaev/Non-Work/rslph/src/planning/command.rs`
  - `/Users/vmakaev/Non-Work/rslph/src/planning/personas.rs`
- GSD skill files:
  - `/Users/vmakaev/.claude/get-shit-done/templates/state.md`
  - `/Users/vmakaev/.claude/get-shit-done/templates/continue-here.md`
  - `/Users/vmakaev/.claude/get-shit-done/workflows/execute-plan.md`
  - `/Users/vmakaev/.claude/get-shit-done/workflows/execute-phase.md`
  - `/Users/vmakaev/.claude/get-shit-done/workflows/resume-project.md`
  - `/Users/vmakaev/.claude/get-shit-done/workflows/transition.md`
  - `/Users/vmakaev/.claude/get-shit-done/references/continuation-format.md`
