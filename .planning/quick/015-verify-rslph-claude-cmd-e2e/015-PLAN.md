---
phase: quick
plan: 015
type: execute
wave: 1
depends_on: []
files_modified:
  - tests/fake_claude_lib/scenario.rs
  - tests/e2e/test_rslph_integration.rs
autonomous: true

must_haves:
  truths:
    - "Fake Claude scenarios use RSLPH_CLAUDE_CMD env var by default"
    - "Build command E2E tests pass with RSLPH_CLAUDE_CMD"
    - "Plan command has E2E test coverage with fake Claude"
  artifacts:
    - path: "tests/fake_claude_lib/scenario.rs"
      provides: "Updated env_vars() returning RSLPH_CLAUDE_CMD"
      contains: "RSLPH_CLAUDE_CMD"
    - path: "tests/e2e/test_rslph_integration.rs"
      provides: "Plan command E2E tests"
      contains: "test_rslph_plan"
  key_links:
    - from: "tests/e2e/test_rslph_integration.rs"
      to: "tests/fake_claude_lib/scenario.rs"
      via: "FakeClaudeHandle.env_vars()"
      pattern: "scenario\\.env_vars\\(\\)"
---

<objective>
Verify and test with E2E that RSLPH_CLAUDE_CMD is used in both build and plan commands.

Purpose: The fake Claude test infrastructure currently uses the deprecated RSLPH_CLAUDE_PATH env var. The config system now supports RSLPH_CLAUDE_CMD which allows passing command with arguments. Need to update the test infrastructure and add plan command E2E coverage.

Output: Updated scenario builder using RSLPH_CLAUDE_CMD and new plan command E2E tests.
</objective>

<execution_context>
@/Users/vmakaev/.claude/get-shit-done/workflows/execute-plan.md
@/Users/vmakaev/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@src/config.rs (RSLPH_CLAUDE_CMD handling)
@tests/fake_claude_lib/scenario.rs (FakeClaudeHandle.env_vars)
@tests/e2e/test_rslph_integration.rs (existing build E2E tests)
</context>

<tasks>

<task type="auto">
  <name>Task 1: Update FakeClaudeHandle.env_vars() to use RSLPH_CLAUDE_CMD</name>
  <files>tests/fake_claude_lib/scenario.rs</files>
  <action>
Update the `env_vars()` method in `FakeClaudeHandle` to return `RSLPH_CLAUDE_CMD` instead of `RSLPH_CLAUDE_PATH`:

1. Change the second tuple from `("RSLPH_CLAUDE_PATH", ...)` to `("RSLPH_CLAUDE_CMD", ...)`
2. Update the docstring to reference `RSLPH_CLAUDE_CMD` instead of `RSLPH_CLAUDE_PATH`
3. Keep `FAKE_CLAUDE_CONFIG` unchanged

This aligns the test infrastructure with the new config system that prefers RSLPH_CLAUDE_CMD.
  </action>
  <verify>
`cargo build --tests` compiles successfully.
Existing E2E tests in test_rslph_integration.rs still pass.
  </verify>
  <done>
FakeClaudeHandle.env_vars() returns RSLPH_CLAUDE_CMD tuple.
All existing E2E tests pass (they use rslph_with_fake_claude helper which calls env_vars()).
  </done>
</task>

<task type="auto">
  <name>Task 2: Add E2E tests for plan command with fake Claude</name>
  <files>tests/e2e/test_rslph_integration.rs</files>
  <action>
Add new E2E tests for the plan command using the fake Claude infrastructure:

1. `test_rslph_plan_single_response` - Plan command runs and invokes fake Claude:
   - Create scenario with text response
   - Create workspace with PROGRESS.md
   - Run `rslph plan PROGRESS.md --no-tui`
   - Verify fake Claude was invoked exactly once

2. `test_rslph_plan_uses_rslph_claude_cmd_env` - Explicitly verify RSLPH_CLAUDE_CMD works for plan:
   - Create scenario
   - Create workspace without any config
   - Set RSLPH_CLAUDE_CMD env var explicitly (like test_rslph_uses_rslph_claude_path_env does for build)
   - Run plan command
   - Verify invocation count is 1

Use the same pattern as existing build tests:
- Use `rslph_with_fake_claude()` helper
- Use `WorkspaceBuilder` for test workspaces
- Assert on `scenario.invocation_count()`
  </action>
  <verify>
`cargo test test_rslph_plan` runs both new tests successfully.
Tests verify fake Claude invocation for plan command.
  </verify>
  <done>
Two new E2E tests exist for plan command.
Both tests pass, proving RSLPH_CLAUDE_CMD works for plan command.
  </done>
</task>

<task type="auto">
  <name>Task 3: Verify all E2E tests pass</name>
  <files>tests/e2e/test_rslph_integration.rs</files>
  <action>
Run the full E2E test suite to ensure:

1. All existing build E2E tests still pass (they now use RSLPH_CLAUDE_CMD internally)
2. New plan E2E tests pass
3. The test_rslph_uses_rslph_claude_path_env test still passes (backward compatibility via config.rs)

Run: `cargo test --test e2e test_rslph`

If any tests fail, diagnose and fix the issue (likely in the env var handling).
  </action>
  <verify>
`cargo test --test e2e test_rslph` shows all tests passing.
  </verify>
  <done>
All E2E integration tests pass.
Both build and plan commands work with RSLPH_CLAUDE_CMD.
Backward compatibility with RSLPH_CLAUDE_PATH is maintained.
  </done>
</task>

</tasks>

<verification>
- `cargo test --test e2e test_rslph` passes all tests
- `cargo test fake_claude` passes (scenario tests)
- grep confirms RSLPH_CLAUDE_CMD in env_vars()
</verification>

<success_criteria>
- FakeClaudeHandle.env_vars() returns RSLPH_CLAUDE_CMD (not RSLPH_CLAUDE_PATH)
- At least 2 new E2E tests for plan command exist
- All existing build E2E tests continue to pass
- RSLPH_CLAUDE_PATH backward compat test passes
</success_criteria>

<output>
After completion, create `.planning/quick/015-verify-rslph-claude-cmd-e2e/015-SUMMARY.md`
</output>
