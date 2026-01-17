# Phase 1: Foundation - Research

**Researched:** 2026-01-17
**Domain:** Rust CLI infrastructure (config, parsing, file I/O)
**Confidence:** HIGH

## Summary

Phase 1 establishes the infrastructure for rslph: TOML configuration with layered precedence, CLI parsing with subcommands, and progress file handling with atomic writes.

- **Config system**: Use `figment` for layered configuration (defaults < file < env < CLI). Integrates cleanly with `clap` via `Serialized` provider. TOML via `figment`'s built-in TOML feature. Config location via `directories` crate for XDG-compliant paths.

- **CLI parser**: Use `clap` v4 with derive macros. Subcommands via `#[derive(Subcommand)]` enum. Global flags on main struct, subcommand-specific args on enum variants. The value_source() API distinguishes default from explicit values for proper figment merging.

- **Progress file**: Use `pulldown-cmark` with `ENABLE_TASKLISTS` for markdown parsing. `TaskListMarker(bool)` event for checkbox state. Atomic writes via temp file + rename pattern (use `atomicwrites` crate or manual implementation).

**Primary recommendation:** Follow the clap+figment integration pattern that separates CLI defaults from explicit values, enabling correct precedence where explicit CLI args override env vars override config file.

## Config System

### Stack

| Crate | Version | Purpose | Confidence |
|-------|---------|---------|------------|
| figment | 0.10.19 | Layered config merging | HIGH |
| toml | 0.9.11 | TOML parsing (via figment feature) | HIGH |
| serde | 1.0.228 | Serialization/deserialization | HIGH |
| directories | 6.0.0 | XDG-compliant config paths | HIGH |

### Layered Precedence Pattern

The correct precedence order (lowest to highest):

1. **CLI defaults** (clap default values)
2. **Config file** (TOML at `~/.config/rslph/config.toml`)
3. **Environment variables** (`RSLPH_*` prefix)
4. **Explicit CLI arguments** (user-provided flags)

**Implementation pattern:**

```rust
use figment::{Figment, providers::{Serialized, Toml, Env}};
use clap::Parser;

// Separate defaults from explicit values
let matches = Cli::command().get_matches();
let cli_defaults = /* values where matches.value_source() == DefaultValue */;
let cli_explicit = /* values where matches.value_source() != DefaultValue */;

let config: Config = Figment::new()
    .merge(Serialized::defaults(cli_defaults))   // 1. CLI defaults (lowest)
    .merge(Toml::file(config_path))              // 2. Config file
    .merge(Env::prefixed("RSLPH_"))              // 3. Environment
    .merge(Serialized::defaults(cli_explicit))   // 4. Explicit CLI (highest)
    .extract()?;
```

Source: [clap + Figment integration gist](https://gist.github.com/qknight/0ec68e64634e3eb7b9f9d00691f22443)

### Config File Location

Use `directories` crate for platform-appropriate paths:

```rust
use directories::ProjectDirs;

fn config_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "rslph")
        .map(|dirs| dirs.config_dir().join("config.toml"))
}
```

Platform paths:
- **Linux:** `~/.config/rslph/config.toml`
- **macOS:** `~/Library/Application Support/rslph/config.toml`
- **Windows:** `C:\Users\<user>\AppData\Roaming\rslph\config\config.toml`

Source: [directories 6.0.0 docs](https://docs.rs/directories/6.0.0/directories/struct.ProjectDirs.html)

### Config Struct Pattern

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]  // Use Default impl for missing fields
pub struct Config {
    /// Path to claude CLI executable
    pub claude_path: String,

    /// Maximum iterations before stopping
    pub max_iterations: u32,

    /// Number of recent threads to display
    pub recent_threads: u32,

    /// Notification interval (every N iterations)
    pub notify_interval: u32,

    /// Path to plan prompt file (None = use built-in)
    pub plan_prompt: Option<PathBuf>,

    /// Path to build prompt file (None = use built-in)
    pub build_prompt: Option<PathBuf>,

    /// Shell for notify script execution
    pub notify_shell: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            claude_path: "claude".to_string(),
            max_iterations: 20,
            recent_threads: 5,
            notify_interval: 10,
            plan_prompt: None,
            build_prompt: None,
            notify_shell: "/bin/sh".to_string(),
        }
    }
}
```

### Serde Best Practices

- Use `#[serde(default)]` on struct to allow partial config files
- Use `#[serde(deny_unknown_fields)]` to catch typos
- Use `Option<T>` for truly optional fields that default to None
- Use `#[serde(default = "function_name")]` for custom defaults

