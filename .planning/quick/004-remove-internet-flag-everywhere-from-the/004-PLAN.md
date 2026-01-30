---
phase: quick-004
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/build/iteration.rs
  - src/planning/command.rs
  - src/eval/command.rs
  - README.md
  - .planning/STATE.md
  - .planning/PROJECT.md
autonomous: true

must_haves:
  truths:
    - "No --internet flag appears in Claude CLI invocations"
    - "No TODO comments about removing --internet flag remain"
    - "Documentation no longer mentions the workaround"
    - "Pending todo CLAUDE-INTERNET-FLAG is removed from STATE.md"
  artifacts:
    - path: "src/build/iteration.rs"
      provides: "Build iteration without --internet flag"
      contains: "args = vec!"
    - path: "src/planning/command.rs"
      provides: "Planning commands without --internet flag"
      contains: "args = vec!"
    - path: "src/eval/command.rs"
      provides: "Eval command without --internet flag"
      contains: "args = vec!"
  key_links: []
---

<objective>
Remove the `--internet` workaround flag from all Claude CLI invocations in the codebase.

Purpose: The underlying Claude CLI hanging issue has been resolved, so the workaround is no longer needed.
Output: Clean codebase without workaround flags and associated TODO comments.
</objective>

<execution_context>
@/Users/vmakaev/.claude/get-shit-done/workflows/execute-plan.md
@/Users/vmakaev/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
</context>

<tasks>

<task type="auto">
  <name>Task 1: Remove --internet flag from source files</name>
  <files>
    src/build/iteration.rs
    src/planning/command.rs
    src/eval/command.rs
  </files>
  <action>
Remove the `--internet` flag and associated TODO/WORKAROUND comments from all Claude CLI argument vectors:

1. **src/build/iteration.rs** (line ~128-130):
   - Remove the TODO comment on line 128
   - Remove `"--internet".to_string(),` line with its trailing WORKAROUND comment

2. **src/planning/command.rs** (4 locations):
   - Lines ~90-92: Remove TODO and --internet line
   - Lines ~217-218: Remove --internet line (no TODO here)
   - Lines ~491-493: Remove TODO and --internet line
   - Lines ~573-575: Remove TODO and --internet line

3. **src/eval/command.rs** (line ~1361):
   - Remove `"--internet".to_string(),` line

After removal, the args vectors should start with `"-p".to_string()` as the first element.
  </action>
  <verify>
    `grep -r "\-\-internet" src/` returns no matches
    `cargo build` succeeds
    `cargo test` passes
  </verify>
  <done>No --internet flag or related TODO/WORKAROUND comments remain in source files</done>
</task>

<task type="auto">
  <name>Task 2: Update documentation and project state</name>
  <files>
    README.md
    .planning/STATE.md
    .planning/PROJECT.md
  </files>
  <action>
1. **README.md** (lines ~418-421):
   - Remove the bullet point about rslph using --internet flag internally
   - The troubleshooting section should still mention "If the Claude CLI hangs" but remove the workaround reference
   - Keep the other troubleshooting bullets (ensure Claude CLI up to date, check authentication)

2. **.planning/STATE.md** (line ~192):
   - Remove the CLAUDE-INTERNET-FLAG pending todo item entirely

3. **.planning/PROJECT.md** (line ~104):
   - Remove the CLAUDE-INTERNET-FLAG bullet from Known Issues/Workarounds section
  </action>
  <verify>
    `grep -r "CLAUDE-INTERNET-FLAG" .planning/` returns no matches
    `grep -r "\-\-internet" README.md` returns no matches
  </verify>
  <done>All documentation updated, pending todo removed from STATE.md</done>
</task>

</tasks>

<verification>
- `grep -r "\-\-internet" src/` returns no matches
- `grep -r "CLAUDE-INTERNET-FLAG" .` returns no matches (excluding planning history files)
- `cargo build` succeeds
- `cargo test` passes
</verification>

<success_criteria>
1. Zero occurrences of `--internet` flag in source files
2. Zero occurrences of CLAUDE-INTERNET-FLAG todo in active planning files
3. README troubleshooting section updated
4. All tests pass, code compiles
</success_criteria>

<output>
After completion, create `.planning/quick/004-remove-internet-flag-everywhere-from-the/004-SUMMARY.md`
</output>
