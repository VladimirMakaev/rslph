# Phase 3: Planning Command - Research

**Researched:** 2026-01-17
**Domain:** Claude CLI integration, prompt design, vagueness detection, stack detection
**Confidence:** MEDIUM (Claude CLI docs verified, prompt patterns from multiple sources, vagueness detection extrapolated)

## Summary

The `rslph plan` command transforms user ideas into structured progress files by piloting Claude CLI as a subprocess. The existing `ClaudeRunner` from Phase 2 handles subprocess execution, so this phase focuses on:

1. **Prompt System**: Baking default prompts into binary with config override capability
2. **Claude CLI Integration**: Using `-p` flag for headless execution with system prompts
3. **Vagueness Detection**: Simple heuristics to decide when adaptive mode should ask questions
4. **Stack Detection**: Checking manifest files to determine project technology
5. **Progress File Generation**: Using existing `ProgressFile` struct from Phase 1

**Primary recommendation:** Use Claude CLI's `--print` (`-p`) flag with `--system-prompt` for headless operation, embed prompts via `include_str!`, and implement simple word-count + keyword heuristics for vagueness detection.

## Standard Stack

The established libraries/tools for this phase:

### Core (Already in Codebase)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4.x | CLI argument parsing | Already implemented, just extend for plan command |
| figment | 0.10.x | Configuration loading | Already implemented for prompt path config |
| tokio | 1.x | Async subprocess execution | Already implemented via ClaudeRunner |
| pulldown-cmark | 0.x | Markdown parsing | Already in progress.rs for ProgressFile |

### New/Extended

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| include_str! | std | Compile-time prompt embedding | Rust built-in, zero runtime cost |
| serde_json | 1.x | package.json parsing for stack detection | Already a dependency via serde |
| toml | 0.8.x | Cargo.toml parsing for stack detection | Already a dependency via figment |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| include_str! | rust-embed | Overkill for 2 text files; include_str! is simpler |
| serde_json parsing | regex patterns | Less robust for extracting framework info |
| Simple heuristics | ML-based vagueness | Too complex for MVP; heuristics sufficient |

**Installation:**
No new dependencies required. All libraries already in Cargo.toml.

## Architecture Patterns

### Recommended Project Structure (Extensions)

```
src/
├── prompts/
│   ├── mod.rs           # Module with get_plan_prompt(), get_build_prompt()
│   ├── defaults.rs      # include_str! embedded defaults
│   └── loader.rs        # Load override from config path
├── planning/
│   ├── mod.rs           # Module root
│   ├── command.rs       # Main plan command handler
│   ├── stack.rs         # Stack detection logic
│   ├── vagueness.rs     # Vagueness detection heuristics
│   └── personas.rs      # Persona prompts for adaptive mode
└── cli.rs               # Extend Commands enum for plan subcommand

prompts/
├── PROMPT_plan.md       # Baked-in default plan prompt
└── PROMPT_build.md      # Baked-in default build prompt (for Phase 4)
```

### Pattern 1: Prompt Resolution with Config Override

**What:** Load baked-in prompt unless config provides override path
**When to use:** Every plan/build command execution

```rust
// Source: Standard Rust pattern, figment config docs
pub fn get_plan_prompt(config: &Config) -> color_eyre::Result<String> {
    match &config.plan_prompt {
        Some(path) => {
            // User override takes precedence
            std::fs::read_to_string(path)
                .map_err(|e| eyre::eyre!("Failed to read plan prompt from {}: {}", path.display(), e))
        }
        None => {
            // Use baked-in default
            Ok(include_str!("../../prompts/PROMPT_plan.md").to_string())
        }
    }
}
```

### Pattern 2: Claude CLI Headless Execution

**What:** Run Claude in non-interactive mode with system prompt
**When to use:** Both basic and adaptive planning modes

```rust
// Claude CLI flags for headless operation
// Source: https://www.gradually.ai/en/claude-code-commands/
let args = vec![
    "-p".to_string(),                          // Print mode (headless, exit after response)
    "--system-prompt".to_string(),              // Custom system prompt
    system_prompt,
    "--output-format".to_string(),              // Output format
    "text".to_string(),                         // Plain text for progress file
    user_input,                                  // The user's plan/idea
];

let mut runner = ClaudeRunner::spawn(
    &config.claude_path,
    &args,
    working_dir,
).await?;
```

