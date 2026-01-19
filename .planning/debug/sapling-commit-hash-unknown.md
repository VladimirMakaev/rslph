---
status: diagnosed
trigger: "Investigate why the VCS commit hash shows 'unknown' instead of the actual hash"
created: 2026-01-18T23:26:00Z
updated: 2026-01-18T23:26:00Z
---

## Current Focus

hypothesis: CONFIRMED - sl commit produces no stdout output, parsing fails
test: Made test commit with sl, observed output
expecting: Hash in stdout, but got empty output
next_action: Return diagnosis

## Symptoms

expected: Log shows `[VCS] Committed: abc123 (Sapling)` with actual hash
actual: Log shows `[VCS] Committed: unknown (Sapling)`
errors: No error, just wrong value
reproduction: Any commit via SaplingVcs::commit()
started: Since VCS integration was implemented

## Eliminated

(none - root cause found on first hypothesis)

## Evidence

- timestamp: 2026-01-18T23:25:30Z
  checked: src/vcs/sapling.rs commit() implementation
  found: |
    Code assumes hash is on first line of stdout:
    ```rust
    let hash = stdout
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().last())
        .unwrap_or("unknown")
        .to_string();
    ```
  implication: If stdout is empty, falls back to "unknown"

- timestamp: 2026-01-18T23:25:51Z
  checked: Actual sl commit output
  found: |
    `sl commit -m "message"` produces NO OUTPUT to stdout on success.
    The command succeeds (exit code 0) but stdout is empty.
    Tested with: `sl commit -m "test commit" 2>&1` - only saw addremove output, not commit output.
  implication: The parsing logic cannot work because there's no output to parse

- timestamp: 2026-01-18T23:26:00Z
  checked: How to get commit hash after sl commit
  found: |
    After commit, must query the hash separately using:
    - `sl log -l 1 --template '{node}'` returns full 40-char hash
    - `sl log -l 1 --template '{node|short}'` returns short hash
  implication: Need to run separate command after commit to get hash

## Resolution

root_cause: Sapling's `sl commit` command produces no stdout output on success, unlike Git which prints the commit hash. The code assumes the hash is in stdout and falls back to "unknown" when stdout is empty.

fix: After successful commit, run `sl log -l 1 --template '{node|short}'` to retrieve the commit hash of the newly created commit.

verification: (not performed - diagnosis only mode)

files_changed: []