```rust
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Config {
    #[serde(default = "default_max_iterations")]
    max_iterations: u32,
}

fn default_max_iterations() -> u32 { 20 }
```

Source: [Rust Users Forum: Serde TOML patterns](https://users.rust-lang.org/t/serde-toml-deserialize-with-on-options/77347)

## CLI Parser

### Stack

| Crate | Version | Purpose | Confidence |
|-------|---------|---------|------------|
| clap | 4.5.54 | CLI argument parsing | HIGH |

Features needed: `derive`, `env`

### Subcommand Pattern

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "rslph")]
#[command(about = "Ralph Wiggum Loop - autonomous AI coding agent")]
#[command(version)]
pub struct Cli {
    /// Override config file path
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// Override claude command path
    #[arg(long, global = true)]
    pub claude_path: Option<String>,

    /// Maximum iterations
    #[arg(long, global = true)]
    pub max_iterations: Option<u32>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Transform idea/plan into structured progress file
    Plan {
        /// Path to the plan/idea file
        plan: PathBuf,

        /// Use adaptive mode with clarifying questions
        #[arg(long)]
        adaptive: bool,
    },

    /// Execute tasks iteratively with fresh context
    Build {
        /// Path to the progress file
        plan: PathBuf,

        /// Run single iteration only
        #[arg(long)]
        once: bool,

        /// Preview without executing
        #[arg(long)]
        dry_run: bool,
    },
}
```

Source: [clap 4.5 docs](https://docs.rs/clap/latest/clap/struct.Command.html), [Building CLI tools with Rust](https://dev.to/godofgeeks/building-cli-tools-with-rust-clap-4bo2)

### Separating Default from Explicit Values

To correctly merge with figment, track which CLI args were explicitly provided:

```rust
use clap::parser::ValueSource;

fn is_explicit(matches: &ArgMatches, arg_name: &str) -> bool {
    matches.value_source(arg_name) != Some(ValueSource::DefaultValue)
}

// Build two config structs:
// 1. cli_defaults: only values where is_explicit() == false
// 2. cli_explicit: only values where is_explicit() == true
```

### Flag Conventions

- **Short flags:** Common operations (`-c` for config, `-v` for verbose)
- **Long flags:** All options (`--max-iterations`, `--claude-path`)
- **Global flags:** Options that apply to all subcommands (use `global = true`)
- **Positional args:** Primary inputs (plan file path)

## Progress File Parser

### Stack

| Crate | Version | Purpose | Confidence |
|-------|---------|---------|------------|
| pulldown-cmark | 0.13.0 | Markdown parsing | HIGH |
| atomicwrites | 0.4.4 | Atomic file writes | HIGH |
| regex | (optional) | Section extraction backup | MEDIUM |

### Markdown Parsing with pulldown-cmark

Enable task list parsing:

```rust
use pulldown_cmark::{Parser, Options, Event, Tag};

let mut options = Options::empty();
options.insert(Options::ENABLE_TASKLISTS);

let parser = Parser::new_ext(markdown_content, options);

for event in parser {
    match event {
        Event::Start(Tag::Heading { level, .. }) => {
            // Section header starting
        }
        Event::TaskListMarker(checked) => {
            // checked == true for [x], false for [ ]
        }
        Event::Text(text) => {
            // Text content
        }
        _ => {}
    }
}
```

Source: [pulldown-cmark 0.13.0 docs](https://docs.rs/pulldown-cmark/latest/pulldown_cmark/)

### Progress File Format

Based on requirements (PROG-01 through PROG-07):

```markdown
# Progress: [Plan Name]

