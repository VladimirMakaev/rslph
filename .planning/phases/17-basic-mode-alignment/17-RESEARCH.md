# Phase 17: Basic Mode Alignment - Research

**Researched:** 2026-02-02
**Domain:** CLI autonomous development loop, portableralph compatibility
**Confidence:** HIGH

## Summary

This research verifies alignment between rslph basic mode and the snarktank/ralph reference implementation. Through direct analysis of the GitHub repository files (prompt.md, CLAUDE.md, ralph.sh, prd.json.example), I compared the reference behavior against rslph's current implementation.

**Key finding:** rslph basic mode is already substantially aligned with portableralph. The core loop behavior, termination conditions, and VCS integration all match. The remaining gaps are primarily in prompt content details and progress file format, which can be addressed through targeted updates.

**Primary recommendation:** Focus changes on MODE-02 (prompt alignment) and MODE-04 (progress file format), as MODE-03 and MODE-05 are already implemented correctly.

## Per-Requirement Analysis

### MODE-02: Basic mode prompts match portableralph

**Requirement:** Use exact prompt structure from snarktank/ralph reference implementation

**Current State Analysis:**

| Aspect | portableralph | rslph Basic | Gap |
|--------|---------------|-------------|-----|
| **Task selection** | "Highest priority incomplete story" | "First incomplete task" | Minor (equivalent if ordered by priority) |
| **One task per iteration** | YES (explicit) | YES (explicit) | ALIGNED |
| **Quality checks** | "Run quality checks before commit" | Not mentioned | GAP |
| **Progress documentation** | "Append to progress.txt with date, story ID, learnings" | Update progress.md sections | FORMAT DIFFERENCE |
| **Completion marker** | `<promise>COMPLETE</promise>` | `RALPH_DONE` in Status | COMPATIBLE |
| **Story sizing** | "If you can't describe it in 2-3 sentences, split it" | "Each task completable in 1-2 iterations" | SIMILAR |

**portableralph Prompt Structure (from CLAUDE.md):**

1. Read PRD from `prd.json` and progress log from `progress.txt`
2. Verify correct branch from PRD
3. Select highest-priority incomplete user story
4. Implement that single story
5. Run quality checks (typecheck, lint, test)
6. Update documentation if patterns discovered
7. Commit with format: `feat: [Story ID] - [Story Title]`
8. Update PRD to mark story as complete
9. Append progress to progress.txt

**Key portableralph Emphases:**
- "All commits must pass project quality checks"
- "Keep changes focused and minimal"
- "Follow existing patterns"
- "Learnings for future iterations" section capturing patterns and gotchas
- Respond with `<promise>COMPLETE</promise>` when all stories pass

**rslph Current Basic Build Prompt:**
- One task per iteration (matches)
- Verify before marking complete (matches spirit)
- RALPH_DONE on completion (compatible)
- Missing: explicit quality check requirement
- Missing: explicit "learnings" documentation guidance

**Changes Needed for MODE-02:**

1. **Add quality check guidance** to PROMPT_build.md:
   ```markdown
   Before marking a task complete:
   - Verify implementation works as expected
   - Run any relevant quality checks (tests, typecheck, lint) if available
   - Keep changes focused and minimal
   ```

2. **Add story sizing guidance** to PROMPT_plan.md:
   ```markdown
   Each task should be describable in 2-3 sentences. If it requires more explanation, split it into smaller tasks.
   ```

3. **Ensure "Recent Attempts" guidance** emphasizes learnings:
   ```markdown
   When documenting attempts, include:
   - What was tried
   - What happened (success/failure)
   - Learnings/patterns discovered for future iterations
   ```

**Confidence:** HIGH - verified directly against snarktank/ralph repository

---

### MODE-03: Basic mode loop behavior identical

**Requirement:** Fresh context per iteration, one task per iteration, RALPH_DONE termination

**Current State Analysis:**

