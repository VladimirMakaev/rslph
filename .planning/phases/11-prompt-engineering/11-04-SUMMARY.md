---
phase: 11-prompt-engineering
plan: 04
subsystem: prompts
tags: [prompt-mode, cli, config]

dependency-graph:
  requires: [11-01, 11-02, 11-03]
  provides: [mode-selection, cli-mode-flag, basic-prompts]
  affects: [11-05]

tech-stack:
  added: []
  patterns: [mode-based-prompt-loading, file-override-precedence]

key-files:
  created:
    - prompts/basic/PROMPT_plan.md
    - prompts/basic/PROMPT_build.md
  modified:
    - src/prompts/defaults.rs
    - src/prompts/loader.rs
    - src/cli.rs

decisions:
  - id: basic-mode-content
    choice: "Use current rslph prompts (not PortableRalph)"
    rationale: "Backward compatibility - users can use file overrides for PortableRalph"
  - id: mode-file-precedence
    choice: "File overrides > mode selection"
    rationale: "Power users with custom prompts should not be affected by mode flag"

metrics:
  duration: 3m 32s
  completed: 2026-01-21
---

# Phase 11 Plan 04: Basic Mode and Mode Selection Summary

Mode-aware prompt infrastructure with basic mode prompts and CLI --mode flag

## What Was Built

### Basic Mode Prompts
Created `prompts/basic/` directory with exact copies of current rslph prompts:
- `PROMPT_plan.md` (158 lines) - Planning Assistant prompt
- `PROMPT_build.md` (112 lines) - Build Agent prompt with RALPH_DONE

### Mode-Aware defaults.rs
Replaced old prompt loading with mode-based selection:
```rust
impl PromptMode {
    pub fn plan_prompt(&self) -> &'static str {
        match self {
            PromptMode::Basic => BASIC_PLAN,
            PromptMode::Gsd => GSD_PLAN,
            PromptMode::GsdTdd => GSD_TDD_PLAN,
        }
    }
}
```

### Mode-Aware loader.rs
Updated loader to use config.prompt_mode:
```rust
pub fn get_plan_prompt(config: &Config) -> color_eyre::Result<String> {
    if let Some(path) = &config.plan_prompt {
        return std::fs::read_to_string(path)...;  // File override wins
    }
    Ok(config.prompt_mode.plan_prompt().to_string())  // Mode selection
}
```

### CLI --mode Flag
Added global flag for mode selection:
```
--mode <MODE>  Prompt mode selection (basic, gsd, gsd_tdd)
```

## Commits

| Hash | Description |
|------|-------------|
| 7c6af41 | feat(11-04): add basic mode prompts |
| afb180c | feat(11-04): add mode-aware prompt methods to defaults.rs |
| d0a0f10 | feat(11-04): update loader.rs to use mode selection |
| e45e448 | feat(11-04): add --mode CLI flag for prompt mode selection |

## Test Coverage

- defaults.rs: 5 tests (basic, gsd, gsd_tdd, discovery)
- loader.rs: 7 tests (mode default, mode switching, file override precedence)
- cli.rs: 2 new tests (mode flag parsing, all mode values)

## Mode Selection Precedence

```
CLI --mode > env RSLPH_PROMPT_MODE > config prompt_mode > default (basic)
```

File overrides (plan_prompt, build_prompt) always take precedence over mode.

## Deviations from Plan

None - plan executed exactly as written.

## Next Phase Readiness

Phase 11-05 (Final Integration) can proceed:
- All three prompt modes now have working prompts
- Mode selection works via CLI and config
- File overrides maintain backward compatibility
- 30+ tests verify the prompt system
