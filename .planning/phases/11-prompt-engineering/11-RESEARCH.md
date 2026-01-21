# Phase 11: Prompt Engineering - Research

**Researched:** 2026-01-21
**Domain:** LLM Prompt Architecture, Autonomous Agent Patterns, TDD Integration
**Confidence:** HIGH

## Summary

This research synthesizes patterns from GSD (Get Shit Done), current rslph prompt architecture, PortableRalph prompts, and LLM prompt engineering best practices to design a multi-mode prompt system. The goal is enabling users to select coherent prompt pairs (plan + build) that apply different autonomous agent philosophies.

The existing rslph architecture already supports prompt loading with file-based overrides. The enhancement adds a `prompt_mode` configuration that selects from baked-in prompt sets, where each mode consists of paired plan and build prompts designed to work together. This avoids the fragile mix-and-match of incompatible prompts.

GSD patterns provide rich source material for the `gsd` and `gsd-tdd` modes, particularly around deviation handling, verification patterns, and TDD tracking. The key insight is that GSD's subagent-spawning model differs from rslph's iteration loop model, so patterns must be adapted rather than copied directly.

**Primary recommendation:** Implement prompt modes as an enum with compile-time baked prompts, using trait-based prompt providers for extensibility. Mode selection via config file (`prompt_mode = "basic"`) with CLI override (`--mode=gsd`).

## Standard Stack

The established libraries/tools for this domain:

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust `include_str!` | built-in | Compile-time prompt embedding | Zero runtime cost, current pattern |
| serde | 1.x | Mode enum serialization | Already in use for config |
| figment | existing | Config layering | Already handles precedence |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| strum | 0.26 | Enum string conversion | Clean mode string parsing |
| strum_macros | 0.26 | Derive macros for strum | `EnumString`, `Display` derives |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Enum-based modes | Trait objects | Trait objects add complexity for 3 fixed modes |
| Baked-in prompts | All file-based | File-based loses compile-time guarantees |
| strum | Manual FromStr | strum reduces boilerplate for enum parsing |

**Installation:**
```bash
cargo add strum strum_macros
```

## Architecture Patterns

### Recommended Project Structure

```
src/prompts/
├── mod.rs           # Public exports
├── defaults.rs      # Baked-in prompts (expanded for modes)
├── loader.rs        # Mode-aware loading logic
└── modes.rs         # PromptMode enum definition

prompts/
├── basic/
│   ├── PROMPT_plan.md
│   └── PROMPT_build.md
├── gsd/
│   ├── PROMPT_plan.md
│   └── PROMPT_build.md
└── gsd_tdd/
    ├── PROMPT_plan.md
    └── PROMPT_build.md
```

### Pattern 1: Mode Enum with Compile-Time Prompts

**What:** Define prompt modes as an enum with associated prompt pairs embedded at compile time.

**When to use:** Always - this is the core pattern for the feature.

**Example:**
```rust
// Source: Synthesized from rslph patterns and strum documentation

use strum::{Display, EnumString};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, EnumString, Display)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PromptMode {
    #[default]
    Basic,
    Gsd,
    GsdTdd,
}

impl PromptMode {
    pub fn plan_prompt(&self) -> &'static str {
        match self {
            PromptMode::Basic => include_str!("../../prompts/basic/PROMPT_plan.md"),
            PromptMode::Gsd => include_str!("../../prompts/gsd/PROMPT_plan.md"),
            PromptMode::GsdTdd => include_str!("../../prompts/gsd_tdd/PROMPT_plan.md"),
        }
    }

    pub fn build_prompt(&self) -> &'static str {
        match self {
            PromptMode::Basic => include_str!("../../prompts/basic/PROMPT_build.md"),
            PromptMode::Gsd => include_str!("../../prompts/gsd/PROMPT_build.md"),
            PromptMode::GsdTdd => include_str!("../../prompts/gsd_tdd/PROMPT_build.md"),
        }
    }
}
```

