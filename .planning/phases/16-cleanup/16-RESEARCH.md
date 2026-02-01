# Phase 16: Cleanup - Research

**Researched:** 2026-02-01
**Domain:** Code deletion (gsd_tdd mode, non-TUI code paths)
**Confidence:** HIGH (direct codebase investigation)

## Summary

This research maps all locations of deprecated code that must be removed in Phase 16. The codebase has two primary removal targets:

1. **gsd_tdd mode** - A prompt mode enum variant with associated prompts, CLI args, tests, and documentation
2. **Non-TUI code paths** - The `--no-tui` flag and all conditional logic that branches on TUI/non-TUI

The research reveals that gsd_tdd mode is well-isolated (single enum variant with match arms), but non-TUI removal is more complex because it's used extensively in E2E tests for headless testing. The E2E tests will need migration to TUI-aware testing or complete rewrites.

**Primary recommendation:** Remove gsd_tdd mode first (simpler, isolated changes), then tackle non-TUI removal which requires E2E test strategy decisions.

## Deletion Inventory: gsd_tdd Mode

### Summary Statistics
- **Source files affected:** 5 files
- **Test files affected:** 2 files (modes.rs, defaults.rs)
- **Prompt files to delete:** 2 files (directory)
- **Documentation files:** 1 file (README.md)
- **Planning docs:** References only (no changes needed)

### Source Code Locations

| File | Line(s) | What to Remove | Deletion Order |
|------|---------|----------------|----------------|
| `src/prompts/modes.rs` | 35 | `GsdTdd` enum variant | 2nd (after match arms) |
| `src/prompts/modes.rs` | 59-62 | `gsd_tdd` parse test | 4th |
| `src/prompts/modes.rs` | 69 | `GsdTdd.to_string()` test | 4th |
| `src/prompts/modes.rs` | 74-78 | serde roundtrip test using GsdTdd | 4th |
| `src/prompts/defaults.rs` | 14-15 | `GSD_TDD_PLAN`, `GSD_TDD_BUILD` constants | 1st |
| `src/prompts/defaults.rs` | 26 | `PromptMode::GsdTdd => GSD_TDD_PLAN` match arm | 1st |
| `src/prompts/defaults.rs` | 35 | `PromptMode::GsdTdd => GSD_TDD_BUILD` match arm | 1st |
| `src/prompts/defaults.rs` | 74-81 | `test_gsd_tdd_prompts_exist` test | 4th |
| `src/prompts/loader.rs` | 113-114 | Test using `PromptMode::GsdTdd` | 4th |
| `src/prompts/loader.rs` | 127 | Test using `PromptMode::GsdTdd` | 4th |
| `src/cli.rs` | 25 | Help text mentioning `gsd_tdd` | 3rd |
| `src/cli.rs` | 81 | Modes comment mentioning `gsd_tdd` | 3rd |
| `src/cli.rs` | 396, 414 | Test with `gsd_tdd` mode | 4th |
| `src/cli.rs` | 472, 475 | Test with `gsd_tdd` mode | 4th |
| `src/cli.rs` | 484 | Test vector with `gsd_tdd` | 4th |
| `src/config.rs` | 109 | Comment mentioning `gsd_tdd` | 3rd |
| `src/eval/parallel.rs` | 269 | Test using `PromptMode::GsdTdd` | 4th |
| `src/eval/command.rs` | 532 | Comment mentioning `gsd_tdd` | 3rd |
| `src/planning/command.rs` | 33 | Doc comment mentioning GsdTdd | 3rd |
| `src/build/state.rs` | 97 | Comment mentioning GsdTdd | 3rd |

### Prompt Files to Delete

```
prompts/gsd_tdd/
├── PROMPT_plan.md
└── PROMPT_build.md
```

**Action:** Delete entire `prompts/gsd_tdd/` directory.

### Documentation Updates

| File | Lines | What to Change |
|------|-------|----------------|
| `README.md` | 96 | Remove `gsd_tdd` from `--mode` description |
| `README.md` | 114 | Remove `--mode gsd_tdd` example |
| `README.md` | 132 | Remove `gsd_tdd` from modes list |
| `README.md` | 173, 195, 239, 266 | Remove `gsd_tdd` references |
| `README.md` | 299-313 | Delete entire gsd_tdd section |
| `README.md` | 389 | Remove from eval example |

### Recommended Deletion Order (gsd_tdd)

1. **Remove match arms in defaults.rs** - Remove GsdTdd branches from `plan_prompt()` and `build_prompt()`
2. **Remove enum variant in modes.rs** - Delete `GsdTdd` from `PromptMode` enum
3. **Update comments/docs** - Remove mentions from cli.rs, config.rs, command.rs, build/state.rs
4. **Remove tests** - Delete all tests that reference GsdTdd
5. **Delete prompt files** - Remove `prompts/gsd_tdd/` directory
6. **Update README.md** - Remove all gsd_tdd documentation