### Pattern 3: Stack Detection via Manifest Files

**What:** Check for project manifest files to determine stack
**When to use:** Auto-detect testing strategy in planning

```rust
// Source: Common pattern from specfy/stack-analyser
pub struct DetectedStack {
    pub language: Language,
    pub framework: Option<String>,
    pub test_runner: Option<String>,
    pub package_manager: Option<String>,
}

pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Unknown,
}

pub fn detect_stack(project_dir: &Path) -> DetectedStack {
    // Check in priority order
    if project_dir.join("Cargo.toml").exists() {
        return detect_rust_stack(project_dir);
    }
    if project_dir.join("package.json").exists() {
        return detect_node_stack(project_dir);
    }
    if project_dir.join("pyproject.toml").exists() || project_dir.join("setup.py").exists() {
        return detect_python_stack(project_dir);
    }
    if project_dir.join("go.mod").exists() {
        return detect_go_stack(project_dir);
    }
    DetectedStack::unknown()
}
```

### Pattern 4: Vagueness Detection Heuristics

**What:** Simple rules to determine if input is too vague for basic mode
**When to use:** Adaptive mode decision making

```rust
// Source: Extrapolated from ambiguity detection research
pub struct VaguenessScore {
    pub score: f32,         // 0.0 = very specific, 1.0 = very vague
    pub reasons: Vec<String>,
}

pub fn assess_vagueness(input: &str) -> VaguenessScore {
    let mut score = 0.0;
    let mut reasons = Vec::new();

    let word_count = input.split_whitespace().count();

    // Very short inputs are vague
    if word_count < 5 {
        score += 0.4;
        reasons.push("Very short input".to_string());
    } else if word_count < 15 {
        score += 0.2;
        reasons.push("Short input".to_string());
    }

    // Check for specificity indicators (reduce vagueness)
    let specificity_markers = [
        "must", "should", "requires", "needs to",
        "using", "with", "implement", "add", "create",
        "endpoint", "api", "database", "component", "module",
    ];
    let has_specificity = specificity_markers.iter()
        .any(|m| input.to_lowercase().contains(m));
    if has_specificity {
        score -= 0.2;
    }

    // Check for vague indicators (increase vagueness)
    let vague_markers = [
        "something", "somehow", "maybe", "possibly",
        "kind of", "sort of", "like a", "basically",
        "stuff", "things", "whatever",
    ];
    for marker in vague_markers {
        if input.to_lowercase().contains(marker) {
            score += 0.15;
            reasons.push(format!("Contains vague term: {}", marker));
        }
    }

    // Questions without specifics are vague
    if input.contains('?') && word_count < 10 {
        score += 0.2;
        reasons.push("Short question".to_string());
    }

    VaguenessScore {
        score: score.clamp(0.0, 1.0),
        reasons,
    }
}
```

### Pattern 5: Multi-Turn Adaptive Mode (Personas)

**What:** Sequential Claude invocations for requirements clarifier and testing strategist
**When to use:** Adaptive mode (`--adaptive` flag)

```rust
// Adaptive mode workflow
pub async fn run_adaptive_planning(
    input: &str,
    config: &Config,
    working_dir: &Path,
) -> Result<ProgressFile> {
    let stack = detect_stack(working_dir);

    // Step 1: Requirements clarification (persona 1)
    let clarifier_prompt = format!(
        "{}\n\n## Project Stack\n{}\n\n## User Input\n{}",
        REQUIREMENTS_CLARIFIER_PERSONA,
        stack.to_summary(),
        input
    );
    let questions = run_claude_headless(&config.claude_path, &clarifier_prompt).await?;

    // Step 2: Get user answers (interactive or skip if none needed)
    let answers = if questions.trim().is_empty() {
        String::new()
    } else {
        prompt_user_for_answers(&questions)?
    };

    // Step 3: Testing strategy (persona 2)
    let testing_prompt = format!(
        "{}\n\n## Project Stack\n{}\n\n## Requirements\n{}\n{}",
        TESTING_STRATEGIST_PERSONA,
        stack.to_summary(),
        input,
        answers
    );
    let testing_strategy = run_claude_headless(&config.claude_path, &testing_prompt).await?;

    // Step 4: Final planning with context
    let plan_prompt = format!(
        "{}\n\n## Stack\n{}\n\n## Requirements\n{}\n\n## Testing Strategy\n{}",
        get_plan_prompt(config)?,
        stack.to_summary(),
        format!("{}\n{}", input, answers),
        testing_strategy
    );
    let progress_md = run_claude_headless(&config.claude_path, &plan_prompt).await?;

    ProgressFile::parse(&progress_md)
}
```

