# Portableralph Alignment Research

**Domain:** CLI autonomous development loop
**Researched:** 2026-02-01
**Confidence:** MEDIUM (based on official Ralph documentation and WebSearch findings)

## Executive Summary

This document analyzes the portableralph/ralph ecosystem to identify exact behavioral requirements for aligning rslph's basic mode with the reference implementation. Research focused on the original snarktank/ralph implementation which is the authoritative source for the "ralph loop" pattern.

## Reference Implementation: snarktank/ralph

**Source:** [snarktank/ralph](https://github.com/snarktank/ralph)

### Core Architecture

Ralph is an autonomous AI agent loop that:
1. Reads `prd.json` to find the next incomplete story (marked `passes: false`)
2. Invokes Claude Code with instructions from `prompt.md` + the story details
3. Claude implements the story and verifies acceptance criteria
4. Updates `prd.json` (marking `passes: true`) and commits changes
5. Logs learnings to `progress.txt` for context persistence
6. Continues until all stories pass or `<promise>COMPLETE</promise>` marker

### Key Files

| File | Purpose |
|------|---------|
| `ralph.sh` | Main orchestration bash script |
| `prompt.md` | Instructions Claude receives each iteration (for Amp) |
| `CLAUDE.md` | Instructions for Claude Code integration |
| `prd.json` | Task queue containing user stories with acceptance criteria |
| `progress.txt` | Persistent memory across context windows |

### Loop Structure (ralph.sh)

```bash
# Pseudocode reconstruction from research
while [ $iteration -le $max_iterations ]; do
    # Check for branch changes and archive previous runs

    # Run Claude with prompt
    claude --prompt "$(cat prompt.md)" "$story_details"

    # Check for completion signal
    if grep -q "<promise>COMPLETE</promise>" output; then
        exit 0
    fi

    # Update iteration counter
    iteration=$((iteration + 1))
done
```

Key behaviors:
- **Branch tracking**: Monitors branch changes via `.last-branch` file
- **Archiving**: Automatically archives previous runs when switching branches
- **Iteration limit**: Configurable max iterations (default 10)
- **Tool selection**: Supports `--tool amp` or `--tool claude` flags

### Termination Conditions

1. **`<promise>COMPLETE</promise>` signal**: Claude outputs this when all stories pass
2. **Max iterations reached**: Loop stops after configured limit
3. **All stories pass**: All entries in `prd.json` have `passes: true`

### Memory Persistence

Ralph maintains continuity through:
- **Git commits**: Code preservation across iterations
- **`progress.txt`**: Append-only learnings/patterns/gotchas
- **`prd.json` story notes**: Story-specific insights
- **CLAUDE.md updates**: Reusable patterns discovered during work

### prompt.md Structure (Reconstructed)

Based on research, the prompt instructs Claude to:

1. Read the PRD from `prd.json`
2. Check the progress log at `progress.txt`
3. Verify the correct git branch
4. Pick the **highest priority** user story where `passes: false`
5. Implement the story
6. Run quality checks (typecheck, lint, test) before committing
7. Keep changes focused and minimal
8. Follow existing code patterns
9. Update `progress.txt` with learnings
10. Mark story as complete in `prd.json`

Quality standards emphasized:
- All work must pass project quality checks before committing
- "Keep changes focused and minimal"
- Follow existing code patterns

Progress documentation format:
```
[date] [story-id]
- Implementation: [details]
- Learnings: [patterns, gotchas discovered]
```

### prd.json Format

```json
{
  "project": "MyApp",
  "branch": "ralph/task-priority",
  "goal": "Add priority levels to tasks",
  "stories": [
    {
      "id": "US-001",
      "title": "Add priority column",
      "description": "Add priority column to tasks table",
      "acceptance": [
        "Column accepts: 'high' | 'medium' | 'low'",
        "Default is 'medium'"
      ],
      "passes": false
    }
  ]
}
```

## Comparison: rslph Basic Mode vs portableralph

### Feature Alignment Matrix

| Feature | portableralph | rslph Basic | Aligned? | Notes |
|---------|---------------|-------------|----------|-------|
| **Plan file format** | `prd.json` (JSON) | `progress.md` (Markdown) | NO | Different format, similar purpose |
| **Progress tracking** | `progress.txt` (append-only) | `progress.md` sections | PARTIAL | rslph integrates in same file |
| **Termination signal** | `<promise>COMPLETE</promise>` | `RALPH_DONE` in Status | YES | Same concept, different marker |
| **Task identification** | `passes: false` | `[ ]` checkbox | YES | Equivalent semantics |
| **One task per iteration** | YES (highest priority story) | YES (first incomplete) | YES | Same behavior |
| **Quality checks** | Required before commit | Not enforced | NO | Missing in rslph |
| **Branch tracking** | `.last-branch` file | Not implemented | NO | Missing in rslph |
| **Iteration archiving** | On branch switch | Not implemented | NO | Missing in rslph |
| **Memory across iterations** | `progress.txt` learnings | Recent Attempts section | PARTIAL | Similar but different format |
| **VCS integration** | Git commit on story complete | Optional VCS commit | YES | rslph has this |
| **Max iterations** | Configurable (default 10) | Configurable | YES | Aligned |

### Prompt Content Comparison

#### Planning Prompt

| Aspect | portableralph | rslph Basic |
|--------|---------------|-------------|
| **Purpose** | Create prd.json from requirements | Create progress.md from requirements |
| **Story sizing** | "If you can't describe it in 2-3 sentences, split it" | "Each task completable in 1-2 iterations" |
| **Testing strategy** | Depends on project config | Explicit Testing Strategy section |
| **Acceptance criteria** | Binary-testable per story | Not structured |
| **Output format** | JSON | Markdown |

#### Build Prompt

| Aspect | portableralph | rslph Basic |
|--------|---------------|-------------|
| **One task rule** | YES | YES |
| **Quality checks before commit** | Required | Not enforced |
| **Failure memory** | `progress.txt` | Recent Attempts section |
| **Completion marker** | `<promise>COMPLETE</promise>` | `RALPH_DONE` |
| **Output format** | Updates prd.json + progress.txt | Complete progress.md |

### Critical Differences

#### 1. Progress File Format

**portableralph:** Uses separate files
- `prd.json` for task tracking
- `progress.txt` for learnings (append-only)
- CLAUDE.md for patterns

**rslph:** Single integrated file
- `progress.md` contains all sections
- Status, Analysis, Tasks, Testing Strategy, Recent Attempts, Iteration Log

**Impact:** rslph is more self-contained but may lose the "append-only learnings" property

#### 2. Quality Checks

**portableralph:** Explicitly requires quality checks pass before commit
```
All work must pass project quality checks (typecheck, lint, test) before committing
```

**rslph:** Does not enforce quality checks in basic mode

**Impact:** rslph may produce code that doesn't pass tests/lint

#### 3. Task Selection

**portableralph:** "Pick the **highest priority** user story"

**rslph:** "Find the FIRST incomplete task"

**Impact:** Functionally equivalent if tasks are ordered by priority (which they typically are)

#### 4. Completion Signal

**portableralph:** `<promise>COMPLETE</promise>` (XML-style)

**rslph:** `RALPH_DONE` in Status section

**Impact:** Compatible - both signal completion clearly

## Alignment Recommendations

### Must Align (Critical for ralph compatibility)

1. **RALPH_DONE semantics**: Already aligned - rslph uses `RALPH_DONE` in Status section
2. **One task per iteration**: Already aligned
3. **Task completion tracking**: Already aligned (checkbox semantics)
4. **Iteration loop behavior**: Already aligned (spawn fresh Claude each iteration)

### Should Align (Valuable additions)

1. **Quality check enforcement**: Add optional quality check step before marking task complete
2. **Learnings persistence**: Ensure Recent Attempts section is preserved across iterations (append-only behavior)
3. **Story sizing guidance**: Add "2-3 sentences" guidance to planning prompt

### Optional Alignment (Nice to have)

1. **Branch tracking**: Archive progress when switching branches
2. **Codebase patterns section**: Add CLAUDE.md-like pattern persistence
3. **Binary acceptance criteria**: Add acceptance criteria per task

## Verbatim Prompt Excerpts (Reconstructed)

Based on research, portableralph prompts emphasize:

### Planning Phase

> "If you can't describe it in 2-3 sentences, split it."

> Appropriate stories include adding database columns or single UI components, while oversized items like "Build entire dashboard" require decomposition.

### Build Phase

> Read the PRD from `prd.json`, check the progress log at `progress.txt`, verify the correct git branch, then "Pick the **highest priority** user story where `passes: false`" and implement it.

> All work must pass project quality checks (typecheck, lint, test) before committing. The agent should "Keep changes focused and minimal" and follow existing code patterns throughout.

> Updates to `progress.txt` must be appended (never replaced) with a specific format including date, story ID, thread URL, implementation details, and a learnings section capturing discovered patterns and gotchas.

### Completion Criteria

> Work continues one story per iteration until all stories have `passes: true`, at which point the agent should respond with `<promise>COMPLETE</promise>`.

## Implementation Guidance for rslph v1.3

### Basic Mode Prompt Updates

#### PROMPT_plan.md Changes

1. Add story sizing guidance:
   ```
   Each task should be describable in 2-3 sentences. If it requires more explanation, split it into smaller tasks.
   ```

2. Consider adding acceptance criteria per task (optional, may be over-engineering for basic mode)

#### PROMPT_build.md Changes

1. **No changes required for core alignment** - current rslph basic mode behavior already matches portableralph's core loop:
   - One task per iteration
   - RALPH_DONE on completion
   - Checkbox-based tracking

2. **Optional enhancement**: Add quality check reminder
   ```
   Before marking a task complete:
   - Verify implementation works
   - Run any relevant tests
   - Keep changes focused and minimal
   ```

### Loop Behavior Verification

The rslph build loop already implements:

- [x] Fresh Claude subprocess each iteration
- [x] Re-read progress file at iteration start
- [x] Check for RALPH_DONE before processing
- [x] Check for all tasks complete
- [x] Single task per iteration
- [x] Timeout handling with retry
- [x] Configurable max iterations
- [x] VCS commit on task completion

No changes needed to loop behavior for portableralph alignment.

### Potential Gaps

1. **Quality checks**: Not enforced - acceptable for basic mode
2. **Append-only learnings**: Recent Attempts may be trimmed - acceptable
3. **Branch tracking**: Not implemented - low priority

## Confidence Assessment

| Finding | Confidence | Source |
|---------|------------|--------|
| Loop termination on COMPLETE signal | HIGH | Official snarktank/ralph docs |
| prd.json format | HIGH | Official examples |
| One task per iteration | HIGH | Multiple sources agree |
| Quality check requirement | MEDIUM | Mentioned but not verified in code |
| Exact prompt.md content | LOW | Reconstructed from descriptions |
| progress.txt format | MEDIUM | Described but not seen verbatim |

## Sources

- [snarktank/ralph](https://github.com/snarktank/ralph) - Original reference implementation
- [LeslieCBarry/claude-ralph](https://github.com/LeslieCBarry/claude-ralph) - Claude Code adaptation
- [technofriends/ralph](https://github.com/technofriends/ralph) - Fork with documentation
- [Minimal autonomous agent prompt gist](https://gist.github.com/asidko/ee3c87cd787d398e3d92388c74abf5ec) - Alternative minimal pattern
- [aaron777collins/portableralph](https://github.com/aaron777collins/portableralph) - Portable version

## Conclusion

rslph basic mode is **substantially aligned** with portableralph core behavior. The key differences are in file format (markdown vs JSON) and some optional features (branch tracking, quality enforcement). No critical changes are needed for basic mode alignment - the current implementation correctly captures the essential ralph loop pattern:

1. Fresh subprocess per iteration
2. One task at a time
3. Progress file persistence
4. RALPH_DONE termination signal
5. Failure memory in Recent Attempts

The main opportunity for enhanced alignment is adding optional quality check enforcement, but this can be considered a v1.4 or "strict mode" feature rather than a v1.3 requirement.