### Pattern 2: Config with Mode Field

**What:** Add `prompt_mode` field to Config, overridable by CLI.

**When to use:** Config loading, mode selection.

**Example:**
```rust
// Source: Extending existing rslph config.rs pattern

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    // ... existing fields ...

    /// Prompt mode selection (basic, gsd, gsd_tdd)
    pub prompt_mode: PromptMode,

    // File overrides still work - they take precedence over mode
    pub plan_prompt: Option<PathBuf>,
    pub build_prompt: Option<PathBuf>,
}
```

### Pattern 3: Precedence Chain for Prompts

**What:** Clear precedence for prompt selection: CLI override file > config override file > mode prompt > default.

**When to use:** Prompt loading functions.

**Example:**
```rust
// Source: Extending existing loader.rs pattern

pub fn get_plan_prompt(config: &Config) -> color_eyre::Result<String> {
    // 1. File override takes precedence (power users)
    if let Some(path) = &config.plan_prompt {
        return std::fs::read_to_string(path).map_err(|e| {
            eyre!("Failed to read plan prompt from '{}': {}", path.display(), e)
        });
    }

    // 2. Mode-based selection
    Ok(config.prompt_mode.plan_prompt().to_string())
}
```

### Pattern 4: GSD TDD Iteration Tracking

**What:** Track TDD attempt count in progress file for escape hatch logic.

**When to use:** `gsd-tdd` mode build prompt, iteration state management.

**Example:**
```markdown
## TDD State

current_feature: "User login validation"
tdd_phase: red  # red | green | refactor
consecutive_failures: 0
escaped: false
escape_reason: null
```

### Anti-Patterns to Avoid

- **Separate Mode Selection per Command:** Don't allow `--plan-mode=basic --build-mode=gsd`. Modes are coherent pairs designed to work together.

- **Runtime Mode Switching:** Don't change modes mid-build. Mode is locked at build start from config/CLI.

- **Dynamic Prompt Generation:** Don't build prompts from fragments at runtime. Use complete, tested prompt files.

- **Ignoring File Overrides:** Don't remove existing `plan_prompt`/`build_prompt` override capability. Power users need escape hatch.

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Enum string parsing | Manual `match` on strings | `strum::EnumString` derive | Handles case variations, error messages |
| Default mode selection | Hardcoded fallback logic | `#[default]` derive on enum | Single source of truth |
| TDD test type heuristics | Custom detection logic | Project stack detection (existing) | Already have stack info from plan |
| Deviation handling rules | Ad-hoc if/else trees | Structured deviation rules in prompt | GSD pattern proven in practice |

**Key insight:** The prompt text itself should encode behavior rules, not Rust code. Rust code handles mode selection and loading; prompt text handles agent behavior.

## Common Pitfalls

### Pitfall 1: Mixing Incompatible Prompt Pairs

**What goes wrong:** User selects GSD plan prompt but uses basic build prompt. Agent creates GSD-style plans with checkpoints but build agent doesn't understand checkpoint semantics.

**Why it happens:** Desire for flexibility without understanding interdependencies.

**How to avoid:** Mode selects both prompts atomically. No mix-and-match API.

**Warning signs:** Checkpoint instructions in plan but no checkpoint handling in build output.

### Pitfall 2: TDD Escape Hatch Without State

**What goes wrong:** Agent escapes TDD after 3 failures, but next iteration has no memory of this decision, re-attempts test-first.

**Why it happens:** Progress file doesn't persist TDD state.

**How to avoid:** Add `## TDD State` section to progress file for gsd-tdd mode. Parse and update on each iteration.

**Warning signs:** Agent flipping between test-first and implementation-first across iterations.

### Pitfall 3: Overly Complex Deviation Handling

**What goes wrong:** Build prompt has 20 deviation rules with complex priority ordering. Agent spends tokens reasoning about which rule applies.