### Anti-Patterns to Avoid

- **Blocking on user input in basic mode:** Basic mode should NEVER ask questions; just do best-effort structuring
- **Complex ML for vagueness:** Simple heuristics are sufficient for v1; don't over-engineer
- **Parsing Claude output as JSON:** Use markdown output and parse with pulldown-cmark (already implemented)
- **Hardcoding stack detection:** Make it extensible for future language support

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Prompt embedding | Load at runtime | `include_str!` macro | Compile-time, no file I/O, no missing file errors |
| Markdown parsing | Regex extraction | pulldown-cmark (exists) | Already implemented, handles edge cases |
| Config merging | Manual precedence | figment (exists) | Already implemented, handles all layers |
| Subprocess streaming | Channel-based | ClaudeRunner (exists) | Phase 2 already implemented this |
| JSON manifest parsing | Custom parsers | serde_json | Standard, handles all JSON edge cases |

**Key insight:** Most infrastructure already exists from Phase 1-2. This phase is primarily about prompt design and orchestrating existing components.

## Common Pitfalls

### Pitfall 1: Forgetting --print Mode Behavior

**What goes wrong:** Running Claude interactively instead of headlessly
**Why it happens:** Default claude command enters REPL
**How to avoid:** Always use `-p` flag for automated execution
**Warning signs:** Process hangs waiting for input

### Pitfall 2: Prompt Path Resolution

**What goes wrong:** Relative paths in config don't resolve correctly
**Why it happens:** Working directory may differ from config file location
**How to avoid:** Resolve paths relative to config file location, or require absolute paths
**Warning signs:** "File not found" errors for valid-looking paths

### Pitfall 3: Stack Detection False Positives

**What goes wrong:** Detecting wrong stack (e.g., Python project with package.json for tooling)
**Why it happens:** Multiple manifest files exist
**How to avoid:** Use priority ordering, check for primary indicators
**Warning signs:** Incorrect testing strategy generated

### Pitfall 4: Vagueness Threshold Too Strict

**What goes wrong:** Asking clarifying questions for reasonably specific inputs
**Why it happens:** Threshold set too low
**How to avoid:** Start with 0.6 threshold, tune based on user feedback
**Warning signs:** Users complaining about unnecessary questions in adaptive mode

### Pitfall 5: Progress File Parse Errors

**What goes wrong:** Claude generates markdown that doesn't parse into ProgressFile
**Why it happens:** Prompt doesn't constrain output format enough
**How to avoid:** Include explicit format example in PROMPT_plan.md, validate output
**Warning signs:** Parse errors on generated files

## Code Examples

### Example 1: PROMPT_plan.md Structure

```markdown
# Planning Assistant

You are a planning assistant that transforms user ideas into structured task lists.

## Your Role

Given a user's idea or plan, you will:
1. Analyze the requirements
2. Break down into discrete, actionable tasks
3. Organize tasks into logical phases
4. Generate a testing strategy based on the project stack
5. Output a structured progress file

## Output Format

You MUST output a valid progress file in this exact format:

# Progress: [Plan Name]

## Status

In Progress

## Analysis

[Brief analysis of the requirements and approach]

## Tasks

### Phase 1: [Phase Name]

- [ ] Task 1 description
- [ ] Task 2 description

### Phase 2: [Phase Name]

- [ ] Task 1 description

## Testing Strategy

[Based on detected stack, specify:]
- Unit testing approach
- Integration testing approach
- Type checking (if applicable)
- Linting/static analysis

## Completed This Iteration

[Leave empty]

## Recent Attempts

[Leave empty]

## Iteration Log

| Iteration | Started | Duration | Tasks Completed | Notes |
|-----------|---------|----------|-----------------|-------|

## Guidelines

1. Each task should be completable in 1-2 iterations
2. Tasks should be specific and actionable
3. Include testing tasks for each feature
4. Order tasks by dependency (earlier phases first)
5. Use imperative verbs: "Add", "Implement", "Create", "Fix"
```