## Deletion Inventory: Non-TUI Code Paths

### Summary Statistics
- **Source files with `no_tui` param:** 5 files
- **Source files with `use_tui` logic:** 3 files
- **CLI definitions:** 3 commands (plan, build, eval)
- **E2E tests using `--no-tui`:** 4 test files, 30+ test cases

### CLI Definitions to Remove

| File | Lines | Command | What to Remove |
|------|-------|---------|----------------|
| `src/cli.rs` | 48-50 | Plan | `no_tui: bool` field + `--no-tui` arg |
| `src/cli.rs` | 66-68 | Build | `no_tui: bool` field + `--no-tui` arg |
| `src/cli.rs` | 89-91 | Eval | `no_tui: bool` field + `--no-tui` arg |

### Main.rs Conditional Logic

| Lines | What to Change |
|-------|----------------|
| 43, 76-78, 83 | Remove `no_tui` from Plan destructure, remove conditional print |
| 110, 116-127, 135 | Remove `no_tui` from Build destructure, simplify `use_tui` logic |
| 161, 196-198 | Remove `no_tui` from Eval destructure, remove conditional print |

### Command Handlers

| File | Function | Lines | Change |
|------|----------|-------|--------|
| `src/build/command.rs` | `run_build_command` | 49, 60-69, 91-101 | Remove `no_tui` param, always use TUI |
| `src/planning/command.rs` | `run_plan_command` | 47, 56-68 | Remove `tui` param, always use TUI |
| `src/eval/command.rs` | `run_eval_command` | 56, 69, 91, 147-173 | Remove `no_tui` param, always use TUI |
| `src/eval/command.rs` | `run_parallel_eval_mode` | 140-173 | Remove `no_tui` param and conditional |
| `src/eval/command.rs` | `run_single_trial` | 377, 469 | Remove `_no_tui` param |
| `src/eval/parallel.rs` | `run_parallel_evals` | 71, 85, 121 | Remove `no_tui` param |
| `src/eval/parallel.rs` | `run_trial` | 154, 187 | Remove `no_tui` param |

### E2E Tests Using `--no-tui`

**Critical:** These tests CANNOT run in TUI mode because they use `assert_cmd::Command` which doesn't support interactive terminals.

| File | Test Count | Tests Using --no-tui |
|------|------------|----------------------|
| `tests/e2e/test_rslph_integration.rs` | 11 tests | All use `--no-tui` |
| `tests/e2e/test_eval_integration.rs` | 7 tests | All use `--no-tui` |
| `tests/e2e/test_interactive_planning.rs` | 3 tests | All use `--no-tui` |
| `tests/e2e/test_token_tracking.rs` | 1 test | Uses `--no-tui` |

**Test files:**
```
tests/e2e/test_rslph_integration.rs:59, 152, 206, 236, 268, 379, 407, 443, 493, 554, 624
tests/e2e/test_eval_integration.rs:38, 89, 158, 210, 287, 351, 393
tests/e2e/test_interactive_planning.rs:38, 93, 236
tests/e2e/test_token_tracking.rs:119
```

### Build Command Unit Tests

These tests in `src/build/command.rs` pass `no_tui: true`:
```
Lines: 626, 657, 711, 787, 837, 885, 1000, 1102, 1175, 1240, 1341
```

## Architecture Patterns

### Pattern 1: TUI-Only Command Structure

After cleanup, all commands should always launch TUI:

```rust
// BEFORE: Conditional TUI
let use_tui = config.tui_enabled && !no_tui && !dry_run;
if use_tui {
    run_with_tui(...).await
} else {
    run_headless(...).await
}

// AFTER: TUI-only (no branching)
// config.tui_enabled still respected for users who want to disable
if config.tui_enabled && !dry_run {
    run_with_tui(...).await
} else {
    // dry_run mode is text-only by design
    run_dry_run(...).await
}
```

### Pattern 2: Prompt Mode Simplification

After removing GsdTdd:

```rust
// BEFORE
pub enum PromptMode {
    Basic,
    Gsd,
    GsdTdd,  // Remove
}

// AFTER
pub enum PromptMode {
    Basic,
    Gsd,
}
```

Match arms reduce from 3 to 2 in all `plan_prompt()` and `build_prompt()` implementations.

## Common Pitfalls

