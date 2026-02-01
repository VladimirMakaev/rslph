# Requirements: v1.3 Hardening

**Project:** rslph
**Milestone:** v1.3 "Hardening"
**Created:** 2026-02-01
**Status:** Approved

---

## v1.3 Requirements

### TUI Consolidation (TUI)

| ID | Requirement | Priority |
|----|-------------|----------|
| TUI-01 | TUI-only mode for all commands — Remove all non-TUI code paths from plan, build, and eval commands | Must |
| TUI-02 | Multiline text input with cursor — Use tui-textarea for proper cursor navigation, word movement, and undo/redo | Must |
| TUI-03 | Full dialog display — Show complete Claude conversation in TUI including streaming output, questions, and user responses | Must |
| TUI-04 | Input widget at bottom — Following claude_dialog.py pattern, input docked at bottom with conversation history above | Must |
| TUI-05 | Streaming text handling — Buffer text until newlines, display complete lines as they arrive (port claude_dialog.py pattern) | Must |

### Mode Consolidation (MODE)

| ID | Requirement | Priority |
|----|-------------|----------|
| MODE-01 | Remove gsd_tdd mode — Delete gsd_tdd mode entirely from codebase (prompts, CLI args, tests) | Must |
| MODE-02 | Basic mode prompts match portableralph — Use exact prompt structure from snarktank/ralph reference implementation | Must |
| MODE-03 | Basic mode loop behavior identical — Fresh context per iteration, one task per iteration, RALPH_DONE termination | Must |
| MODE-04 | Basic mode progress file format — Match portableralph progress.txt structure for task tracking and learnings | Must |
| MODE-05 | Basic mode commit behavior — Commit after each completed task (matching ralph.sh) | Must |

### E2E Test Coverage (TEST)

| ID | Requirement | Priority |
|----|-------------|----------|
| TEST-01 | Planning with 0 question rounds — E2E test where Claude produces plan without asking questions | Must |
| TEST-02 | Planning with 1 question round — E2E test where Claude asks 1 round of clarifying questions | Must |
| TEST-03 | Planning with 2 question rounds — E2E test where Claude asks 2 rounds of questions | Must |
| TEST-04 | Planning with 3 question rounds — E2E test where Claude asks 3 rounds of questions | Must |
| TEST-05 | TUI input prompt with cursor — Snapshot test showing input widget with visible cursor | Must |
| TEST-06 | Multi-iteration build with tokens — E2E test verifying token accumulation across iterations | Must |
| TEST-07 | Tests use ratatui-testlib and fake claude — All TUI tests use established TestBackend + insta pattern | Must |

### GSD Hardening — Ralph Loop Integration (GSD)

**Goal:** Translate GSD's multi-persona workflow to Ralph Loop's progress file model. Each GSD pattern must serialize to progress.md and work with fresh-context-per-iteration.

| ID | Requirement | Priority |
|----|-------------|----------|
| GSD-01 | Progress file persona field — Add `## Next Persona` section (executor/verifier/researcher/planner) that determines system prompt for next iteration | Must |
| GSD-02 | Persona prompt library — Define system prompts for each persona with distinct behaviors and constraints | Must |
| GSD-03 | Executor persona — Standard build execution with deviation rules (auto-fix bugs, ask about architecture changes) | Must |
| GSD-04 | Verifier persona — Goal-backward verification (check truths, artifacts, wiring) after all tasks complete | Must |
| GSD-05 | Checkpoint section in progress — Add `## Checkpoint` section with type (human-verify/decision/human-action), awaiting message, resume task | Should |
| GSD-06 | Checkpoint iteration result — Add Checkpoint variant to IterationResult enum that pauses loop for user input | Should |
| GSD-07 | TUI checkpoint display — Show checkpoint information and await user response via input widget | Should |
| GSD-08 | Compact decisions in progress — Add decisions to Recent Attempts with format: "Decision: [what] — [why]" | Should |
| GSD-09 | Persona auto-transition — After all tasks complete, set next_persona to "verifier" for goal-backward check | Should |

**Progress File Schema (Extended):**
```markdown
# Progress: [Name]

## Status
In Progress

## Next Persona
executor

## Tasks
### Phase 1
- [x] Task 1
- [ ] Task 2

## Recent Attempts
### Iteration 3
- Tried: Implemented auth
- Result: Success
- Decision: Used jose for JWT — Better TS types

## Checkpoint
Type: human-verify
Awaiting: Verify login flow works
Resume: Task 3
```