| Behavior | portableralph | rslph | Status |
|----------|---------------|-------|--------|
| Fresh subprocess each iteration | YES (ralph.sh spawns new process) | YES (run_single_iteration spawns Claude) | ALIGNED |
| Re-read progress at iteration start | YES (reads prd.json + progress.txt) | YES (`ctx.progress = ProgressFile::load()`) | ALIGNED |
| One task per iteration | YES (prompt instructs) | YES (prompt instructs) | ALIGNED |
| RALPH_DONE termination | YES (`<promise>COMPLETE</promise>`) | YES (`is_done()` checks for RALPH_DONE) | ALIGNED |
| All tasks complete termination | YES (all stories pass) | YES (`completed_tasks() == total_tasks()`) | ALIGNED |
| Max iterations limit | YES (configurable, default 10) | YES (config.max_iterations) | ALIGNED |
| Iteration timeout | Not explicit | YES (config.iteration_timeout) | ENHANCED |
| Timeout retry | Not explicit | YES (config.timeout_retries) | ENHANCED |

**Code Verification (from src/build/iteration.rs):**

```rust
// Step 1: Re-read progress file (may have been updated externally)
ctx.progress = ProgressFile::load(&ctx.progress_path)?;

// Step 2: Check for early exit conditions
if ctx.progress.is_done() {
    return Ok(IterationResult::Done(DoneReason::RalphDoneMarker));
}

if ctx.progress.completed_tasks() == ctx.progress.total_tasks()
    && ctx.progress.total_tasks() > 0
{
    return Ok(IterationResult::Done(DoneReason::AllTasksComplete));
}
```

**Code Verification (from src/build/state.rs):**

```rust
pub enum DoneReason {
    AllTasksComplete,      // All tasks in progress file marked complete
    RalphDoneMarker,       // RALPH_DONE marker detected in status
    MaxIterationsReached,  // Maximum iterations reached
    UserCancelled,         // User cancelled via Ctrl+C
    SingleIterationComplete, // Single iteration mode (--once flag)
}
```

**Changes Needed for MODE-03:** NONE

The loop behavior is already fully aligned with portableralph:
- Fresh Claude subprocess spawned each iteration via `run_single_iteration`
- Progress file re-read at each iteration start
- Prompt instructs one task per iteration
- RALPH_DONE marker checked via `is_done()` which checks `status.contains("RALPH_DONE")`
- All tasks complete checked before spawning
- Max iterations configurable

**Confidence:** HIGH - verified directly from source code

---

### MODE-04: Basic mode progress file format

**Requirement:** Match portableralph progress.txt structure for task tracking and learnings

**Current State Analysis:**

**portableralph Format:**
- Separate files: `prd.json` (task tracking) + `progress.txt` (learnings)
- prd.json: JSON with stories array, each having `passes: boolean`
- progress.txt: Append-only text with date, story ID, implementation details, learnings

**prd.json Example (from snarktank/ralph):**
```json
{
  "project": "MyApp",
  "branchName": "ralph/task-priority",
  "description": "Task Priority System",
  "userStories": [
    {
      "id": "US-001",
      "title": "Add priority field",
      "description": "As a developer, I need to store task priority",
      "acceptanceCriteria": ["Column accepts high/medium/low", "Default is medium"],
      "priority": 1,
      "passes": false,
      "notes": ""
    }
  ]
}
```

**progress.txt Format (reconstructed from CLAUDE.md):**
```
[date] [story-id] [thread-url]
- Implementation: [details]
- Files changed: [list]
- Learnings for future iterations:
  - [patterns discovered]
  - [gotchas encountered]
  - [useful context]
```

**rslph Current Format:**
- Single file: `progress.md` (integrated)
- Markdown with sections: Status, Analysis, Tasks, Testing Strategy, Completed This Iteration, Recent Attempts, Iteration Log
- Tasks as checkboxes: `- [ ] Task description` / `- [x] Task description`

**rslph progress.md Structure:**
```markdown
# Progress: [Plan Name]

## Status
In Progress (or RALPH_DONE)

## Analysis
[Brief analysis]

## Tasks
### Phase 1: [Name]
- [x] Completed task
- [ ] Pending task

## Testing Strategy
[Testing approach]

## Completed This Iteration
- [x] [Task just completed]

## Recent Attempts
### Iteration [N]
- Tried: [what]
- Result: [outcome]
- Next: [plan]

## Iteration Log
| Iteration | Started | Duration | Tasks Completed | Notes |
```