### Pitfall 1: E2E Test Breakage
**What goes wrong:** Removing `--no-tui` breaks all E2E tests because `assert_cmd` cannot handle TUI mode.
**Why it happens:** E2E tests use `Command::output()` which requires stdout capture, but TUI writes to raw terminal.
**How to avoid:**
- Option A: Keep a hidden env var `RSLPH_FORCE_HEADLESS=1` for testing only
- Option B: Rewrite E2E tests to use `pty` crate for terminal simulation
- Option C: Convert E2E tests to integration tests that test modules directly
**Warning signs:** E2E tests timing out or producing empty output.

### Pitfall 2: Match Arm Exhaustiveness
**What goes wrong:** Removing `GsdTdd` variant causes compiler errors in match statements.
**Why it happens:** Rust requires exhaustive matching.
**How to avoid:** Remove all match arms for `GsdTdd` BEFORE removing the enum variant.
**Warning signs:** "non-exhaustive patterns" compiler error.

### Pitfall 3: include_str! Compilation Error
**What goes wrong:** `include_str!("../../prompts/gsd_tdd/PROMPT_plan.md")` fails at compile time.
**Why it happens:** File doesn't exist during compilation.
**How to avoid:** Remove `const GSD_TDD_*` declarations BEFORE deleting prompt files.
**Warning signs:** "couldn't read file" compilation error.

### Pitfall 4: Config File Compatibility
**What goes wrong:** Users with `prompt_mode = "gsd_tdd"` in config.toml get parse errors.
**Why it happens:** Enum variant no longer exists.
**How to avoid:**
- Document breaking change in release notes
- Consider adding migration logic that converts `gsd_tdd` to `gsd` with a warning
**Warning signs:** "unknown variant `gsd_tdd`" errors on startup.

## Deletion Order Summary

### Phase 16-01: Remove gsd_tdd Mode

1. Remove match arms in `src/prompts/defaults.rs`
2. Remove `GsdTdd` variant from `src/prompts/modes.rs`
3. Update help text in `src/cli.rs`
4. Update doc comments in `src/config.rs`, `src/planning/command.rs`, `src/build/state.rs`
5. Remove tests referencing GsdTdd in modes.rs, defaults.rs, loader.rs, cli.rs, parallel.rs
6. Delete `prompts/gsd_tdd/` directory
7. Update `README.md` to remove all gsd_tdd documentation

### Phase 16-02: Remove Non-TUI Code Paths

**Blocker:** Requires E2E test strategy decision first.

Options:
1. **Keep internal headless mode for testing** - Add `RSLPH_FORCE_HEADLESS` env var
2. **Rewrite E2E tests** - Use `pty` crate or convert to integration tests
3. **Skip E2E tests** - Accept reduced test coverage

After decision:
1. Remove `--no-tui` from CLI (plan, build, eval)
2. Remove `no_tui` parameters from command handlers
3. Simplify `use_tui` conditional logic
4. Update/remove E2E tests as per strategy
5. Remove `no_tui` from unit tests in command.rs

## Open Questions

1. **E2E Test Strategy** — RESOLVED
   - What we know: `assert_cmd` cannot handle TUI, 22+ tests affected
   - Decision: **Restructure tests to use library directly** (like tui_tests.rs pattern)
   - Tests will call command functions (run_build_command, run_plan_command) with TestBackend
   - No hidden env vars or flags needed

2. **Config Migration**
   - What we know: Users may have `prompt_mode = "gsd_tdd"` in config
   - What's unclear: How many users actually use this
   - Recommendation: Silent fallback to `gsd` with one-time warning message

3. **dry_run Behavior**
   - What we know: `dry_run` mode bypasses TUI in build command
   - What's unclear: Should dry_run also always use TUI?
   - Recommendation: Keep dry_run as text-only (it's preview output, not interactive)

## Sources

### Primary (HIGH confidence)
- Direct codebase investigation via Grep/Read tools
- `src/prompts/modes.rs` - enum definition
- `src/prompts/defaults.rs` - prompt loading
- `src/cli.rs` - CLI definitions
- `src/main.rs` - command dispatch
- `src/build/command.rs` - build handler
- `src/eval/command.rs` - eval handler
- `tests/e2e/*.rs` - E2E tests

### File Counts (verified)
- gsd_tdd in source: 5 files
- no_tui in source: 8 files
- gsd_tdd in tests: 3 files
- no_tui in tests: 4 files

## Metadata

**Confidence breakdown:**
- gsd_tdd locations: HIGH - exhaustive grep search
- Non-TUI locations: HIGH - exhaustive grep search
- Deletion order: HIGH - based on Rust compilation requirements
- E2E test impact: HIGH - verified test file contents

**Research date:** 2026-02-01
**Valid until:** N/A (codebase-specific, does not expire)
