---
phase: quick-005
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/config.rs
  - src/cli.rs
  - src/subprocess/runner.rs
  - src/build/iteration.rs
  - src/build/command.rs
  - src/build/state.rs
  - src/planning/command.rs
  - src/eval/command.rs
  - src/eval/parallel.rs
  - src/main.rs
autonomous: true
user_setup: []

must_haves:
  truths:
    - "RSLPH_CLAUDE_CMD env var can specify full claude command with args (e.g., 'claude --internet')"
    - "--no-dsp CLI flag appends --dangerously-skip-permissions to all Claude invocations"
    - "Both features work together and are applied everywhere Claude is spawned"
  artifacts:
    - path: "src/config.rs"
      provides: "ClaudeCommand struct with base_command and base_args parsed from RSLPH_CLAUDE_CMD"
    - path: "src/cli.rs"
      provides: "--no-dsp global flag"
    - path: "src/subprocess/runner.rs"
      provides: "spawn_claude helper that builds full args with dsp flag support"
  key_links:
    - from: "src/config.rs"
      to: "RSLPH_CLAUDE_CMD env"
      via: "parse_claude_cmd function"
      pattern: "RSLPH_CLAUDE_CMD"
    - from: "src/build/iteration.rs"
      to: "ClaudeRunner::spawn"
      via: "spawn_claude helper"
      pattern: "spawn_claude"
---

<objective>
Add RSLPH_CLAUDE_CMD env var support and --no-dsp CLI flag.

Purpose: Allow users to specify a full claude command with arguments via env var (e.g., "claude --internet"), and add a --no-dsp flag that appends "--dangerously-skip-permissions" to all Claude invocations for automated/CI usage.

Output: Updated config parsing, CLI flag, and Claude invocation sites.
</objective>

<context>
@.planning/STATE.md
@src/config.rs
@src/cli.rs
@src/subprocess/runner.rs
@src/build/iteration.rs
@src/planning/command.rs
@src/eval/command.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add ClaudeCommand parsing and --no-dsp flag</name>
  <files>
    src/config.rs
    src/cli.rs
    src/subprocess/runner.rs
    src/subprocess/mod.rs
  </files>
  <action>
1. In `src/config.rs`:
   - Add `ClaudeCommand` struct with `command: String` and `base_args: Vec<String>` fields
   - Add `claude_cmd: ClaudeCommand` field to Config (replaces `claude_path: String`)
   - Create `parse_claude_cmd(raw: &str) -> ClaudeCommand` function that:
     - Splits the string by whitespace (use shell-words crate if available, else simple split)
     - First element is the command, rest are base_args
     - Applies resolve_command_path to the command
   - In Config::load and load_with_overrides:
     - Check for RSLPH_CLAUDE_CMD env var first
     - If set, parse it into ClaudeCommand
     - If not set, fall back to claude_path config (for backward compatibility)
   - Update Default impl: claude_cmd defaults to ClaudeCommand { command: "claude".to_string(), base_args: vec![] }
   - Add tests for parse_claude_cmd with various inputs

2. In `src/cli.rs`:
   - Add global `--no-dsp` flag: `#[arg(long, global = true)] pub no_dsp: bool`
   - The flag means "no dangerous skip permissions" - when set, appends `--dangerously-skip-permissions` to Claude

3. In `src/subprocess/runner.rs` or a new helper module:
   - Create `build_claude_args(base_args: &[String], additional_args: &[String], no_dsp: bool) -> Vec<String>` function:
     - Starts with base_args.to_vec()
     - If no_dsp is true, push "--dangerously-skip-permissions"
     - Extends with additional_args
     - Returns the combined args
   - Export this from subprocess module

4. Update tests in config.rs for the new behavior
  </action>
  <verify>
    cargo build --lib
    cargo test config::tests
    cargo test cli::tests
  </verify>
  <done>
    - ClaudeCommand struct exists with command and base_args
    - parse_claude_cmd correctly splits "claude --internet" into command="claude" and base_args=["--internet"]
    - --no-dsp flag is parseable in CLI
    - build_claude_args helper exists and correctly combines args
  </done>
</task>

<task type="auto">
  <name>Task 2: Update all Claude spawn sites to use new config and flag</name>
  <files>
    src/build/iteration.rs
    src/build/command.rs
    src/build/state.rs
    src/planning/command.rs
    src/eval/command.rs
    src/eval/parallel.rs
    src/main.rs
  </files>
  <action>
