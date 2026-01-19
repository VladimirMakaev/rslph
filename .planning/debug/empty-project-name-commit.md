---
status: diagnosed
trigger: "Investigate why the VCS commit message has an empty project name"
created: 2026-01-18T12:00:00Z
updated: 2026-01-18T12:00:00Z
---

## Current Focus

hypothesis: Commit message uses parsed response name instead of original progress file name
test: Trace data flow from progress file to commit message
expecting: Found that updated_progress.name comes from Claude response, not original file
next_action: Report root cause

## Symptoms

expected: Commit message shows `[project-name][iter 1] Completed 1 task(s)`
actual: Commit message shows `[][iter 1] Completed 1 task(s)`
errors: None - no error, just empty project name
reproduction: Run build with VCS enabled, observe commit message after task completion
started: Since VCS integration was added

## Eliminated

(none - root cause found on first hypothesis)

## Evidence

- timestamp: 2026-01-18T12:00:00Z
  checked: src/build/iteration.rs line 200
  found: `format_iteration_commit(&updated_progress.name, ...)` uses parsed response name
  implication: If Claude omits project name in output, commit will have empty name

- timestamp: 2026-01-18T12:00:00Z
  checked: src/build/iteration.rs lines 166-180
  found: `updated_progress = ProgressFile::parse(&response_text)` - name comes from Claude's output
  implication: Claude controls the name field, not the original progress file

- timestamp: 2026-01-18T12:00:00Z
  checked: src/progress.rs lines 136-142
  found: Name parsed from H1 heading `# Progress: {name}` - if Claude outputs `# Progress:` without name, field is empty
  implication: Parsing logic correctly handles empty case, but caller uses wrong source

- timestamp: 2026-01-18T12:00:00Z
  checked: prompts/PROMPT_build.md lines 30, 34, 65
  found: Prompt says "Start with `# Progress:`" and example shows `# Progress: Example Task`
  implication: Claude may interpret `[Name]` as optional placeholder and omit actual name

- timestamp: 2026-01-18T12:00:00Z
  checked: src/build/iteration.rs line 42
  found: `ctx.progress = ProgressFile::load(&ctx.progress_path)?` - original progress has correct name
  implication: ctx.progress.name is available and has the correct project name

## Resolution

root_cause: Commit message uses `updated_progress.name` (parsed from Claude's response) instead of `ctx.progress.name` (original progress file), causing empty name when Claude omits the project name in its H1 heading output.

fix: Change line 200 in iteration.rs from `&updated_progress.name` to `&ctx.progress.name`

verification: (not yet applied)

files_changed: []