**Gap Analysis:**

| Feature | portableralph | rslph | Gap |
|---------|---------------|-------|-----|
| Task tracking | `passes: boolean` in JSON | `[x]`/`[ ]` checkbox | EQUIVALENT |
| Project name | `project` field | `# Progress: [name]` title | EQUIVALENT |
| Branch tracking | `branchName` field | Not tracked | GAP (optional) |
| Acceptance criteria | Per-story array | Not structured | GAP (optional) |
| Learnings persistence | Append-only progress.txt | Recent Attempts section | PARTIAL |
| Story notes | `notes` field per story | Not structured | GAP (optional) |
| Iteration history | Not explicit | Iteration Log table | ENHANCED |

**Changes Needed for MODE-04:**

The current rslph format already captures the essential semantics:
- Task completion tracking (checkboxes equivalent to `passes: boolean`)
- Failure memory (Recent Attempts equivalent to progress.txt learnings)
- Iteration tracking (Iteration Log enhanced over portableralph)

**Recommended minimal changes:**

1. **Rename "Recent Attempts" to "Learnings" or keep but emphasize learnings:**
   Update PROMPT_build.md to emphasize documenting learnings, not just failures:
   ```markdown
   ## Recent Attempts / Learnings

   Document what you learned each iteration:
   - Patterns discovered
   - Gotchas encountered
   - Useful context for future iterations
   ```

2. **Consider adding optional "Codebase Patterns" section:**
   portableralph maintains a "Codebase Patterns" section at the top of progress.txt for reusable insights. This could be added to rslph progress format, but is OPTIONAL.

**Decision:** Keep current progress.md format. The markdown format is more readable and self-contained than JSON + separate text file. The semantics are equivalent.

**Confidence:** HIGH - verified against prd.json.example and documentation

---

### MODE-05: Basic mode commit behavior

**Requirement:** Commit after each completed task (matching ralph.sh)

**Current State Analysis:**

**portableralph Behavior (from ralph.sh and CLAUDE.md):**
- Commit after implementing story with format: `feat: [Story ID] - [Story Title]`
- All commits must pass quality checks
- Git commit happens as part of story completion

**rslph Current Behavior (from src/build/iteration.rs):**

```rust
// Step 11: VCS auto-commit if tasks were completed
if tasks_completed > 0 {
    if let Some(ref vcs) = ctx.vcs {
        let commit_msg =
            format_iteration_commit(&ctx.project_name, ctx.current_iteration, tasks_completed);
        match vcs.commit_all(&commit_msg) {
            Ok(Some(hash)) => {
                ctx.log(&format!("[VCS] Committed: {} ({})", hash, vcs.vcs_type()));
            }
            Ok(None) => {
                ctx.log("[VCS] No file changes to commit");
            }
            Err(e) => {
                // VCS errors are warnings, not failures
                ctx.log(&format!("[VCS] Warning: {}", e));
            }
        }
    }
}
```

**Commit message format (from src/build/iteration.rs):**
```rust
fn format_iteration_commit(project_name: &str, iteration: u32, tasks_completed: u32) -> String {
    format!(
        "[{}][iter {}] Completed {} task(s)",
        project_name, iteration, tasks_completed
    )
}
```

**Comparison:**

| Aspect | portableralph | rslph | Status |
|--------|---------------|-------|--------|
| Auto-commit on completion | YES | YES | ALIGNED |
| VCS detection | Git only | Git + Sapling | ENHANCED |
| Commit message format | `feat: [Story ID] - [Story Title]` | `[Project][iter N] Completed M task(s)` | DIFFERENT |
| Quality check before commit | Required | Not enforced | GAP |
| Commit failure handling | Not explicit | Warning only, continues | ROBUST |

**Changes Needed for MODE-05:** MINIMAL

The commit behavior is already implemented correctly:
- VCS auto-commit after each iteration if tasks completed
- Support for both Git and Sapling

**Optional improvements:**
1. **Update commit message format** to be more semantic:
   ```
   [Project][iter N] feat: [Task description]
   ```
   or match portableralph exactly:
   ```
   feat: [Task description]
   ```

2. **Add quality check reminder** to prompt (covered in MODE-02)