1. Thread `no_dsp: bool` through the call chain:
   - In `src/main.rs`: Read cli.no_dsp, pass to run_plan_command, run_build_command, run_eval_command
   - Add no_dsp parameter to BuildContext in src/build/state.rs
   - Add no_dsp parameter to run_build_command in src/build/command.rs
   - Add no_dsp parameter to run_plan_command in src/planning/command.rs
   - Add no_dsp parameter to run_eval_command and run_single_trial in src/eval/command.rs
   - Add no_dsp to parallel trial execution in src/eval/parallel.rs

2. Update all ClaudeRunner::spawn calls:

   In `src/build/iteration.rs` (run_single_iteration):
   - Change: `ClaudeRunner::spawn(&ctx.config.claude_path, &args, working_dir)`
   - To: Use ctx.config.claude_cmd.command and build_claude_args(&ctx.config.claude_cmd.base_args, &args, ctx.no_dsp)

   In `src/planning/command.rs` (run_basic_planning, run_tui_planning, run_adaptive_planning, run_claude_headless, generate_project_name):
   - Change all ClaudeRunner::spawn calls similarly
   - Thread no_dsp through function parameters

   In `src/eval/command.rs` (discover_run_script):
   - Change ClaudeRunner::spawn call similarly
   - Note: This is used for test discovery, should also respect no_dsp

3. Ensure backward compatibility:
   - If RSLPH_CLAUDE_CMD is not set, behavior remains unchanged
   - If --no-dsp is not passed, no extra args are added
  </action>
  <verify>
    cargo build
    cargo test
  </verify>
  <done>
    - All ClaudeRunner::spawn calls use config.claude_cmd.command
    - All calls build args using build_claude_args with no_dsp
    - no_dsp is threaded through all command handlers
    - Tests pass
  </done>
</task>

<task type="auto">
  <name>Task 3: Add integration tests</name>
  <files>
    src/config.rs
    tests/e2e/mod.rs (or inline tests)
  </files>
  <action>
1. Add unit tests in src/config.rs:
   - test_parse_claude_cmd_simple: "claude" -> command="claude", base_args=[]
   - test_parse_claude_cmd_with_args: "claude --internet" -> command="claude", base_args=["--internet"]
   - test_parse_claude_cmd_multiple_args: "claude --internet --verbose" -> command="claude", base_args=["--internet", "--verbose"]
   - test_parse_claude_cmd_absolute_path: "/usr/bin/claude --flag" -> command="/usr/bin/claude", base_args=["--flag"]
   - test_claude_cmd_env_override: Set RSLPH_CLAUDE_CMD, verify Config loads it

2. Add unit tests for build_claude_args:
   - test_build_args_no_dsp_false: base=["--internet"], additional=["--verbose"], no_dsp=false -> ["--internet", "--verbose"]
   - test_build_args_no_dsp_true: base=["--internet"], additional=["--verbose"], no_dsp=true -> ["--internet", "--dangerously-skip-permissions", "--verbose"]
   - test_build_args_empty_base: base=[], additional=["--verbose"], no_dsp=true -> ["--dangerously-skip-permissions", "--verbose"]

3. Add CLI parse test:
   - test_parse_no_dsp_flag: Verify `rslph --no-dsp plan idea.txt` parses correctly

4. Verify all existing tests still pass
  </action>
  <verify>
    cargo test
    cargo clippy -- -D warnings
  </verify>
  <done>
    - All new tests pass
    - All existing tests pass
    - No clippy warnings
    - cargo build succeeds
  </done>
</task>

</tasks>

<verification>
cargo build
cargo test
cargo clippy -- -D warnings

Manual verification:
- RSLPH_CLAUDE_CMD="echo test" rslph plan "hello" --no-tui (should use echo as command)
- rslph --no-dsp plan "hello" --no-tui (should work, though echo won't understand --dangerously-skip-permissions)
</verification>

<success_criteria>
- RSLPH_CLAUDE_CMD env var correctly parsed into command + base args
- --no-dsp flag correctly appends --dangerously-skip-permissions
- All Claude spawn sites use the new helper
- Backward compatible when env var not set
- All tests pass, no clippy warnings
</success_criteria>

<output>
After completion, create `.planning/quick/005-add-rslph-claude-cmd-env-and-no-dsp-flag/005-SUMMARY.md`
</output>