**Why it happens:** Over-engineering from GSD's comprehensive rule set.

**How to avoid:** GSD has 4 deviation rules. Adopt that constraint. More rules don't improve behavior.

**Warning signs:** Agent output contains extensive deviation-rule reasoning before action.

### Pitfall 4: Ignoring rslph's Iteration Model

**What goes wrong:** GSD prompts reference subagent spawning, fresh context, or multi-plan parallel execution. rslph runs single-task iterations.

**Why it happens:** Copy-paste from GSD without adaptation.

**How to avoid:** Translate GSD patterns to iteration model:
- GSD "fresh context" = rslph "iteration boundary"
- GSD "spawn subagent" = rslph "complete task, move to next"
- GSD "parallel plans" = rslph "sequential phases"

**Warning signs:** Prompt contains `Task()` spawning syntax or references to subagent patterns.

### Pitfall 5: Basic Mode Drift

**What goes wrong:** "Improvements" to basic mode prompts make them no longer match PortableRalph.

**Why it happens:** Well-meaning cleanup during implementation.

**How to avoid:** Basic mode is a FAITHFUL reproduction. Document as "intentionally exact copy, do not modify."

**Warning signs:** Basic mode prompts differ from current `PROMPT_plan.md` and `PROMPT_build.md`.

## Code Examples

Verified patterns from GSD and existing codebase:

### GSD Deviation Rules (Adapted for rslph)

```markdown
## Deviation Rules

Four rules govern unplanned work. Apply WITHOUT asking:

1. **Auto-Fix Bugs:** Bugs in code you just wrote → fix immediately, document in Recent Attempts
2. **Add Critical Functionality:** Missing piece blocking task completion → add it, note in Completed This Iteration
3. **Fix Blocking Issues:** Environment/config issues blocking progress → fix, document
4. **Ask About Architecture:** Changes affecting other phases/tasks → stop, document, await next iteration guidance

Order: Try 1-3 first. Only reach 4 for cross-cutting changes.
```

### TDD Phase Tracking

```markdown
## TDD State

feature: "Login endpoint"
phase: red
test_file: "tests/auth/login_test.rs"
impl_file: "src/auth/login.rs"
attempts: 1
max_attempts: 3
escaped: false

### TDD Flow

**RED** (current):
- Write failing test for expected behavior
- Run test to confirm it fails
- Move to GREEN

**GREEN**:
- Implement minimum code to pass test
- Run test to confirm it passes
- Move to REFACTOR

**REFACTOR**:
- Clean up code while tests pass
- Commit: "refactor(XX-YY): clean up [feature]"
- Move to next feature or RED
```

### Test Type Heuristics

```markdown
## Testing Strategy Selection

Based on project stack detection:

| Stack Indicator | Prefer | Rationale |
|-----------------|--------|-----------|
| Web framework (axum, actix) | Integration tests | HTTP endpoints need full stack |
| CLI application | E2E tests | User-facing commands |
| Library crate | Unit tests | API contract testing |
| Database models | Integration tests | Need real DB behavior |
| Pure functions | Unit tests | No external dependencies |
| UI components | Snapshot/E2E tests | Visual regression matters |

**Default:** Unit tests for internal modules, integration tests for public API.
```

### Verification Pattern from GSD