## Status

<!-- RALPH_DONE when complete -->
In Progress

## Analysis

[Research and analysis notes]

## Tasks

### Phase 1: Foundation
- [x] Task 1 description
- [ ] Task 2 description

### Phase 2: Core
- [ ] Task 3 description

## Testing Strategy

- Unit tests for X
- Integration tests for Y

## Completed This Iteration

- [x] Task 1 description

## Recent Attempts

### Iteration 5
- Tried: X
- Result: Failed because Y
- Next: Will try Z

### Iteration 4
- Tried: A
- Result: Success

## Iteration Log

| Iteration | Started | Duration | Tasks Completed | Notes |
|-----------|---------|----------|-----------------|-------|
| 5 | 2026-01-17 10:30 | 2m 15s | 0 | Error in X |
| 4 | 2026-01-17 10:25 | 3m 45s | 1 | Completed Task 1 |
```

### Section Extraction Pattern

```rust
pub struct ProgressFile {
    pub status: String,
    pub analysis: String,
    pub tasks: Vec<Task>,
    pub testing_strategy: String,
    pub completed_this_iteration: Vec<String>,
    pub recent_attempts: Vec<Attempt>,
    pub iteration_log: Vec<IterationEntry>,
}

pub struct Task {
    pub phase: String,
    pub description: String,
    pub completed: bool,
}

pub struct Attempt {
    pub iteration: u32,
    pub tried: String,
    pub result: String,
    pub next: Option<String>,
}
```

### Atomic File Writes

**Pattern 1: Using atomicwrites crate**

```rust
use atomicwrites::{AtomicFile, AllowOverwrite};
use std::io::Write;

fn write_progress(path: &Path, content: &str) -> io::Result<()> {
    let af = AtomicFile::new(path, AllowOverwrite);
    af.write(|f| {
        f.write_all(content.as_bytes())
    })?;
    Ok(())
}
```

**Pattern 2: Manual implementation**

```rust
use std::fs;
use std::io::Write;
use std::path::Path;

