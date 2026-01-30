---
phase: quick
plan: 003
type: execute
wave: 1
depends_on: []
files_modified:
  - README.md
autonomous: true

must_haves:
  truths:
    - "README shows CI status badge with live status from GitHub Actions"
    - "README shows crates.io version badge with current version"
    - "Release workflow publishes to crates.io on version tags"
  artifacts:
    - path: "README.md"
      provides: "Status badges at top of file"
      contains: "shields.io"
  key_links:
    - from: "README.md CI badge"
      to: ".github/workflows/ci.yml"
      via: "GitHub Actions badge URL"
    - from: "README.md crates.io badge"
      to: "crates.io/crates/rslph"
      via: "shields.io crates.io badge"
---

<objective>
Add CI status badge and crates.io version badge to README.md header.

Purpose: Provide at-a-glance project health indicators for visitors
Output: README.md with visible status badges; release workflow already publishes on tags
</objective>

<execution_context>
@/Users/vmakaev/.claude/get-shit-done/workflows/execute-plan.md
@/Users/vmakaev/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
Repository: VladimirMakaev/rslph
CI workflow: .github/workflows/ci.yml (runs clippy, fmt, test)
Release workflow: .github/workflows/release.yml (publishes to crates.io on v* tags)
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add CI and crates.io badges to README</name>
  <files>README.md</files>
  <action>
Add badges immediately after the `# rslph` heading line (line 1). Insert a new line 2 with the following badges:

```markdown
[![CI](https://github.com/VladimirMakaev/rslph/actions/workflows/ci.yml/badge.svg)](https://github.com/VladimirMakaev/rslph/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/rslph.svg)](https://crates.io/crates/rslph)
[![License](https://img.shields.io/crates/l/rslph.svg)](https://crates.io/crates/rslph)
```

This adds:
1. CI status badge - links to GitHub Actions workflow, shows pass/fail status
2. Crates.io version badge - shows current published version
3. License badge - shows license from Cargo.toml

Also update the "Clone the repository" section (around line 37) to use the correct GitHub URL:
- Change `git clone https://github.com/yourusername/rslph.git` to `git clone https://github.com/VladimirMakaev/rslph.git`

Add an alternative installation method via cargo after the "From Source" section:
```markdown
### From crates.io

```bash
cargo install rslph
```
```
  </action>
  <verify>
Read README.md and confirm:
1. Badges appear on line 3 (after heading and blank line)
2. Badge URLs point to VladimirMakaev/rslph
3. Clone URL is correct
4. cargo install section exists
  </verify>
  <done>
README.md displays CI status badge, crates.io version badge, and license badge at the top. Clone URL is corrected. Cargo install instructions added.
  </done>
</task>

<task type="auto">
  <name>Task 2: Verify release workflow triggers cargo publish</name>
  <files>.github/workflows/release.yml</files>
  <action>
Verify the release.yml workflow is correctly configured:
1. Triggers on push of tags matching 'v*' pattern
2. Runs `cargo publish` with CARGO_REGISTRY_TOKEN secret
3. No changes needed if already correct (which it is based on current file)

The workflow at .github/workflows/release.yml already correctly:
- Triggers on: push tags 'v*'
- Runs: cargo publish with CARGO_REGISTRY_TOKEN

No modifications required. This task is verification only.
  </action>
  <verify>
Read .github/workflows/release.yml and confirm:
1. on.push.tags includes 'v*'
2. cargo publish command exists
3. CARGO_REGISTRY_TOKEN is used
  </verify>
  <done>
Release workflow is confirmed to publish to crates.io when version tags are pushed.
  </done>
</task>

</tasks>

<verification>
1. `grep -n "badge.svg" README.md` shows badge lines near top
2. `grep "crates.io/crates/rslph" README.md` shows crates.io link
3. `grep "VladimirMakaev/rslph" README.md` shows correct repo URL
4. `grep "cargo install rslph" README.md` shows install instructions
</verification>

<success_criteria>
- README.md has CI badge linking to GitHub Actions ci.yml workflow
- README.md has crates.io version badge linking to crates.io/crates/rslph
- README.md has license badge
- Clone URL uses correct VladimirMakaev/rslph repository
- Cargo install instructions present
- Release workflow confirmed to publish on v* tags
</success_criteria>

<output>
After completion, create `.planning/quick/003-create-a-badge-on-readme-with-ci-status-/003-SUMMARY.md`
</output>