### Example 2: Stack Detection Implementation

```rust
// src/planning/stack.rs
use std::path::Path;

#[derive(Debug, Clone)]
pub struct DetectedStack {
    pub language: Language,
    pub framework: Option<String>,
    pub test_runner: Option<String>,
    pub type_checker: Option<String>,
    pub linter: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Unknown,
}

impl DetectedStack {
    pub fn to_summary(&self) -> String {
        let mut parts = vec![format!("Language: {:?}", self.language)];
        if let Some(ref fw) = self.framework {
            parts.push(format!("Framework: {}", fw));
        }
        if let Some(ref tr) = self.test_runner {
            parts.push(format!("Test Runner: {}", tr));
        }
        if let Some(ref tc) = self.type_checker {
            parts.push(format!("Type Checker: {}", tc));
        }
        if let Some(ref l) = self.linter {
            parts.push(format!("Linter: {}", l));
        }
        parts.join("\n")
    }
}

fn detect_rust_stack(dir: &Path) -> DetectedStack {
    DetectedStack {
        language: Language::Rust,
        framework: None, // Could parse Cargo.toml for frameworks
        test_runner: Some("cargo test".to_string()),
        type_checker: Some("rustc".to_string()),
        linter: Some("clippy".to_string()),
    }
}

fn detect_node_stack(dir: &Path) -> DetectedStack {
    let pkg_path = dir.join("package.json");
    let mut stack = DetectedStack {
        language: Language::JavaScript,
        framework: None,
        test_runner: None,
        type_checker: None,
        linter: None,
    };

    if let Ok(content) = std::fs::read_to_string(&pkg_path) {
        if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
            // Detect TypeScript
            if pkg.get("devDependencies")
                .and_then(|d| d.get("typescript"))
                .is_some()
            {
                stack.language = Language::TypeScript;
                stack.type_checker = Some("tsc".to_string());
            }

            // Detect test runner
            if pkg.get("devDependencies").and_then(|d| d.get("jest")).is_some() {
                stack.test_runner = Some("jest".to_string());
            } else if pkg.get("devDependencies").and_then(|d| d.get("vitest")).is_some() {
                stack.test_runner = Some("vitest".to_string());
            }

            // Detect linter
            if pkg.get("devDependencies").and_then(|d| d.get("eslint")).is_some() {
                stack.linter = Some("eslint".to_string());
            }

            // Detect framework
            if pkg.get("dependencies").and_then(|d| d.get("react")).is_some() {
                stack.framework = Some("React".to_string());
            } else if pkg.get("dependencies").and_then(|d| d.get("next")).is_some() {
                stack.framework = Some("Next.js".to_string());
            }
        }
    }

    stack
}
```

### Example 3: Planning Command Handler

