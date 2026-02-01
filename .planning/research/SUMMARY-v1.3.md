# Research Summary: v1.3 Hardening

**Project:** rslph v1.3 "Hardening"
**Domain:** TUI consolidation, GSD personas, portableralph alignment
**Researched:** 2026-02-01
**Confidence:** HIGH

---

## Executive Summary

v1.3 Hardening focuses on consolidating existing features and introducing GSD-style multi-persona workflow. Research reveals that:

1. **TUI consolidation** is straightforward - use tui-textarea for proper multiline input, remove non-TUI code paths
2. **portableralph alignment** requires minimal changes - rslph basic mode is already well-aligned with core loop behavior
3. **GSD persona integration** requires extending the progress file schema to support persona selection and checkpoints
4. **gsd_tdd mode removal** is safe - replace with configurable persona selection

---

## Key Findings

### TUI Input (from TUI-INPUT.md)

**Recommended solution:** tui-textarea crate

| Feature | tui-textarea Provides |
|---------|----------------------|
| Multiline editing | ✓ Full multi-line with cursor tracking |
| Cursor navigation | ✓ Row/column positioning |
| Emacs keybindings | ✓ Ctrl+N/P/F/B, Ctrl+A/E, Ctrl+K |
| Word movement | ✓ Alt+F/B |
| Undo/redo | ✓ Ctrl+U/R |

**Compatibility note:** tui-textarea 0.7.0 supports ratatui 0.29; PR #119 adds 0.30 support (draft status). Check at implementation time.

**Integration approach:** Replace `input_buffer: String` with `textarea: Option<TextArea<'static>>` in PlanTuiState and App structs.

---

### portableralph Alignment (from PORTABLERALPH.md)

**Finding:** rslph basic mode is **already substantially aligned** with portableralph.

| Feature | portableralph | rslph Basic | Aligned? |
|---------|---------------|-------------|----------|
| Fresh context per iteration | ✓ | ✓ | YES |
| One task per iteration | ✓ | ✓ | YES |
| RALPH_DONE termination | `<promise>COMPLETE</promise>` | `RALPH_DONE` in Status | YES |
| Progress file persistence | progress.txt | progress.md | YES |
| VCS commit on task complete | ✓ | ✓ | YES |
| Quality checks before commit | Required | Not enforced | GAP (optional) |

**Recommendation:** No critical changes needed for basic mode alignment. Optional enhancement: add quality check reminder to build prompt.

---

### GSD Workflow (from GSD-WORKFLOW.md)

**9 Distinct Personas identified:**
1. **Orchestrator** - Coordination layer (stays lean)
2. **gsd-planner** - Creates PLAN.md files
3. **gsd-executor** - Implements code with deviation rules
4. **gsd-verifier** - Goal-backward verification
5. **gsd-debugger** - Root cause analysis
6. **gsd-phase-researcher** - Phase-specific research
7. **gsd-project-researcher** - Ecosystem survey
8. **gsd-plan-checker** - Validates plans before execution
9. **gsd-codebase-mapper** - Analyzes codebase structure

**Key patterns for Ralph Loop:**
- Goal-backward verification (task completion ≠ goal achievement)
- Deviation rules (auto-fix bugs, ask about architectural changes)
- Checkpoint protocol (human-verify, decision, human-action)
- Wave-based execution (dependency ordering)

---

### Integration Approach (from INTEGRATION-PITFALLS.md)

**Recommended: Extended Progress Schema**

```rust
pub struct ProgressFile {
    // Existing fields...

    // NEW: Persona selection for next iteration
    pub next_persona: Option<Persona>,

    // NEW: Compact decisions list
    pub decisions: Vec<CompactDecision>,

    // NEW: Checkpoint if paused
    pub checkpoint: Option<Checkpoint>,
}
```

**Why this approach:**
- Minimal architectural change to core Ralph Loop
- Backward compatible (old files work)
- Proven pattern (GSD's .continue-here.md is similar)
- Clear migration path

**Anti-patterns to avoid:**
1. Inline persona switching mid-iteration
2. Bloated progress file with 50+ fields
3. Stateful session resume across iterations
4. Implicit persona inheritance from phase name

---

## Requirement Implications

### TUI Consolidation (Priority: P0)

| REQ ID | Requirement |
|--------|-------------|
| TUI-01 | Remove all non-TUI code paths from plan, build, eval commands |
| TUI-02 | Add tui-textarea for multiline input with cursor |
| TUI-03 | TUI input tests with TestBackend + insta snapshots |

### Mode Consolidation (Priority: P0)

| REQ ID | Requirement |
|--------|-------------|
| MODE-01 | Remove gsd_tdd mode from codebase |
| MODE-02 | Verify basic mode matches portableralph behavior |

### GSD Personas (Priority: P1)

| REQ ID | Requirement |
|--------|-------------|
| PERS-01 | Add persona field to progress file schema |
| PERS-02 | Define executor, verifier, researcher, planner personas |
| PERS-03 | Select persona at iteration start based on progress file |
| PERS-04 | Add checkpoint result variant to iteration loop |
| PERS-05 | TUI display for checkpoint information |

### E2E Tests (Priority: P1)

| REQ ID | Requirement |
|--------|-------------|
| TEST-01 | Planning with 0 question rounds |
| TEST-02 | Planning with 1 question round |
| TEST-03 | Planning with 2 question rounds |
| TEST-04 | Planning with 3 question rounds |
| TEST-05 | TUI input prompt with cursor display |
| TEST-06 | Multi-iteration build with token tracking |

---

## Open Questions

1. **tui-textarea compatibility:** Will PR #119 be merged/released before implementation? If not, use git dependency or fork?
2. **Persona inference:** Should persona be explicit in progress file or inferred from task metadata?
3. **Checkpoint timeout:** What happens if user doesn't respond to checkpoint?

---

## Confidence Assessment

| Area | Confidence | Reason |
|------|------------|--------|
| TUI-only consolidation | HIGH | Clear code paths to remove |
| tui-textarea integration | MEDIUM | Version compatibility uncertainty |
| portableralph alignment | HIGH | Already substantially aligned |
| GSD persona mapping | HIGH | Direct source analysis |
| Integration approach | HIGH | Based on existing patterns |

---

## Sources

- `.planning/research/GSD-WORKFLOW.md` - GSD persona and workflow analysis
- `.planning/research/TUI-INPUT.md` - TUI multiline input patterns
- `.planning/research/PORTABLERALPH.md` - Reference implementation alignment
- `.planning/research/INTEGRATION-PITFALLS.md` - Integration challenges and recommendations
- [tui-textarea](https://github.com/rhysd/tui-textarea) - Recommended TUI input crate
- [snarktank/ralph](https://github.com/snarktank/ralph) - Original ralph implementation

---

*Research completed: 2026-02-01*
*Ready for requirements: yes*
