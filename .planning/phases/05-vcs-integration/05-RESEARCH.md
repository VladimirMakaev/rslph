# Phase 5: VCS Integration - Research

**Researched:** 2026-01-18
**Domain:** Version control auto-commit, Git/Sapling integration, rollback safety
**Confidence:** HIGH (Official docs verified, jj-vcs project analysis, existing codebase patterns)

## Summary

Phase 5 implements automatic VCS commits after each build iteration to enable rollback safety. This requires:

1. **VCS Detection**: Determine whether the repository uses Git (`.git`) or Sapling (`.sl`) by walking up the directory tree
2. **Auto-Commit**: After each iteration, stage changed files and commit with a descriptive message
3. **Error Handling**: Gracefully handle "nothing to commit" scenarios and VCS failures

**Key Decision: Shell out to `git` and `sl` commands rather than using the git2 crate.** This approach is:
- Simpler (no C library dependency via libgit2)
- More compatible (uses user's configured git/sl with all their settings)
- Battle-tested (jj-vcs project migrated from git2 to subprocess for similar reasons)
- Sufficient for simple add/commit operations (no network operations needed)

**Primary recommendation:** Create a VCS abstraction trait with Git and Sapling implementations that shell out to respective commands. Detect VCS type at iteration start, commit atomically after each iteration.

## Standard Stack

The established libraries/tools for this phase:

### Core (Already in Codebase)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tokio | 1.49 | Async subprocess execution | Existing pattern in ClaudeRunner |
| std::process::Command | stdlib | Synchronous VCS commands | Simple operations don't need async |
| std::path | stdlib | Directory traversal for VCS detection | Walk up tree to find .git/.sl |
| thiserror | 2.0 | VCS-specific error types | Existing error handling pattern |

### External Commands (Required)

| Command | Purpose | Detection |
|---------|---------|-----------|
| `git` | Git VCS operations | `.git` directory exists |
| `sl` | Sapling VCS operations | `.sl` directory exists |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Shell out to git | git2 crate (libgit2 bindings) | git2 adds C dependency, SSH issues, version pinning problems, slower for some ops. jj-vcs project deprecated git2 for these reasons |
| Shell out to sl | No Rust crate exists | Sapling has no Rust bindings, CLI is the only option |
| Custom VCS abstraction | gix/gitoxide | Overkill for simple add/commit; pure Rust but complex API |

**Installation:**
No new dependencies required. Uses existing tokio and std::process.

## Architecture Patterns

### Recommended Project Structure (Extensions)

```
src/
├── vcs/
│   ├── mod.rs           # VCS trait, VcsType enum, detect_vcs()
│   ├── git.rs           # GitVcs implementation
│   ├── sapling.rs       # SaplingVcs implementation
│   └── commit.rs        # Commit message formatting
├── build/
│   ├── command.rs       # Add VCS commit after iteration (hook point)
│   └── ...
└── error.rs             # Add VcsError variants
```

### Pattern 1: VCS Abstraction Trait

**What:** Define a trait for VCS operations with Git and Sapling implementations
**When to use:** All VCS operations in the codebase

```rust
// Source: Standard Rust trait pattern

/// VCS type detected in repository
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsType {
    Git,
    Sapling,
}

/// VCS operations abstraction
pub trait Vcs: Send + Sync {
    /// Get the VCS type
    fn vcs_type(&self) -> VcsType;

    /// Check if there are uncommitted changes
    fn has_changes(&self) -> Result<bool, VcsError>;

    /// Stage all changes (git add -A / sl addremove)
    fn stage_all(&self) -> Result<(), VcsError>;

    /// Create a commit with message
    fn commit(&self, message: &str) -> Result<String, VcsError>;

    /// Commit all changes atomically (stage + commit)
    fn commit_all(&self, message: &str) -> Result<Option<String>, VcsError> {
        if !self.has_changes()? {
            return Ok(None); // Nothing to commit
        }
        self.stage_all()?;
        let hash = self.commit(message)?;
        Ok(Some(hash))
    }
}
```

### Pattern 2: VCS Detection by Walking Up Directory Tree

**What:** Find VCS root by checking for `.git` or `.sl` directories
**When to use:** At start of build command, before any VCS operations

```rust
// Source: git2::Repository::discover pattern, adapted for multi-VCS

use std::path::{Path, PathBuf};

/// Detection result
pub struct VcsDetection {
    pub vcs_type: VcsType,
    pub root: PathBuf,
}

/// Detect VCS type and root directory by walking up from start_path
pub fn detect_vcs(start_path: &Path) -> Result<VcsDetection, VcsError> {
    let start = if start_path.is_file() {
        start_path.parent().unwrap_or(start_path)
    } else {
        start_path
    };

    let mut current = start.canonicalize()
        .map_err(|e| VcsError::Detection(format!("Invalid path: {}", e)))?;

    loop {
        // Check for Sapling first (it can have .git symlink for compatibility)
        if current.join(".sl").is_dir() {
            return Ok(VcsDetection {
                vcs_type: VcsType::Sapling,
                root: current,
            });
        }

        // Check for Git
        if current.join(".git").exists() {
            return Ok(VcsDetection {
                vcs_type: VcsType::Git,
                root: current,
            });
        }

        // Move up to parent
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => {
                return Err(VcsError::NotARepository(
                    start.to_string_lossy().to_string()
                ));
            }
        }
    }
}
```

### Pattern 3: Git Implementation (Shell Out)

**What:** Git operations via subprocess commands
**When to use:** When VcsType::Git is detected

```rust
// Source: Standard git CLI patterns

use std::process::{Command, Output};

pub struct GitVcs {
    root: PathBuf,
}

impl GitVcs {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn run_git(&self, args: &[&str]) -> Result<Output, VcsError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .output()
            .map_err(|e| VcsError::CommandFailed {
                command: format!("git {}", args.join(" ")),
                error: e.to_string(),
            })?;
        Ok(output)
    }
}

impl Vcs for GitVcs {
    fn vcs_type(&self) -> VcsType {
        VcsType::Git
    }

    fn has_changes(&self) -> Result<bool, VcsError> {
        // git status --porcelain returns empty string if clean
        let output = self.run_git(&["status", "--porcelain"])?;
        if !output.status.success() {
            return Err(VcsError::CommandFailed {
                command: "git status --porcelain".to_string(),
                error: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        Ok(!output.stdout.is_empty())
    }

    fn stage_all(&self) -> Result<(), VcsError> {
        // git add -A stages all changes including deletions
        let output = self.run_git(&["add", "-A"])?;
        if !output.status.success() {
            return Err(VcsError::CommandFailed {
                command: "git add -A".to_string(),
                error: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        Ok(())
    }

    fn commit(&self, message: &str) -> Result<String, VcsError> {
        // git commit -m "message" --no-verify (skip hooks for auto-commits)
        let output = self.run_git(&["commit", "-m", message, "--no-verify"])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // "nothing to commit" is exit code 1 but not an error for us
            if stderr.contains("nothing to commit") {
                return Err(VcsError::NothingToCommit);
            }
            return Err(VcsError::CommitFailed(stderr.to_string()));
        }

        // Extract commit hash from output
        // git commit output: "[branch hash] message"
        let stdout = String::from_utf8_lossy(&output.stdout);
        let hash = extract_commit_hash(&stdout).unwrap_or_default();
        Ok(hash)
    }
}

fn extract_commit_hash(output: &str) -> Option<String> {
    // Parse "[branch abc1234] message" format
    output.lines().next()
        .and_then(|line| line.split_whitespace().nth(1))
        .map(|s| s.trim_end_matches(']').to_string())
}
```

### Pattern 4: Sapling Implementation

**What:** Sapling operations via `sl` command
**When to use:** When VcsType::Sapling is detected

```rust
// Source: Sapling SCM documentation https://sapling-scm.com/docs/commands/

pub struct SaplingVcs {
    root: PathBuf,
}

impl SaplingVcs {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn run_sl(&self, args: &[&str]) -> Result<Output, VcsError> {
        let output = Command::new("sl")
            .args(args)
            .current_dir(&self.root)
            .output()
            .map_err(|e| VcsError::CommandFailed {
                command: format!("sl {}", args.join(" ")),
                error: e.to_string(),
            })?;
        Ok(output)
    }
}

impl Vcs for SaplingVcs {
    fn vcs_type(&self) -> VcsType {
        VcsType::Sapling
    }

    fn has_changes(&self) -> Result<bool, VcsError> {
        // sl status returns empty if clean
        let output = self.run_sl(&["status"])?;
        if !output.status.success() {
            return Err(VcsError::CommandFailed {
                command: "sl status".to_string(),
                error: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        Ok(!output.stdout.is_empty())
    }

    fn stage_all(&self) -> Result<(), VcsError> {
        // sl addremove stages new files and removes missing files
        let output = self.run_sl(&["addremove"])?;
        if !output.status.success() {
            return Err(VcsError::CommandFailed {
                command: "sl addremove".to_string(),
                error: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        Ok(())
    }

    fn commit(&self, message: &str) -> Result<String, VcsError> {
        // sl commit -m "message"
        let output = self.run_sl(&["commit", "-m", message])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("nothing changed") {
                return Err(VcsError::NothingToCommit);
            }
            return Err(VcsError::CommitFailed(stderr.to_string()));
        }

        // Extract commit hash
        let stdout = String::from_utf8_lossy(&output.stdout);
        let hash = stdout.lines().next()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        Ok(hash)
    }
}
```

### Pattern 5: Commit Message Formatting

**What:** Consistent, informative auto-commit messages
**When to use:** Every iteration commit

```rust
// Source: Git commit best practices

/// Format commit message for iteration checkpoint
pub fn format_iteration_commit(
    iteration: u32,
    tasks_completed: u32,
    project_name: &str,
) -> String {
    if tasks_completed == 0 {
        format!(
            "rslph({}): iteration {} checkpoint (no tasks completed)",
            project_name,
            iteration
        )
    } else {
        format!(
            "rslph({}): iteration {} - {} task(s) completed",
            project_name,
            iteration,
            tasks_completed
        )
    }
}

// Example messages:
// "rslph(my-project): iteration 1 - 2 task(s) completed"
// "rslph(my-project): iteration 5 checkpoint (no tasks completed)"
```

### Pattern 6: Integration with Build Loop

**What:** Hook VCS commit after successful iteration
**When to use:** In build/command.rs after log_iteration

```rust
// Source: Existing build loop structure

// In run_build_command():
BuildState::IterationComplete { iteration, tasks_completed } => {
    // ... existing logging ...

    log_iteration(&mut ctx, iteration, tasks_completed)?;

    // NEW: VCS auto-commit for rollback safety
    if let Some(vcs) = &ctx.vcs {
        let message = format_iteration_commit(
            iteration,
            tasks_completed,
            &ctx.progress.name,
        );

        match vcs.commit_all(&message) {
            Ok(Some(hash)) => {
                eprintln!("[VCS] Committed: {} ({})", hash, vcs.vcs_type());
            }
            Ok(None) => {
                eprintln!("[VCS] No changes to commit");
            }
            Err(e) => {
                // Log but don't fail the build for VCS errors
                eprintln!("[VCS] Warning: {}", e);
            }
        }
    }

    // ... rest of state machine ...
}
```

### Anti-Patterns to Avoid

- **Failing build on VCS errors:** VCS operations are rollback convenience, not core functionality. Log warnings, don't fail.
- **Using git2 crate for simple operations:** Adds C dependency, SSH issues, version lock-in. Shell out instead.
- **Skipping "nothing to commit" check:** Always check `has_changes()` before committing to avoid unnecessary errors.
- **Using interactive git/sl commands:** Never use `-i` flag (add -i, commit -i) as they require terminal.
- **Assuming VCS is available:** Gracefully handle non-repository directories (no .git or .sl).
- **Committing during iteration:** Only commit AFTER iteration completes to ensure atomic checkpoints.

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Git operations | Custom libgit2 wrapper | Shell out to `git` CLI | Simpler, uses user's config, no C dependency |
| Sapling operations | Nothing exists | Shell out to `sl` CLI | Only option, Sapling has no Rust bindings |
| VCS detection | Recursive path search | Walk up checking `.git`/`.sl` | Standard pattern, same as git2::discover |
| Commit message format | Freeform text | Consistent `rslph(project): iteration N` | Enables grep/filtering of auto-commits |
| Empty commit handling | Silent skip | Explicit check + return None | Prevents confusing error messages |

**Key insight:** For local-only VCS operations (add, commit, status), shelling out to the CLI is simpler and more compatible than library bindings.

## Common Pitfalls

### Pitfall 1: VCS Command Not Found

**What goes wrong:** `git` or `sl` not installed or not in PATH
**Why it happens:** Not everyone has VCS CLI installed globally
**How to avoid:**
- Check command availability at build start
- Disable auto-commit if VCS not found (warn, don't fail)
- Gracefully degrade to no VCS mode
**Warning signs:** "command not found" errors

### Pitfall 2: Nothing to Commit Error

**What goes wrong:** `git commit` fails with exit code 1 when nothing to commit
**Why it happens:** Not checking for changes before committing
**How to avoid:**
- Always call `has_changes()` before `commit()`
- Handle `VcsError::NothingToCommit` as success, not error
- Use `commit_all()` which encapsulates this check
**Warning signs:** Exit code 1 from commit, stderr contains "nothing to commit"

### Pitfall 3: Commit During Active Iteration

**What goes wrong:** Partial state committed, unusable checkpoint
**Why it happens:** Triggering commit too early in iteration lifecycle
**How to avoid:**
- Only commit AFTER `log_iteration()` succeeds
- Only commit when state transitions to `IterationComplete`
- Never commit in `Running` state
**Warning signs:** Checkpoints with incomplete progress files

### Pitfall 4: Pre-commit Hooks Fail

**What goes wrong:** User's pre-commit hooks reject auto-commits
**Why it happens:** Linters, formatters, or validators in hooks
**How to avoid:**
- Use `--no-verify` flag to skip hooks for auto-commits
- Document that auto-commits bypass hooks
- Let user opt-in to hook enforcement via config (future feature)
**Warning signs:** Commit rejected, hook errors in stderr

### Pitfall 5: VCS Detection in Submodule

**What goes wrong:** Detects submodule's VCS instead of parent repo
**Why it happens:** Walking up finds first .git, which might be submodule
**How to avoid:**
- For Phase 5 scope: Accept this limitation, document it
- Future: Check for `.git` file (submodule) vs directory (real repo)
**Warning signs:** Commits only affecting submodule directory

### Pitfall 6: Sapling .sl/store/git Mode

**What goes wrong:** Sapling repo has `.git` symlink for compatibility, detected as Git
**Why it happens:** Sapling can operate in Git-compatible mode
**How to avoid:**
- Check for `.sl` FIRST, before `.git`
- `.sl` directory takes precedence
**Warning signs:** Using git commands on Sapling repo

## Code Examples

Verified patterns from official sources:

### Example 1: VcsError Type

```rust
// src/error.rs additions

#[derive(Debug, thiserror::Error)]
pub enum VcsError {
    #[error("Not a VCS repository: {0}")]
    NotARepository(String),

    #[error("VCS command failed: {command}")]
    CommandFailed {
        command: String,
        error: String,
    },

    #[error("Nothing to commit")]
    NothingToCommit,

    #[error("Commit failed: {0}")]
    CommitFailed(String),

    #[error("VCS not available: {0} not found in PATH")]
    NotAvailable(String),

    #[error("VCS detection failed: {0}")]
    Detection(String),
}
```

### Example 2: Factory Function

```rust
// src/vcs/mod.rs

/// Create appropriate VCS implementation based on detection
pub fn create_vcs(working_dir: &Path) -> Result<Box<dyn Vcs>, VcsError> {
    let detection = detect_vcs(working_dir)?;

    match detection.vcs_type {
        VcsType::Git => {
            // Verify git is available
            Command::new("git")
                .arg("--version")
                .output()
                .map_err(|_| VcsError::NotAvailable("git".to_string()))?;

            Ok(Box::new(GitVcs::new(detection.root)))
        }
        VcsType::Sapling => {
            // Verify sl is available
            Command::new("sl")
                .arg("--version")
                .output()
                .map_err(|_| VcsError::NotAvailable("sl".to_string()))?;

            Ok(Box::new(SaplingVcs::new(detection.root)))
        }
    }
}

/// Attempt to create VCS, returning None if not available
pub fn try_create_vcs(working_dir: &Path) -> Option<Box<dyn Vcs>> {
    match create_vcs(working_dir) {
        Ok(vcs) => Some(vcs),
        Err(e) => {
            eprintln!("[VCS] Warning: {}", e);
            None
        }
    }
}
```

### Example 3: BuildContext Extension

```rust
// src/build/state.rs additions

pub struct BuildContext {
    // ... existing fields ...

    /// VCS for auto-commit (None if not in a repository or VCS unavailable)
    pub vcs: Option<Box<dyn Vcs>>,
}

impl BuildContext {
    pub fn new(
        progress_path: PathBuf,
        progress: ProgressFile,
        config: Config,
        cancel_token: CancellationToken,
        once_mode: bool,
        dry_run: bool,
    ) -> Self {
        // Detect VCS during context creation
        let vcs = try_create_vcs(&progress_path);

        if let Some(ref v) = vcs {
            eprintln!("[VCS] Detected {} repository", v.vcs_type());
        }

        Self {
            progress_path,
            progress,
            config,
            cancel_token,
            current_iteration: 0,
            max_iterations: config.max_iterations,
            once_mode,
            dry_run,
            iteration_start: None,
            vcs,
        }
    }
}
```

### Example 4: Git Porcelain Status Parsing

```rust
// git status --porcelain output format:
// XY PATH
// Where X = index status, Y = worktree status
// Examples:
//  M file.rs       - modified in worktree (not staged)
// M  file.rs       - modified and staged
// MM file.rs       - modified, staged, then modified again
// A  new.rs        - new file staged
// ?? untracked.rs  - untracked file

fn has_changes_porcelain(&self) -> Result<bool, VcsError> {
    let output = self.run_git(&["status", "--porcelain"])?;
    // Non-empty output means changes exist
    Ok(!output.stdout.is_empty())
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| libgit2/git2 crate | Shell out to git CLI | 2024-2025 | jj-vcs deprecated git2, simpler approach wins |
| Manual .git detection | Repository::discover pattern | Standard | Walk up tree, standard across tools |
| Git-only support | Multi-VCS (Git + Sapling) | 2023+ | Sapling adoption at Meta/other companies |
| Silent VCS errors | Warn-and-continue | Best practice | VCS is convenience, not critical path |

**Deprecated/outdated:**
- Using `git2` crate for simple operations: SSH issues, C dependency, version lock-in
- Expecting only Git: Sapling growing in adoption
- Making VCS errors fatal: Should be warnings, not failures

## Open Questions

Things that couldn't be fully resolved:

1. **Submodule Handling**
   - What we know: Detection finds first .git, may be submodule
   - What's unclear: Should we commit to parent or submodule?
   - Recommendation: Accept limitation for v1, commit to detected repo

2. **User Identity for Commits**
   - What we know: git/sl use configured user.name and user.email
   - What's unclear: What if not configured?
   - Recommendation: Let command fail with helpful error from git/sl

3. **Large Repository Performance**
   - What we know: git status can be slow in huge repos
   - What's unclear: Is this a practical concern?
   - Recommendation: Monitor, consider `--untracked-files=no` if slow

4. **Rollback UX**
   - What we know: Commits exist for rollback
   - What's unclear: How users actually rollback (git reset, git revert, sl undo?)
   - Recommendation: Document in user guide, suggest `git reset --hard HEAD~1`

## Sources

### Primary (HIGH confidence)
- [git2 0.20.3 docs](https://docs.rs/git2/latest/git2/) - Repository, Index, Signature APIs
- [Sapling SCM docs](https://sapling-scm.com/docs/commands/add) - add, commit command syntax
- [Sapling SCM commit docs](https://sapling-scm.com/docs/commands/commit) - commit options and behavior
- [jj-vcs git2 deprecation issue](https://github.com/jj-vcs/jj/issues/5548) - Reasons to avoid git2 crate

### Secondary (MEDIUM confidence)
- [git status --porcelain](https://www.stefanjudis.com/today-i-learned/the-short-version-of-git-status-and-the-close-but-different-porcelain-mode/) - Machine-readable status format
- [libgit2 vs git CLI discussion](https://lists.gnu.org/archive/html/guix-devel/2022-11/msg00272.html) - Performance and integration tradeoffs

### Tertiary (LOW confidence)
- General git commit message best practices from various sources

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - tokio/std::process already in codebase, VCS CLIs well-documented
- Architecture: HIGH - Trait pattern standard, shell-out approach validated by jj-vcs
- VCS detection: HIGH - Standard walk-up-tree pattern, used by git2::discover
- Sapling support: MEDIUM - Documentation available but less tested in ecosystem
- Error handling: HIGH - Standard Rust patterns, nothing-to-commit well-known issue

**Research date:** 2026-01-18
**Valid until:** 90 days (VCS patterns are stable, CLI interfaces rarely change)
