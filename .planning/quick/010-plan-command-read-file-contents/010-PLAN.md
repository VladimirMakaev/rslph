---
phase: quick-010
plan: 01
type: execute
wave: 1
depends_on: []
files_modified: [src/main.rs]
autonomous: true

must_haves:
  truths:
    - "rslph plan INITIAL.md reads file contents when INITIAL.md exists"
    - "rslph plan 'Build a todo app' treats argument as literal text"
    - "Error message shown when file path exists but cannot be read"
  artifacts:
    - path: "src/main.rs"
      provides: "File detection and content reading for plan command"
      contains: "std::fs::read_to_string"
  key_links:
    - from: "src/main.rs"
      to: "run_plan_command"
      via: "file contents passed instead of path string"
      pattern: "read_to_string.*plan"
---

<objective>
Make the plan command read file contents when the argument is a file path.

Purpose: Users expect `rslph plan INITIAL.md` to read the file contents, not treat "INITIAL.md" as literal text.
Output: Plan command that correctly detects file paths and reads their contents.
</objective>

<execution_context>
@/Users/vmakaev/.claude/get-shit-done/workflows/execute-plan.md
@/Users/vmakaev/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@src/main.rs
@src/cli.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add file detection and reading to plan command</name>
  <files>src/main.rs</files>
  <action>
In the Plan command match arm (around line 19-65), before calling run_plan_command:

1. Import std::path::Path at the top of the file (add to existing use statements)

2. Add logic to detect if `plan` is a file path and read contents:
```rust
// Resolve plan input: read from file if exists, otherwise use as literal text
let plan_input = {
    let path = Path::new(&plan);
    if path.exists() && path.is_file() {
        match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(e) => {
                eprintln!("Error reading plan file '{}': {}", plan, e);
                std::process::exit(1);
            }
        }
    } else {
        plan.clone()
    }
};
```

3. Update the println! to show actual source:
```rust
if Path::new(&plan).exists() {
    println!("Planning from file: {}", plan);
} else {
    println!("Planning: {}", plan);
}
```

4. Pass `&plan_input` to run_plan_command instead of `&plan`

Note: Use Path::new() to check if file exists - this handles both absolute and relative paths correctly. The `.is_file()` check ensures we don't try to read directories.
  </action>
  <verify>
Run: `cargo build`
Run: `cargo test`
  </verify>
  <done>
Plan command compiles and existing tests pass.
  </done>
</task>

<task type="auto">
  <name>Task 2: Add unit tests for file path detection</name>
  <files>src/main.rs</files>
  <action>
Since main.rs doesn't have a test module and the logic is inline in main(), we'll verify the behavior through integration testing approach:

1. Create a simple test by running the command with --help to ensure it still works:
```bash
cargo run -- plan --help
```

2. The real verification is manual testing (which we'll do in verify step):
- Create a temp file with plan content
- Run `rslph plan <tempfile>` and verify it reads contents
- Run `rslph plan "inline text"` and verify it uses literal text

Note: The actual file reading logic is simple enough that compile-time verification + manual testing is sufficient. Adding unit tests would require refactoring the main function, which is out of scope for this quick fix.
  </action>
  <verify>
Run: `cargo run -- plan --help` (should show help without errors)
Run: `echo "Test plan content" > /tmp/test-plan.txt && cargo run -- plan /tmp/test-plan.txt --no-tui 2>&1 | head -5` (should show "Planning from file: /tmp/test-plan.txt")
Run: `cargo run -- plan "Build something inline" --no-tui 2>&1 | head -5` (should show "Planning: Build something inline")
  </verify>
  <done>
Both file path and inline text modes work correctly.
  </done>
</task>

</tasks>

<verification>
1. `cargo build` - compiles without errors
2. `cargo test` - all existing tests pass
3. Manual test with file path shows "Planning from file: X"
4. Manual test with inline text shows "Planning: X"
</verification>

<success_criteria>
- Plan command reads file contents when argument is an existing file path
- Plan command uses literal text when argument is not a file path
- Error handling for unreadable files (permission denied, etc.)
- No breaking changes to existing functionality
</success_criteria>

<output>
After completion, create `.planning/quick/010-plan-command-read-file-contents/010-SUMMARY.md`
</output>
