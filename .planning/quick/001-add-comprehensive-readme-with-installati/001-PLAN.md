---
type: quick
plan: 001
wave: 1
depends_on: []
files_modified: [README.md]
autonomous: true

must_haves:
  truths:
    - "User can understand what rslph is and why to use it"
    - "User can install rslph from source"
    - "User can run basic plan and build commands"
    - "User can configure rslph via config file or environment"
  artifacts:
    - path: "README.md"
      provides: "Complete project documentation"
      min_lines: 150
---

<objective>
Create comprehensive README.md with project overview, installation instructions, usage examples, and configuration documentation.

Purpose: Enable new users to understand, install, and use rslph without referring to source code
Output: README.md at repository root with all essential documentation
</objective>

<context>
@.planning/PROJECT.md
@Cargo.toml
@src/config.rs
</context>

<tasks>

<task type="auto">
  <name>Task 1: Create comprehensive README.md</name>
  <files>README.md</files>
  <action>
Create README.md at repository root with the following sections:

1. **Header and Badge Area**
   - Project name "rslph" with tagline "Ralph Wiggum Loop - Autonomous AI Coding Agent"
   - Brief one-paragraph description explaining the core concept

2. **Features Section**
   - Fresh context per iteration (prevents context pollution)
   - Progress file as memory (accumulated learnings)
   - Rich TUI with live output and collapsible threads
   - Git and Sapling VCS auto-commit per iteration
   - Configurable via TOML, environment variables, or CLI
   - Built-in evaluation framework for benchmarking

3. **Prerequisites**
   - Rust toolchain (cargo)
   - Claude CLI installed and authenticated (`claude` command available)
   - Git or Sapling VCS

4. **Installation**
   - Clone repository
   - Build with `cargo build --release`
   - Optionally add to PATH or symlink

5. **Quick Start**
   - Show minimal example: `rslph plan "create a hello world program" && rslph build progress.md`
   - Explain what happens in each step

6. **Commands Reference**
   - `rslph plan <plan>` - Transform idea into structured progress file
     - `--adaptive` flag for clarifying questions
     - `--mode` flag for prompt modes (basic, gsd, gsd_tdd)
     - `--no-tui` for plain output
   - `rslph build <progress.md>` - Execute tasks iteratively
     - `--once` for single iteration
     - `--dry-run` for preview
     - `--max-iterations` override
   - `rslph eval <project>` - Run evaluation benchmarks
     - `--list` to show available projects (calculator, fizzbuzz)
     - `--trials` for multiple runs
     - `--modes` for comparing prompt modes

7. **Configuration**
   - Config file location: `~/.config/rslph/config.toml` (XDG-compliant)
   - Environment variables with `RSLPH_` prefix
   - Precedence: defaults < config file < environment < CLI
   - Document key config options with example TOML:
     ```toml
     claude_path = "claude"
     max_iterations = 20
     tui_enabled = true
     iteration_timeout = 600
     prompt_mode = "basic"
     ```

8. **Prompt Modes**
   - `basic` - Default, current rslph prompts
   - `gsd` - GSD-adapted prompts with XML structure
   - `gsd_tdd` - Strict test-driven development flow

9. **TUI Controls**
   - Key bindings: q (quit), j/k (scroll), t (toggle thinking), c (conversation view)
   - Page navigation: PageUp/PageDown

10. **How It Works**
    - Brief explanation of the Ralph Wiggum Loop pattern
    - Progress file is the memory between iterations
    - Each iteration: fresh context + progress file + codebase
    - RALPH_DONE marker signals completion

11. **License**
    - Placeholder or actual license reference

Use clear markdown formatting with code blocks for commands and examples.
  </action>
  <verify>
    - File exists: `ls README.md`
    - Has expected sections: `grep -E "^##" README.md` shows all major sections
    - Code blocks render: visual inspection of markdown
    - Line count appropriate: `wc -l README.md` shows 150+ lines
  </verify>
  <done>README.md exists at repository root with all documented sections, properly formatted markdown, and actionable installation/usage instructions</done>
</task>

</tasks>

<verification>
- README.md exists at repository root
- All major sections present (Features, Installation, Commands, Configuration)
- Code examples use correct command syntax matching --help output
- Config options match src/config.rs defaults
</verification>

<success_criteria>
- New user can install rslph following README instructions
- All CLI commands documented with key flags
- Configuration options documented with examples
- README renders correctly on GitHub
</success_criteria>

<output>
After completion, create `.planning/quick/001-add-comprehensive-readme-with-installati/001-SUMMARY.md`
</output>