**Decision:** Current commit behavior is aligned. The commit message format difference is stylistic, not functional.

**Confidence:** HIGH - verified from source code

---

## Architecture Patterns

### Current rslph Basic Mode Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        rslph CLI                                 │
│  ┌─────────┐    ┌─────────────┐    ┌──────────────────────────┐ │
│  │ plan    │    │ build       │    │ config                   │ │
│  │ command │    │ command     │    │ (max_iterations, etc)    │ │
│  └────┬────┘    └──────┬──────┘    └──────────────────────────┘ │
│       │                │                                         │
│       ▼                ▼                                         │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                    BuildContext                              │ │
│  │  - progress_path   - mode (Basic)   - cancel_token          │ │
│  │  - progress        - vcs            - iteration_tokens      │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                          │                                       │
│                          ▼                                       │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │              run_single_iteration()                          │ │
│  │  1. Re-read progress file                                    │ │
│  │  2. Check RALPH_DONE / all complete                          │ │
│  │  3. Spawn Claude subprocess                                  │ │
│  │  4. Parse response                                           │ │
│  │  5. Write updated progress                                   │ │
│  │  6. VCS commit if tasks completed                            │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### Pattern: State Machine Build Loop

The build command uses a state machine pattern:

```rust
enum BuildState {
    Starting,
    Running { iteration },
    IterationComplete { iteration, tasks_completed },
    Done { reason },
    Failed { error },
}
```

This provides clean state transitions and termination handling.

### Pattern: Progress File as Single Source of Truth

Unlike portableralph's split prd.json + progress.txt, rslph uses a single progress.md file that:
- Contains task list with checkboxes
- Contains status with RALPH_DONE marker
- Contains failure memory (Recent Attempts)
- Contains iteration history

This is a deliberate simplification that maintains semantic equivalence.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Progress file parsing | Custom regex parser | pulldown_cmark | Markdown is complex, edge cases abound |
| Atomic file writes | Direct fs::write | atomicwrites crate | Crash safety, data integrity |
| VCS operations | Manual git commands | Vcs trait + GitVcs | Abstraction for Git/Sapling support |
| Subprocess management | std::process | ClaudeRunner | Timeout, cancellation, stream handling |

---

## Common Pitfalls

### Pitfall 1: Prompt Drift
**What goes wrong:** Prompts diverge from reference implementation over time
**Why it happens:** Incremental changes without checking against reference
**How to avoid:** Document which parts of prompts are "canonical" vs "enhanced"
**Warning signs:** Users report different behavior than portableralph

### Pitfall 2: Breaking RALPH_DONE Detection
**What goes wrong:** Changes to status parsing break termination
**Why it happens:** Status field format changes or validation added
**How to avoid:** Keep `is_done()` implementation simple: `status.contains("RALPH_DONE")`
**Warning signs:** Build loops never terminate, or terminate prematurely

### Pitfall 3: VCS Commit Ordering
**What goes wrong:** Commits happen before progress file updated
**Why it happens:** Reordering iteration steps
**How to avoid:** Follow order: parse response -> write progress -> commit
**Warning signs:** Progress file doesn't match committed state

### Pitfall 4: Progress File Corruption
**What goes wrong:** Partial writes corrupt progress file
**Why it happens:** Non-atomic writes, crashes during write
**How to avoid:** Use atomicwrites crate (already implemented)
**Warning signs:** Corrupted progress files after crashes

---

## Code Examples

### Checking for RALPH_DONE (Current Implementation)

```rust
// Source: src/progress.rs
impl ProgressFile {
    /// Check if progress file indicates completion (PROG-01)
    pub fn is_done(&self) -> bool {
        self.status.contains("RALPH_DONE")
    }
}
```

### VCS Commit on Task Completion (Current Implementation)

```rust
// Source: src/build/iteration.rs
if tasks_completed > 0 {
    if let Some(ref vcs) = ctx.vcs {
        let commit_msg =
            format_iteration_commit(&ctx.project_name, ctx.current_iteration, tasks_completed);
        match vcs.commit_all(&commit_msg) {
            Ok(Some(hash)) => {
                ctx.log(&format!("[VCS] Committed: {} ({})", hash, vcs.vcs_type()));
            }
            // ...
        }
    }
}
```