fn atomic_write(path: &Path, content: &str) -> io::Result<()> {
    let temp_path = path.with_extension("tmp");

    // Write to temp file
    let mut file = fs::File::create(&temp_path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;  // fsync for durability

    // Atomic rename
    fs::rename(&temp_path, path)?;

    Ok(())
}
```

**Note:** On Windows, `fs::rename` cannot overwrite existing files. Need explicit delete first:

```rust
#[cfg(target_os = "windows")]
fn atomic_write_windows(path: &Path, content: &str) -> io::Result<()> {
    let temp_path = path.with_extension("tmp");

    let mut file = fs::File::create(&temp_path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;

    // Windows: must remove target first
    let _ = fs::remove_file(path);  // Ignore if doesn't exist
    fs::rename(&temp_path, path)?;

    Ok(())
}
```

Source: [rust-atomicwrites GitHub](https://github.com/untitaker/rust-atomicwrites), [atomicwrites 0.4.4 docs](https://docs.rs/atomicwrites/latest/atomicwrites/)

## Recommended Crates

### Core (Phase 1)

```toml
[dependencies]
# CLI
clap = { version = "4.5", features = ["derive", "env"] }

# Configuration
figment = { version = "0.10", features = ["toml", "env"] }
serde = { version = "1.0", features = ["derive"] }

# Paths
directories = "6.0"

# Markdown parsing
pulldown-cmark = "0.13"

# Atomic writes
atomicwrites = "0.4"

# Error handling
color-eyre = "0.6"
thiserror = "2.0"
```

### Why These Specific Crates

| Crate | Why This One |
|-------|--------------|
| figment | Purpose-built for layered config. Cleaner than manual merging with `config` crate |
| clap | Undisputed Rust CLI standard. Derive API is ergonomic and type-safe |
| pulldown-cmark | Fast, safe, CommonMark-compliant. Task list support via flag |
| atomicwrites | Handles cross-platform atomic writes correctly |
| directories | XDG-compliant, no manual path building needed |

### Alternatives Considered

| Instead of | Could Use | Why Not |
|------------|-----------|---------|
| figment | config crate | Older API, clap integration less clean |
| pulldown-cmark | comrak | Heavier, more features than needed |
| atomicwrites | manual temp+rename | Edge cases on Windows, why reinvent |

## Gotchas/Warnings

### Config Precedence Trap

**Problem:** CLI defaults override config file values if merge order is wrong.

**Symptom:** Config file changes seem to have no effect.

**Fix:** Separate CLI defaults from explicit values using `value_source()`. Merge defaults first, explicit last.

### Serde Missing Field Errors

**Problem:** Adding new config fields breaks existing user config files.

**Symptom:** "missing field `new_field`" error.

**Fix:** Always use `#[serde(default)]` on struct. Implement `Default` trait.

### Unknown Fields Silent Failure

**Problem:** User typos in config (`max_iteratons` instead of `max_iterations`) silently ignored.

**Symptom:** Config changes seem to have no effect.

**Fix:** Use `#[serde(deny_unknown_fields)]` to reject typos.

### Atomic Write Windows Gotcha

**Problem:** `fs::rename` fails on Windows if target exists.

**Symptom:** "Access denied" or "file exists" errors on Windows.

**Fix:** Use `atomicwrites` crate which handles this, or explicitly delete before rename on Windows.

### pulldown-cmark Task List Flag

**Problem:** Task lists don't parse without enabling the extension.

**Symptom:** `- [ ]` parsed as regular list items, no `TaskListMarker` events.

**Fix:** Enable `Options::ENABLE_TASKLISTS` when creating parser.

### Config File Doesn't Exist

**Problem:** First-run user has no config file.

**Symptom:** Error trying to read non-existent file.

**Fix:** Use `Toml::file(path).nested()` which returns empty if file missing, or check existence first:

```rust
let figment = if config_path.exists() {
    Figment::new().merge(Toml::file(&config_path))
} else {
    Figment::new()
};
```

### Environment Variable Naming

**Problem:** Env var `RSLPH_CLAUDE_PATH` doesn't map to `claude_path` field.

**Symptom:** Environment overrides don't work.

**Fix:** Figment's `Env::prefixed("RSLPH_")` uses case-insensitive matching and underscore-to-field mapping. Field `claude_path` matches `RSLPH_CLAUDE_PATH`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Config file merging | Custom merge logic | figment | Edge cases in precedence, Option handling |
| CLI parsing | Manual arg parsing | clap | Validation, help text, error messages |
| XDG paths | Hardcoded `~/.config` | directories | macOS/Windows differ, XDG env vars |
| Atomic writes | Manual temp+rename | atomicwrites | Windows edge cases, fsync |
| Markdown parsing | Regex extraction | pulldown-cmark | Markdown is complex, edge cases everywhere |

## Code Examples

### Complete Config Loading

```rust
// Source: Verified pattern from figment + clap docs
use clap::{Parser, ArgMatches, parser::ValueSource};
use figment::{Figment, providers::{Serialized, Toml, Env}};
use directories::ProjectDirs;

pub fn load_config(cli: &Cli, matches: &ArgMatches) -> Result<Config> {
    // Get config file path
    let config_path = cli.config.clone().unwrap_or_else(|| {
        ProjectDirs::from("", "", "rslph")
            .map(|d| d.config_dir().join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    });

    // Build defaults-only config from CLI
    let cli_defaults = build_cli_config(cli, matches, true);

    // Build explicit-only config from CLI
    let cli_explicit = build_cli_config(cli, matches, false);

    // Layer in correct precedence
    let config: Config = Figment::new()
        .merge(Serialized::defaults(Config::default()))  // Hardcoded defaults
        .merge(Serialized::defaults(cli_defaults))       // CLI defaults
        .merge(Toml::file(&config_path))                 // Config file
        .merge(Env::prefixed("RSLPH_"))                  // Environment
        .merge(Serialized::defaults(cli_explicit))       // Explicit CLI
        .extract()?;

    Ok(config)
}

fn build_cli_config(cli: &Cli, matches: &ArgMatches, defaults_only: bool) -> PartialConfig {
    // Check each field's value source
    let include = |name: &str| {
        let is_default = matches.value_source(name) == Some(ValueSource::DefaultValue);
        if defaults_only { is_default } else { !is_default }
    };

    PartialConfig {
        claude_path: if include("claude-path") { cli.claude_path.clone() } else { None },
        max_iterations: if include("max-iterations") { cli.max_iterations } else { None },
        // ... etc
    }
}
```

### Complete Subcommand Matching

```rust
// Source: clap derive docs
fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let matches = Cli::command().get_matches();
    let config = load_config(&cli, &matches)?;

    match cli.command {
        Commands::Plan { plan, adaptive } => {
            run_plan(&config, &plan, adaptive)?;
        }
        Commands::Build { plan, once, dry_run } => {
            run_build(&config, &plan, once, dry_run)?;
        }
    }

    Ok(())
}
```

### Complete Progress File Parsing

```rust
// Source: pulldown-cmark docs
use pulldown_cmark::{Parser, Options, Event, Tag, HeadingLevel};

pub fn parse_progress(content: &str) -> Result<ProgressFile> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(content, options);

    let mut current_section = String::new();
    let mut current_phase = String::new();
    let mut tasks = Vec::new();
    let mut in_heading = false;
    let mut heading_text = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level: HeadingLevel::H2, .. }) => {
                in_heading = true;
                heading_text.clear();
            }
            Event::End(Tag::Heading { level: HeadingLevel::H2, .. }) => {
                in_heading = false;
                current_section = heading_text.trim().to_string();
            }
            Event::Start(Tag::Heading { level: HeadingLevel::H3, .. }) => {
                in_heading = true;
                heading_text.clear();
            }
            Event::End(Tag::Heading { level: HeadingLevel::H3, .. }) => {
                in_heading = false;
                if current_section == "Tasks" {
                    current_phase = heading_text.trim().to_string();
                }
            }
            Event::Text(text) if in_heading => {
                heading_text.push_str(&text);
            }
            Event::TaskListMarker(checked) => {
                // Next text event will be task description
                // Store checked state for pairing
            }
            _ => {}
        }
    }

    // Build ProgressFile from collected data
    Ok(ProgressFile { /* ... */ })
}
```

## Open Questions

1. **Progress file versioning:** Should we include a schema version in the progress file for future migrations? Recommend yes, but format TBD.

2. **Config validation timing:** Validate all paths/values at load time, or lazily when used? Recommend load time for better UX.

3. **RALPH_DONE detection:** Exact string match or regex? Reference implementations use exact match. Recommend exact match for simplicity.

## Sources

### Primary (HIGH confidence)
- [figment 0.10.19 docs](https://docs.rs/figment/latest/figment/) - Layered config API
- [clap 4.5.54 docs](https://docs.rs/clap/latest/clap/) - CLI parsing
- [pulldown-cmark 0.13.0 docs](https://docs.rs/pulldown-cmark/latest/pulldown_cmark/) - Markdown parsing
- [atomicwrites 0.4.4 docs](https://docs.rs/atomicwrites/latest/atomicwrites/) - Atomic file writes
- [directories 6.0.0 docs](https://docs.rs/directories/6.0.0/directories/) - Platform paths

### Secondary (MEDIUM confidence)
- [clap + Figment integration gist](https://gist.github.com/qknight/0ec68e64634e3eb7b9f9d00691f22443) - Integration pattern
- [Building CLI tools with Rust (Clap)](https://dev.to/godofgeeks/building-cli-tools-with-rust-clap-4bo2) - Subcommand patterns
- [rust-atomicwrites GitHub](https://github.com/untitaker/rust-atomicwrites) - Atomic write implementation

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All crates verified via docs.rs, versions confirmed
- Architecture patterns: HIGH - Patterns from official docs and verified examples
- Pitfalls: HIGH - From prior PITFALLS.md research + official docs

**Research date:** 2026-01-17
**Valid until:** 2026-02-17 (stable domain, 30 days)