**Persona Behaviors:**
- **Executor**: One task per iteration, deviation rules, auto-fix bugs
- **Verifier**: Derives must_haves from goal, checks truths/artifacts/wiring, identifies gaps
- **Researcher**: Survey ecosystem, produce findings (planning phase only)
- **Planner**: Decompose goals into tasks (planning phase only)

---

## Future Requirements (deferred)

| ID | Requirement | Rationale |
|----|-------------|-----------|
| FUT-01 | Quality check enforcement before commit | Nice-to-have for strict mode |
| FUT-02 | Branch tracking/archiving | Low priority portableralph feature |
| FUT-03 | Full deviation tracking | Can add in v1.4 with more GSD patterns |
| FUT-04 | Authentication gate handling | Needed for external service integration |

---

## Out of Scope

| Item | Rationale |
|------|-----------|
| Parallel plan execution | Ralph Loop is inherently sequential |
| Subagent spawning via Task tool | GSD-specific, not compatible with Ralph Loop |
| Real-time collaboration | Single-user CLI tool |
| GUI application | CLI/TUI only |

---

## Requirement Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| MODE-01 | Phase 16 (Cleanup) | Pending |
| TUI-01 | Phase 16 (Cleanup) | Pending |
| MODE-02 | Phase 17 (Basic Mode) | Pending |
| MODE-03 | Phase 17 (Basic Mode) | Pending |
| MODE-04 | Phase 17 (Basic Mode) | Pending |
| MODE-05 | Phase 17 (Basic Mode) | Pending |
| TUI-02 | Phase 18 (TUI Enhancement) | Pending |
| TUI-03 | Phase 18 (TUI Enhancement) | Pending |
| TUI-04 | Phase 18 (TUI Enhancement) | Pending |
| TUI-05 | Phase 18 (TUI Enhancement) | Pending |
| GSD-01 | Phase 19 (GSD Personas) | Pending |
| GSD-02 | Phase 19 (GSD Personas) | Pending |
| GSD-03 | Phase 19 (GSD Personas) | Pending |
| GSD-04 | Phase 19 (GSD Personas) | Pending |
| GSD-05 | Phase 19 (GSD Personas) | Pending |
| GSD-06 | Phase 19 (GSD Personas) | Pending |
| GSD-07 | Phase 19 (GSD Personas) | Pending |
| GSD-08 | Phase 19 (GSD Personas) | Pending |
| GSD-09 | Phase 19 (GSD Personas) | Pending |
| TEST-01 | Phase 20 (E2E Tests) | Pending |
| TEST-02 | Phase 20 (E2E Tests) | Pending |
| TEST-03 | Phase 20 (E2E Tests) | Pending |
| TEST-04 | Phase 20 (E2E Tests) | Pending |
| TEST-05 | Phase 20 (E2E Tests) | Pending |
| TEST-06 | Phase 20 (E2E Tests) | Pending |
| TEST-07 | Phase 20 (E2E Tests) | Pending |

**Phase Order (deletions first):**
1. **Phase 16: Cleanup** — Remove gsd_tdd mode + Remove non-TUI code paths
2. **Phase 17: Basic Mode Alignment** — Make --basic identical to portableralph
3. **Phase 18: TUI Enhancement** — Add tui-textarea, dialog display, streaming
4. **Phase 19: GSD Personas** — Persona prompts, progress schema, checkpoints, verification
5. **Phase 20: E2E Tests** — Comprehensive test coverage

**Coverage:**
- v1.3 requirements: 26 total
- Mapped to phases: 26
- Unmapped: 0 ✓

---

## Research Sources

- `.planning/research/TUI-INPUT.md` — tui-textarea integration patterns
- `.planning/research/PORTABLERALPH.md` — Basic mode alignment reference
- `.planning/research/GSD-WORKFLOW.md` — Persona definitions and checkpoint patterns
- `.planning/research/INTEGRATION-PITFALLS.md` — Anti-patterns and recommended approach
- `claude_dialog.py` — Reference Python implementation for TUI/conversation flow

---

*26 requirements | 4 categories | Roadmap ready*