### Fresh Context Each Iteration (Current Implementation)

```rust
// Source: src/build/iteration.rs
pub async fn run_single_iteration(ctx: &mut BuildContext) -> Result<IterationResult, RslphError> {
    // Step 1: Re-read progress file (may have been updated externally)
    ctx.progress = ProgressFile::load(&ctx.progress_path)?;

    // Step 2: Check for early exit conditions
    if ctx.progress.is_done() {
        return Ok(IterationResult::Done(DoneReason::RalphDoneMarker));
    }
    // ...

    // Step 5: Spawn fresh Claude subprocess
    let runner_result = ClaudeRunner::spawn(/* ... */).await;
    // ...
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Separate prd.json + progress.txt | Single progress.md | rslph design | Simpler, self-contained |
| `<promise>COMPLETE</promise>` | `RALPH_DONE` | rslph design | Shorter, equally clear |
| Amp-focused | Claude Code focused | rslph v1.0 | Better Claude integration |
| No TUI | Optional TUI | rslph v1.2 | Enhanced UX |

---

## Open Questions

### 1. Branch Tracking

**What we know:** portableralph tracks branch via `.last-branch` file and archives runs on branch switch

**What's unclear:** Is branch tracking essential for basic mode alignment?

**Recommendation:** LOW PRIORITY - not required for MODE requirements. Consider for future enhancement.

### 2. Acceptance Criteria

**What we know:** portableralph has structured `acceptanceCriteria` array per story

**What's unclear:** Should rslph add structured acceptance criteria to tasks?

**Recommendation:** LOW PRIORITY - checkbox tasks are semantically equivalent for completion tracking. Structured criteria could be added as enhancement but not required for alignment.

### 3. Quality Check Enforcement

**What we know:** portableralph REQUIRES quality checks pass before commit

**What's unclear:** Should rslph enforce quality checks, or just recommend in prompt?

**Recommendation:** Add to prompt as guidance (MODE-02). Enforcement is project-specific and may not always apply (some projects lack tests/lint).

---

## Implementation Recommendations

### Must Implement (Required for Requirements)

1. **MODE-02: Update PROMPT_build.md**
   - Add quality check guidance
   - Emphasize one task per iteration
   - Add learnings documentation guidance

2. **MODE-02: Update PROMPT_plan.md**
   - Add story sizing guidance ("2-3 sentences")

### Already Aligned (No Changes Needed)

3. **MODE-03: Loop behavior** - Already implemented correctly
4. **MODE-05: Commit behavior** - Already implemented correctly

### Optional (Nice to Have)

5. **MODE-04: Progress format enhancements**
   - Consider renaming "Recent Attempts" to "Learnings"
   - Consider adding "Codebase Patterns" section
   - NOT REQUIRED - current format is semantically equivalent

---

## Sources

### Primary (HIGH confidence)
- [snarktank/ralph prompt.md](https://github.com/snarktank/ralph/blob/main/prompt.md) - Build agent instructions
- [snarktank/ralph CLAUDE.md](https://github.com/snarktank/ralph/blob/main/CLAUDE.md) - Claude Code instructions
- [snarktank/ralph ralph.sh](https://github.com/snarktank/ralph/blob/main/ralph.sh) - Loop implementation
- [snarktank/ralph prd.json.example](https://github.com/snarktank/ralph/blob/main/prd.json.example) - PRD format
- rslph source code (src/build/iteration.rs, src/build/state.rs, src/progress.rs)

### Secondary (MEDIUM confidence)
- [snarktank/ralph README.md](https://github.com/snarktank/ralph) - Overview documentation
- Existing research at .planning/research/PORTABLERALPH.md

---

## Metadata

**Confidence breakdown:**
- MODE-02 (Prompts): HIGH - verified against actual snarktank/ralph files
- MODE-03 (Loop behavior): HIGH - verified from rslph source code
- MODE-04 (Progress format): HIGH - verified against prd.json.example and documentation
- MODE-05 (Commit behavior): HIGH - verified from rslph source code

**Research date:** 2026-02-02
**Valid until:** 2026-03-02 (stable domain, 30 days)
