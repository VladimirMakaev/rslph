---
phase: quick-006
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/config.rs
  - src/planning/command.rs
autonomous: true

must_haves:
  truths:
    - "All tests pass with cargo test"
    - "RSLPH_CLAUDE_CMD env var works without conflict with Figment"
    - "Test isolation prevents mutex poisoning cascades"
  artifacts:
    - path: "src/config.rs"
      provides: "Config loading with filtered env vars"
      contains: "filter"
    - path: "src/planning/command.rs"
      provides: "Tests using claude_cmd field correctly"
      contains: "claude_cmd"
---

<objective>
Fix 5 failing tests in the rslph crate to make `cargo test` pass completely.

Purpose: Tests are failing due to two root causes:
1. Figment Env provider tries to parse RSLPH_CLAUDE_CMD but `claude_cmd` is marked `#[serde(skip)]`, causing extraction failure
2. One test sets deprecated `claude_path` field but code uses `claude_cmd.command`

Output: All tests passing on `cargo test`
</objective>

<execution_context>
@/Users/vmakaev/.claude/get-shit-done/workflows/execute-plan.md
@/Users/vmakaev/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@src/config.rs
@src/planning/command.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Filter claude_cmd from Figment Env provider</name>
  <files>src/config.rs</files>
  <action>
  In `src/config.rs`, modify the Env provider on lines 161 and 199 to filter out "claude_cmd" key.

  Currently (line 161):
  ```rust
  figment = figment.merge(Env::prefixed("RSLPH_").lowercase(true));
  ```

  Change to:
  ```rust
  figment = figment.merge(Env::prefixed("RSLPH_").lowercase(true).filter(|key| key != "claude_cmd"));
  ```

  Apply the same change on line 199 (in `load_with_overrides`).

  Reason: The `claude_cmd` field is marked `#[serde(skip)]` because we handle RSLPH_CLAUDE_CMD manually after extraction (parsing it into command + base_args). When Figment's Env provider sees RSLPH_CLAUDE_CMD, it tries to deserialize it into the `claude_cmd` field which fails. Filtering it out prevents this conflict.
  </action>
  <verify>cargo test config::tests --no-fail-fast 2>&1 | grep -E "(PASSED|FAILED|ok|FAILED)"</verify>
  <done>All 4 config tests pass: test_claude_cmd_env_override, test_cli_overrides_highest, test_env_override, test_load_missing_file_uses_defaults</done>
</task>

<task type="auto">
  <name>Task 2: Fix test_run_plan_command_nonexistent_command to use claude_cmd</name>
  <files>src/planning/command.rs</files>
  <action>
  In `src/planning/command.rs`, find the test `test_run_plan_command_nonexistent_command` (around line 829).

  Currently the test creates a Config with `claude_path`:
  ```rust
  let config = Config {
      claude_path: Some("/nonexistent/command".to_string()),
      ..Default::default()
  };
  ```

  Change it to set `claude_cmd` directly (import ClaudeCommand if needed):
  ```rust
  use crate::config::ClaudeCommand;
  // ...
  let config = Config {
      claude_cmd: ClaudeCommand {
          command: "/nonexistent/command".to_string(),
          base_args: vec![],
      },
      ..Default::default()
  };
  ```

  Reason: The code uses `config.claude_cmd.command` to spawn the subprocess, not `config.claude_path`. Setting `claude_path` via Default doesn't propagate to `claude_cmd` without going through the `Config::load` machinery.
  </action>
  <verify>cargo test planning::command::tests::test_run_plan_command_nonexistent_command --no-fail-fast 2>&1 | grep -E "(PASSED|FAILED|ok)"</verify>
  <done>test_run_plan_command_nonexistent_command passes</done>
</task>

<task type="auto">
  <name>Task 3: Verify all tests pass</name>
  <files></files>
  <action>
  Run the full test suite to confirm all tests pass.
  </action>
  <verify>cargo test 2>&1 | tail -20</verify>
  <done>cargo test reports all tests passing (0 failures)</done>
</task>

</tasks>

<verification>
- `cargo test` completes with 0 failures
- No mutex poisoning errors
- RSLPH_CLAUDE_CMD environment variable test passes
</verification>

<success_criteria>
- All tests in the crate pass
- No regressions introduced
- Changes are minimal and targeted
</success_criteria>

<output>
After completion, create `.planning/quick/006-make-all-tests-run-and-pass-on-cargo-tes/006-SUMMARY.md`
</output>