```rust
// src/planning/command.rs
use crate::config::Config;
use crate::progress::ProgressFile;
use crate::subprocess::ClaudeRunner;
use crate::planning::{detect_stack, assess_vagueness, get_plan_prompt};
use std::path::Path;
use tokio_util::sync::CancellationToken;

pub async fn run_plan_command(
    input: &str,
    adaptive: bool,
    config: &Config,
    working_dir: &Path,
    cancel_token: CancellationToken,
) -> color_eyre::Result<ProgressFile> {
    // Detect project stack for testing strategy
    let stack = detect_stack(working_dir);

    if adaptive {
        // Adaptive mode: check vagueness, potentially ask questions
        let vagueness = assess_vagueness(input);
        if vagueness.score > 0.5 {
            // Run requirements clarifier persona
            // ... (multi-turn conversation)
        }
        // Run testing strategist persona
        // ...
    }

    // Build the planning prompt
    let system_prompt = get_plan_prompt(config)?;
    let full_input = format!(
        "## Detected Stack\n{}\n\n## User Request\n{}",
        stack.to_summary(),
        input
    );

    // Execute Claude in headless mode
    let args = vec![
        "-p".to_string(),
        "--system-prompt".to_string(),
        system_prompt,
        full_input,
    ];

    let mut runner = ClaudeRunner::spawn(
        &config.claude_path,
        &args,
        working_dir,
    ).await?;

    // Collect output
    let output = runner.run_to_completion(cancel_token).await?;
    let response_text: String = output.iter()
        .filter_map(|line| match line {
            crate::subprocess::OutputLine::Stdout(s) => Some(s.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Parse into ProgressFile
    ProgressFile::parse(&response_text)
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Interactive Claude sessions | `--print` mode for headless | Claude Code 2.0+ | Enables automated pipelines |
| JSON output parsing | Markdown with pulldown-cmark | Current | More flexible, human-readable |
| Complex ML vagueness detection | Simple heuristics | N/A | Sufficient for MVP, easier to tune |
| Hardcoded prompts | `include_str!` + config override | Rust pattern | Binary works standalone, but configurable |

**Deprecated/outdated:**
- Using `--prompt` flag alone without `-p`: This enters interactive mode
- Expecting JSON schema enforcement in all Claude CLI versions: `--json-schema` is newer feature

## Open Questions

Things that couldn't be fully resolved:

1. **Exact Claude CLI Version Compatibility**
   - What we know: `-p` and `--system-prompt` work in recent versions
   - What's unclear: Minimum supported version
   - Recommendation: Document requirements, add version check in CLI

2. **Adaptive Mode User Interaction**
   - What we know: Need to collect answers to clarifying questions
   - What's unclear: Best UX for question/answer in terminal
   - Recommendation: Simple numbered questions, user types answers, Enter to submit

3. **Progress File Validation**
   - What we know: Claude may generate non-conforming markdown
   - What's unclear: How strict to be with validation
   - Recommendation: Parse with fallbacks, warn on issues, don't fail hard

4. **Testing Strategy Depth**
   - What we know: Need multi-layer strategy (unit, type, lint, e2e)
   - What's unclear: How much detail to include in generated plan
   - Recommendation: Keep high-level, let build phase add specific test tasks

## Sources

### Primary (HIGH confidence)
- [Claude Code Commands Reference](https://www.gradually.ai/en/claude-code-commands/) - CLI flags including `-p`, `--system-prompt`, `--output-format`
- [ClaudeCode.CLI Elixir docs](https://hexdocs.pm/claude_code/ClaudeCode.CLI.html) - Session management, build_command patterns
- Existing codebase: `config.rs`, `progress.rs`, `subprocess/runner.rs` - Implementation patterns

### Secondary (MEDIUM confidence)
- [kylemclaren/ralph](https://github.com/kylemclaren/ralph) - PRD.json pattern, maxIterations config
- [yy/wiggum](https://github.com/yy/wiggum) - LOOP-PROMPT.md and TASKS.md two-file pattern
- [portableralph](https://github.com/aaron777collins/portableralph) - Progress file format, PROMPT_plan.md concept
- [specfy/stack-analyser](https://github.com/specfy/stack-analyser) - Stack detection patterns

### Tertiary (LOW confidence, extrapolated)
- [Ambiguity detection research](https://shanechang.com/p/training-llms-smarter-clarifying-ambiguity-assumptions/) - Heuristics for vagueness
- [Task decomposition](https://idratherbewriting.com/ai/prompt-engineering-task-decomposition.html) - Prompt structuring patterns
- [PJFP Ralph Wiggum Guide](https://pjfp.com/what-is-the-ralph-wiggum-loop-in-programming-ultimate-guide-to-ai-powered-iterative-coding/) - Completion detection patterns

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Using existing dependencies, verified patterns
- Architecture: MEDIUM - Patterns extrapolated from multiple sources, not verified end-to-end
- Claude CLI integration: MEDIUM - Flags verified, but exact behavior untested
- Vagueness detection: LOW - Heuristics designed from principles, no prior art in this exact form
- Stack detection: MEDIUM - Pattern from stack-analyser, implementation details inferred

**Research date:** 2026-01-17
**Valid until:** 30 days (CLI flags stable, prompt patterns may evolve)
