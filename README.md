# rslph

[![CI](https://github.com/VladimirMakaev/rslph/actions/workflows/ci.yml/badge.svg)](https://github.com/VladimirMakaev/rslph/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/rslph.svg)](https://crates.io/crates/rslph)
[![License](https://img.shields.io/crates/l/rslph.svg)](https://crates.io/crates/rslph)

**Ralph Wiggum Loop - Autonomous AI Coding Agent**

rslph is a Rust CLI application that implements the Ralph Wiggum Loop pattern: an autonomous AI coding agent that breaks down complex tasks into iterative steps and executes them using Claude AI. Each iteration starts with fresh context to prevent context pollution while preserving accumulated learnings through a persistent progress file. Features a rich TUI for real-time monitoring of execution.

## Features

- **Fresh Context Per Iteration** - Each iteration resets Claude's context, preventing context window exhaustion while maintaining progress through the persistent progress file
- **Progress File as Memory** - Accumulated learnings survive across iterations, allowing Claude to learn from past attempts and avoid repeating failures
- **Rich TUI Interface** - Real-time monitoring with live Claude output, collapsible message threads, keyboard navigation, and Claude Code-style visual design
- **VCS Auto-Commit** - Automatic Git or Sapling commits after each iteration, creating a clear audit trail
- **Flexible Configuration** - Configure via TOML config file, environment variables, or CLI flags with clear precedence rules
- **Built-in Evaluation Framework** - Benchmark agent performance with hidden test suites and multi-trial statistics
- **Multiple Prompt Modes** - Choose between basic rslph prompts, GSD-adapted prompts, or strict test-driven development flow
- **Token Tracking** - Monitor Claude API token usage (input, output, cache creation, cache reads) across iterations

## Prerequisites

Before installing rslph, ensure you have:

- **Rust toolchain** - Install from [rustup.rs](https://rustup.rs/)
- **Claude CLI** - Install and authenticate the Claude CLI tool
  ```bash
  # Install Claude CLI (instructions at https://github.com/anthropics/anthropic-cli)
  # Authenticate
  claude auth
  ```
- **Git or Sapling VCS** - For auto-commit functionality

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/VladimirMakaev/rslph.git
cd rslph

# Build release binary
cargo build --release

# Binary will be at: target/release/rslph

# Optional: Add to PATH or create symlink
sudo ln -s $(pwd)/target/release/rslph /usr/local/bin/rslph
```

### From crates.io

```bash
cargo install rslph
```

## Quick Start

1. **Create a plan from an idea:**
   ```bash
   rslph plan "create a hello world program in Python"
   ```
   This generates a structured `progress.md` file with tasks.

2. **Execute the plan autonomously:**
   ```bash
   rslph build progress.md
   ```
   Watch in the TUI as Claude iterates through tasks, commits changes, and completes the plan.

**What happens:**
- Claude reads the plan and current codebase
- Executes the next task, updates code
- Commits changes to Git/Sapling
- Updates `progress.md` with results
- Repeats until `RALPH_DONE` marker appears or max iterations reached
- Each iteration has fresh context but sees accumulated progress

## Commands Reference

### `rslph plan`

Transform an idea into a structured progress file that the build command can execute.

```bash
rslph plan <PLAN> [OPTIONS]
```

**Arguments:**
- `<PLAN>` - Path to plan file or inline text describing your goal

**Options:**
- `--adaptive` - Enable adaptive mode with clarifying questions for detailed plans
- `--mode <MODE>` - Prompt mode: `basic`, `gsd`, or `gsd_tdd` (default: basic)
- `--no-tui` - Disable TUI and use plain output
- `--config <CONFIG>` - Override config file path
- `--claude-path <PATH>` - Override Claude CLI path
- `--max-iterations <N>` - Override max iterations setting

**Examples:**
```bash
# Inline plan
rslph plan "add user authentication to my web app"

# From file
rslph plan idea.txt

# With adaptive mode for complex plans
rslph plan --adaptive complex-feature.txt

# Using GSD test-driven mode
rslph plan --mode gsd_tdd "implement a calculator library"
```

### `rslph build`

Execute tasks from a progress file iteratively with fresh context.

```bash
rslph build <PLAN> [OPTIONS]
```

**Arguments:**
- `<PLAN>` - Path to progress file (typically `progress.md`)

**Options:**
- `--once` - Run only a single iteration (for debugging)
- `--dry-run` - Preview what would happen without executing
- `--max-iterations <N>` - Override max iterations (default: 20)
- `--mode <MODE>` - Prompt mode: `basic`, `gsd`, or `gsd_tdd`
- `--no-tui` - Disable TUI interface
- `--config <CONFIG>` - Override config file path
- `--claude-path <PATH>` - Override Claude CLI path

**Examples:**
```bash
# Normal execution with TUI
rslph build progress.md

# Single iteration for testing
rslph build --once progress.md

# Preview without execution
rslph build --dry-run progress.md

# With custom iteration limit
rslph build --max-iterations 50 progress.md
```

**TUI Controls:**
- `q` - Quit
- `j`/`k` - Scroll down/up
- `PageUp`/`PageDown` - Page navigation
- `t` - Toggle thinking blocks collapsed/expanded
- `c` - Toggle conversation view (split screen with all messages)

### `rslph eval`

Run evaluation benchmarks in isolated environments with hidden test suites.

```bash
rslph eval [PROJECT] [OPTIONS]
```

**Arguments:**
- `[PROJECT]` - Project to evaluate (built-in or directory path)

**Options:**
- `--list` - List available built-in projects
- `--trials <N>` - Number of independent trials to run (default: 1)
- `--modes <MODES>` - Comma-separated modes to compare: `basic,gsd,gsd_tdd`
- `--keep` - Keep temporary workspace after completion (for debugging)
- `--max-iterations <N>` - Override max iterations
- `--no-tui` - Disable TUI dashboard
- `--config <CONFIG>` - Override config file path

**Built-in Projects:**
- `calculator` - Simple calculator with arithmetic operations
- `fizzbuzz` - FizzBuzz implementation with test cases

**Examples:**
```bash
# List available projects
rslph eval --list

# Single evaluation
rslph eval calculator

# Multiple trials for statistics
rslph eval --trials 5 fizzbuzz

# Compare prompt modes
rslph eval --modes basic,gsd,gsd_tdd calculator

# Evaluate custom project with tests
rslph eval ./my-project --trials 3
```

**Output:**
- Results saved to JSON: `~/.rslph/evals/eval-results-{project}-{date}.json`
- Statistics: pass rate, avg tokens, iteration count with min/max/mean/variance
- Multi-mode comparison tables when using `--modes`

## Configuration

### Config File Location

rslph uses XDG-compliant configuration:

```
~/.config/rslph/config.toml
```

Create this file to customize default behavior.

### Example Configuration

```toml
# Path to Claude CLI executable (resolved via 'which' if relative)
claude_path = "claude"

# Maximum iterations before stopping (default: 20)
max_iterations = 20

# Enable TUI mode by default (default: true)
tui_enabled = true

# Number of recent messages to display in TUI (default: 10)
tui_recent_messages = 10

# Timeout in seconds for each iteration (default: 600)
iteration_timeout = 600

# Maximum retries for timed-out iterations (default: 3)
timeout_retries = 3

# Default prompt mode (basic, gsd, gsd_tdd)
prompt_mode = "basic"

# Directory for eval workspaces and results (default: ~/.rslph/evals)
eval_dir = "~/.rslph/evals"

# Notification interval - every N iterations (default: 10)
notify_interval = 10

# Number of recent threads to display (default: 5)
recent_threads = 5

# Shell for notify script execution (default: /bin/sh)
notify_shell = "/bin/sh"

# Optional: Override plan/build prompt files
# plan_prompt = "/path/to/custom_plan_prompt.md"
# build_prompt = "/path/to/custom_build_prompt.md"
```

### Environment Variables

Override config with environment variables using the `RSLPH_` prefix:

```bash
export RSLPH_MAX_ITERATIONS=50
export RSLPH_TUI_ENABLED=false
export RSLPH_PROMPT_MODE=gsd_tdd
export RSLPH_CLAUDE_PATH=/custom/path/to/claude
```

### Configuration Precedence

Settings are applied in this order (later overrides earlier):

1. **Default values** (hardcoded in the application)
2. **Config file** (`~/.config/rslph/config.toml`)
3. **Environment variables** (`RSLPH_*`)
4. **CLI flags** (highest precedence)

## Prompt Modes

rslph supports three prompt engineering approaches:

### `basic` (Default)

The original rslph prompts focused on iterative task execution with progress file updates.

- **Best for:** General-purpose autonomous coding tasks
- **Backward compatible** with existing workflows
- **Use when:** You want tried-and-tested prompts

### `gsd` (Get Shit Done)

GSD-adapted prompts with structured XML format and "must-haves" specification.

- **Best for:** Complex multi-phase projects with clear requirements
- **Features:** Explicit success criteria, structured output, must-haves tracking
- **Use when:** You have well-defined requirements and want structured execution

### `gsd_tdd` (Test-Driven Development)

Strict test-driven development flow with three-phase cycle (write test → implement → refactor).

- **Best for:** Projects requiring high code quality and test coverage
- **Features:** Enforced red-green-refactor cycle, automatic test infrastructure setup
- **Use when:** Building libraries, APIs, or critical functionality requiring tests

**Select mode:**
```bash
# Via CLI flag
rslph plan --mode gsd_tdd "build a calculator library"

# Via config file
# config.toml: prompt_mode = "gsd_tdd"

# Via environment
export RSLPH_PROMPT_MODE=gsd
```

## How It Works

The **Ralph Wiggum Loop** is named after The Simpsons character who persists despite setbacks. It emphasizes iterative self-correction over single-pass perfection.

### The Loop Pattern

1. **Plan Phase:**
   - Claude transforms your idea into a structured progress file
   - Tasks are broken down into concrete, executable steps
   - Success criteria and verification steps are defined

2. **Build Phase (iteration loop):**
   - **Read:** Claude reads the progress file + current codebase
   - **Execute:** Completes the next task, modifying code as needed
   - **Commit:** Changes are committed to VCS with descriptive message
   - **Update:** Progress file is updated with results and learnings
   - **Check:** If `RALPH_DONE` marker appears, loop exits successfully
   - **Repeat:** Fresh context for next iteration (no context pollution)

### Key Insight: Progress File as Memory

The progress file is the sole memory mechanism between iterations:

- Each iteration Claude starts with **fresh context** (no accumulated tokens)
- Progress file contains **accumulated learnings** (what was tried, what worked, what failed)
- Claude reads this file every iteration, learning from past attempts
- Best of both worlds: fresh context + persistent memory

### VCS Integration

After each successful iteration, rslph automatically commits:

```
git commit -m "Iteration 3: Implemented user authentication handler"
```

This creates an audit trail and allows easy rollback if needed.

## Project Structure

After running rslph, your project might look like:

```
my-project/
├── progress.md          # Generated by 'plan', updated by 'build'
├── src/                # Your code (created/modified by Claude)
│   ├── main.py
│   └── auth.py
└── .git/               # VCS commits after each iteration
```

## Advanced Usage

### Custom Prompt Files

Override default prompts by setting paths in config:

```toml
plan_prompt = "~/.config/rslph/my_plan_prompt.md"
build_prompt = "~/.config/rslph/my_build_prompt.md"
```

This allows power users to customize Claude's instructions.

### Evaluation Workflow

Benchmark your prompt engineering:

```bash
# Compare all modes across 5 trials
rslph eval --modes basic,gsd,gsd_tdd --trials 5 calculator

# Analyze results JSON
cat ~/.rslph/evals/eval-results-calculator-multimode-*.json | jq
```

Results include:
- Pass rate (tests passed / total tests)
- Token usage statistics (mean, variance, min, max)
- Iteration count statistics
- Per-trial detailed logs

### Debugging Failed Builds

```bash
# Run single iteration to see exact failure
rslph build --once progress.md

# Keep workspace for inspection
rslph eval --keep my-project

# Disable TUI for clean logs
rslph build --no-tui progress.md 2>&1 | tee build.log
```

## Troubleshooting

### Claude CLI hangs

If the Claude CLI hangs, this is a known issue. Current workaround:

- rslph uses `--internet` flag internally to prevent hanging
- Ensure your Claude CLI is up to date
- Check authentication: `claude auth`

### Timeout errors

If iterations timeout (default 600s):

```toml
# Increase timeout in config.toml
iteration_timeout = 1200  # 20 minutes
timeout_retries = 5
```

### VCS not auto-committing

Ensure Git or Sapling is initialized:

```bash
git init
# or
sl init
```

rslph auto-detects VCS and commits after each iteration.

## License

[License information - add your chosen license here]

## Contributing

Contributions welcome! This project uses:

- **Language:** Rust (edition 2021)
- **TUI:** ratatui
- **Config:** figment (TOML)
- **CLI:** clap
- **Testing:** E2E tests with fake Claude simulation

## Acknowledgments

- **Pattern origin:** Geoffrey Huntley's Ralph Wiggum Loop concept (late 2025)
- **Reference implementation:** [portableralph](https://github.com/aaron777collins/portableralph)
- **Claude AI:** Anthropic's Claude powers the autonomous execution

---

**Built with Rust. Powered by Claude. Inspired by Ralph Wiggum's persistence.**
