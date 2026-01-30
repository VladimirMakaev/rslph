---
phase: quick
plan: 002
type: execute
wave: 1
depends_on: []
files_modified:
  - .github/workflows/ci.yml
  - .github/workflows/release.yml
  - Cargo.toml
autonomous: true

must_haves:
  truths:
    - "Every push and PR runs clippy, fmt check, and tests"
    - "Tagged releases publish to crates.io"
    - "Cargo.toml has required metadata for crates.io"
  artifacts:
    - path: ".github/workflows/ci.yml"
      provides: "CI workflow for tests/lint"
    - path: ".github/workflows/release.yml"
      provides: "Release workflow for crates.io"
    - path: "Cargo.toml"
      provides: "Package metadata for publishing"
  key_links:
    - from: ".github/workflows/release.yml"
      to: "crates.io"
      via: "cargo publish with CARGO_REGISTRY_TOKEN"
---

<objective>
Setup GitHub Actions CI to run all tests, linting, and formatting checks on every push/PR. Configure release workflow to publish to crates.io on tagged releases.

Purpose: Automate quality gates and enable easy distribution via crates.io
Output: Two GitHub Actions workflows (ci.yml, release.yml) and updated Cargo.toml with publish metadata
</objective>

<execution_context>
@~/.claude/get-shit-done/workflows/execute-plan.md
@~/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@Cargo.toml
</context>

<tasks>

<task type="auto">
  <name>Task 1: Create CI workflow for tests and linting</name>
  <files>.github/workflows/ci.yml</files>
  <action>
Create `.github/workflows/ci.yml` with:
- Trigger: push to any branch, pull_request to main/master
- Jobs:
  1. **check** - Run `cargo clippy -- -D warnings` (fail on any warning)
  2. **fmt** - Run `cargo fmt --check` (fail if not formatted)
  3. **test** - Run `cargo test --all-targets` (all unit and integration tests)
- Use `ubuntu-latest` runner
- Use `dtolnay/rust-toolchain@stable` for Rust setup
- Cache cargo registry and target with `Swatinem/rust-cache@v2`
- Run jobs in parallel (no dependencies between them)
  </action>
  <verify>
Validate YAML syntax: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"`
  </verify>
  <done>CI workflow file exists at `.github/workflows/ci.yml` with check, fmt, and test jobs</done>
</task>

<task type="auto">
  <name>Task 2: Update Cargo.toml with publish metadata</name>
  <files>Cargo.toml</files>
  <action>
Add required crates.io metadata to Cargo.toml `[package]` section:
- `description = "CLI tool for LLM-powered autonomous task execution"`
- `license = "MIT"` (or appropriate license)
- `repository = "https://github.com/vmakaev/rslph"` (verify owner from git remote)
- `readme = "README.md"`
- `keywords = ["cli", "llm", "automation", "claude", "ai"]`
- `categories = ["command-line-utilities", "development-tools"]`

Check the actual repository URL from git remote:
```bash
git remote get-url origin
```
  </action>
  <verify>
Run `cargo publish --dry-run` to verify all required metadata is present
  </verify>
  <done>Cargo.toml has description, license, repository, readme, keywords, and categories fields</done>
</task>

<task type="auto">
  <name>Task 3: Create release workflow for crates.io publishing</name>
  <files>.github/workflows/release.yml</files>
  <action>
Create `.github/workflows/release.yml` with:
- Trigger: push of tags matching `v*` (e.g., v0.1.0, v1.0.0)
- Jobs:
  1. **publish** - Publish to crates.io
    - Checkout code
    - Setup Rust stable
    - Run `cargo publish` with `CARGO_REGISTRY_TOKEN` secret
- Use `ubuntu-latest` runner
- Add environment variable: `CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}`

Note: User will need to add CARGO_REGISTRY_TOKEN secret in GitHub repo settings
  </action>
  <verify>
Validate YAML syntax: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"`
  </verify>
  <done>Release workflow file exists at `.github/workflows/release.yml` triggered by version tags</done>
</task>

</tasks>

<verification>
1. Both workflow files exist and have valid YAML syntax
2. `cargo publish --dry-run` succeeds (all metadata present)
3. CI workflow has check, fmt, and test jobs
4. Release workflow triggers on v* tags and uses CARGO_REGISTRY_TOKEN
</verification>

<success_criteria>
- `.github/workflows/ci.yml` exists with clippy, fmt, and test jobs
- `.github/workflows/release.yml` exists with publish job triggered by version tags
- `Cargo.toml` has all required crates.io metadata
- `cargo publish --dry-run` passes without errors
</success_criteria>

<output>
After completion, create `.planning/quick/002-setup-github-ci-to-run-all-tests-setup-r/002-SUMMARY.md`
</output>