```markdown
## Verification Levels

Before marking task complete, verify at appropriate level:

1. **Exists:** File/function/endpoint is present (minimum)
2. **Substantive:** Contains real implementation (not TODO/placeholder)
3. **Wired:** Connected to calling code (imported, routed, invoked)
4. **Functional:** Actually works when exercised (tests pass, manual check)

**Required level by task type:**
- Create/Add tasks: Level 3 (Wired)
- Implement tasks: Level 4 (Functional)
- Configure tasks: Level 2 (Substantive)
- Write test tasks: Level 4 (Functional - test must run)
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Single prompt for all uses | Mode-based prompt selection | 2025+ | Specialized agent behavior |
| Hardcoded TDD rules | Configurable TDD modes | 2025 | Flexibility for different projects |
| Post-hoc testing phase | Interleaved testing | 2024 | Better test coverage |
| Freestyle deviation | Structured deviation rules | GSD 2024 | Predictable agent behavior |

**Deprecated/outdated:**
- Single "Testing Phase" at end: Replaced by interleaved testing
- Unstructured agent freedom: Replaced by deviation rules

## Open Questions

Resolved from CONTEXT.md:

### 1. Which GSD patterns apply to rslph's loop model?

**Answer (HIGH confidence):**

Applicable:
- **Deviation Rules:** 4-rule system translates directly
- **Verification Patterns:** 4-level verification applicable
- **TDD RED-GREEN-REFACTOR:** Maps to iteration boundaries
- **One task per iteration:** Already rslph's model

Not applicable (require adaptation):
- **Subagent spawning:** rslph uses iteration loop, not subagents
- **Fresh context management:** rslph iterations have continuity via progress file
- **Parallel plan execution:** rslph runs plans sequentially

### 2. What heuristics for unit test vs e2e test selection?

**Answer (MEDIUM confidence):**

Use project stack indicators:
- CLI app → E2E tests (test user commands)
- Web API → Integration tests (test HTTP layer)
- Library → Unit tests (test public API)
- Pure functions → Unit tests (no external deps)
- UI → E2E/snapshot tests (visual verification)

With `--adaptive` flag, ask: "What testing approach does this project prefer?" and include answer in progress file.

### 3. How to track TDD attempt state in progress file?

**Answer (HIGH confidence):**

Add `## TDD State` section for `gsd-tdd` mode:
```markdown
## TDD State

feature: "Current feature being TDD'd"
phase: red|green|refactor
attempts: 0
max_attempts: 3
escaped: false
escape_reason: null
```

Agent updates this section each iteration. After 3 failed attempts at same phase, set `escaped: true`, document reason, proceed without test-first.

### 4. What format for test strategy info in progress file?

**Answer (HIGH confidence):**

Use existing `## Testing Strategy` section, enhanced:
```markdown
## Testing Strategy

framework: cargo test
mode: tdd|standard
test_types:
  - unit: src modules
  - integration: API endpoints
  - e2e: CLI commands
coverage_target: 80%

### Test Files Created

- tests/auth/login_test.rs: Login validation
- tests/auth/logout_test.rs: Session termination
```

## Sources

### Primary (HIGH confidence)

- `~/.claude/get-shit-done/references/tdd.md` - TDD patterns, RED-GREEN-REFACTOR flow
- `~/.claude/get-shit-done/references/verification-patterns.md` - 4-level verification
- `~/.claude/get-shit-done/workflows/execute-plan.md` - Deviation rules, task execution
- `/Users/vmakaev/NonWork/rslph/src/prompts/` - Current prompt architecture
- `/Users/vmakaev/NonWork/rslph/src/config.rs` - Config with figment layering

### Secondary (MEDIUM confidence)

- [Lakera Prompt Engineering Guide](https://www.lakera.ai/blog/prompt-engineering-guide) - Production prompt patterns
- `~/.claude/get-shit-done/templates/state.md` - State tracking patterns
- `~/.claude/get-shit-done/templates/continue-here.md` - Session continuity

### Tertiary (LOW confidence)

- Web search for TDD heuristics - general principles, not LLM-specific

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - leveraging existing Rust patterns
- Architecture: HIGH - extending proven rslph patterns
- GSD adaptation: HIGH - direct source access
- TDD heuristics: MEDIUM - general software principles applied to LLM context
- Pitfalls: HIGH - derived from GSD documentation and rslph experience

**Research date:** 2026-01-21
**Valid until:** 2026-02-21 (30 days - stable domain)
