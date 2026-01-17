# Feature Landscape: Autonomous AI Coding CLI Tools

**Domain:** CLI-based autonomous coding agents (Ralph Wiggum Loop implementations)
**Researched:** 2026-01-17
**Confidence:** HIGH (multiple authoritative sources cross-referenced)

## Table Stakes

Features users expect from autonomous CLI coding tools. Missing any of these means users will leave for competitors.

### Core Loop Execution

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Autonomous iteration loop** | Core value proposition - runs until task complete | Medium | Must support configurable max iterations, completion detection |
| **Progress persistence** | Users expect to resume after interruption | Low | File-based state (progress.txt, JSON) standard approach |
| **Task/story tracking** | Need to see what's done vs pending | Low | JSON or structured format tracking task status |
| **Completion detection** | Loop must know when to stop | Medium | String matching ("DONE", `<promise>COMPLETE</promise>`) or test pass |
| **Git integration** | Auto-commit changes for rollback safety | Low | Every iteration should commit; enables undo |
| **Max iteration limits** | Safety against runaway loops | Low | Critical safety mechanism - prevents infinite cost |

### Configuration & Customization

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Config file support** | Avoid repeating CLI flags | Low | YAML/TOML/JSON; hierarchical precedence (CLI > env > config) |
| **CLI flag overrides** | Quick one-off changes | Low | All config options overridable via flags |
| **Environment variables** | CI/CD and secrets management | Low | RALPH_* prefix convention |
| **Custom prompts** | Project-specific instructions | Low | PROMPT.md or configurable path |
| **Agent selection** | Support multiple AI backends | Medium | Claude Code, Amp, OpenCode, Codex minimum |

### User Experience

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Real-time output streaming** | Users need to see what's happening | Medium | Live streaming from subprocess crucial for trust |
| **Status display** | Quick view of current state | Low | Iteration count, current task, pass/fail status |
| **Ctrl+C graceful shutdown** | Standard CLI expectation | Low | Clean exit, save state, preserve progress |
| **Dry-run mode** | Preview before executing | Low | `--dry-run` shows what would happen |
| **Single iteration mode** | Human review between iterations | Low | `--once` flag for step-by-step execution |

### Task Management

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **PRD/task file parsing** | Structured input for autonomous work | Medium | JSON format (prd.json) with stories, acceptance criteria |
| **Priority ordering** | Work on highest priority first | Low | Simple priority field in task structure |
| **Acceptance criteria** | Know when task is done | Low | Clear pass/fail conditions per task |
| **Learnings capture** | Compound knowledge across iterations | Low | Append insights to progress.txt |

## Differentiators

Features that provide competitive advantage. Not expected but highly valued.

### TUI/Terminal Interface

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Rich TUI with status bar** | Professional feel, better UX than raw output | High | ratatui/bubbletea patterns; status bar, panels |
| **Collapsible output threads** | Manage verbosity for long runs | High | Expand/collapse per-iteration output |
| **Live progress indicators** | Visual feedback during long operations | Medium | Spinners, progress bars, iteration counters |
| **Keyboard navigation** | Power user efficiency | Medium | Vim-like bindings, quick commands |
| **Multi-pane layout** | View multiple contexts simultaneously | High | Task list, current output, progress summary |

### Advanced Loop Control

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Verification integration** | Auto-run tests/lints between iterations | Medium | Run `cargo test`, `npm test`, or custom commands |
| **Acceptance testing** | Stop when acceptance criteria pass | Medium | Parse test output for pass/fail determination |
| **Checkpoint/rollback** | Undo to specific iteration | Medium | Git tag or checkpoint system per iteration |
| **Sleep intervals** | Rate limiting, API cost management | Low | Configurable delay between iterations |
| **Circuit breaker** | Stop after repeated failures | Medium | Prevent wasted iterations on stuck tasks |

### Lifecycle Hooks

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **onStart hook** | Setup before loop begins | Low | Run scripts/commands at loop start |
| **onIteration hook** | Custom logic each iteration | Low | Post-iteration scripts (linting, testing) |
| **onComplete hook** | Finalization when done | Low | Cleanup, notifications, final commits |
| **onFailure hook** | Handle errors gracefully | Low | Error reporting, recovery actions |
| **Webhook notifications** | Remote monitoring, Slack/Discord alerts | Medium | POST to URL with status JSON |

### Multi-Agent & Parallelism

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Parallel research agents** | Faster PRD generation with specialized agents | High | 3-5 parallel agents for different concerns |
| **Git worktree support** | Parallel feature development | High | Multiple branches running simultaneously |
| **Multi-phase sequencing** | Structured progression through phases | Medium | Explicit checkpoints between phases |

### Context & Memory

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Session management** | Resume previous sessions | Medium | SQLite or file-based conversation history |
| **Auto-compaction** | Handle context window limits | High | Summarize when approaching token limits |
| **Environment variable exposure** | Claude hooks integration | Low | RALPH_* vars for Claude Code awareness |
| **AGENTS.md generation** | Module-specific context for future runs | Medium | Auto-generate context files |

### Developer Experience

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Sandbox/Docker mode** | Safe execution for untrusted operations | High | Containerized execution, isolated filesystem |
| **LSP integration** | Code intelligence for error detection | High | Connect to language servers for diagnostics |
| **Cost tracking** | API usage visibility | Medium | Track tokens/iterations, estimated cost |
| **tmux integration** | Background operation with monitoring | Medium | Live pane for long runs |

## Anti-Features

Things to deliberately NOT build. Common mistakes in this domain.

### Complexity Traps

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **Built-in LLM API calls** | Massive complexity, model churn, auth headaches | Pilot Claude CLI (or other CLIs) as subprocess; let them handle API |
| **Multi-model routing** | Premature optimization, high complexity | Support agent CLI selection; let user pick tool per model |
| **Custom prompt templating engine** | Over-engineering; Markdown files work | Use PROMPT.md files; simple variable substitution if needed |
| **Interactive chat mode** | Different product; dilutes focus | This is autonomous loop, not chat. Use `--once` for human review |
| **Browser/web UI** | Scope creep; different product category | Stay CLI/TUI focused; web monitoring via webhooks if needed |

### Autonomy Dangers

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **Auto-push to remote** | Dangerous; irreversible | Commit locally only; user explicitly pushes |
| **Unlimited iterations by default** | Cost explosion, stuck loops | Require explicit `--max-iterations` or sensible default (10-20) |
| **No-confirmation destructive actions** | Safety risk | Default to confirmation; `--dangerously-skip-*` flags explicit |
| **Modifying files outside project** | Security risk, unexpected changes | Strict working directory scoping; never touch ~/.* |

### Scope Creep

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **Project scaffolding/init wizard** | Many tools do this; not core value | Focus on loop execution; use existing project structures |
| **Code explanation features** | Different use case; chat tools do this | Pure execution focus; explanation is Claude CLI's job |
| **Competitive analysis features** | Out of scope | Let PRD creation handle this separately |
| **File watching/hot reload** | Different paradigm; complicates loop | Iteration-based, not reactive; use manual re-run |

### Implementation Mistakes

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **Sync subprocess execution** | Blocks TUI updates, poor UX | Async subprocess with streaming output capture |
| **Single-file state** | Corruption risk, merge conflicts | Separate files: tasks.json, progress.txt, config.toml |
| **Hardcoded completion markers** | Inflexible | Configurable `--completion-promise` pattern |
| **Blocking on stdin** | Can hang in automated/CI contexts | Timeout-based input or explicit `--interactive` flag |

## Feature Dependencies

Understanding which features must exist before others can be built.

```
Foundation Layer (build first):
  Config file parsing
    ├── CLI flag overrides (extends config)
    └── Environment variable support (extends config)

  Subprocess execution
    ├── Output streaming (requires subprocess)
    ├── Exit code handling (requires subprocess)
    └── Timeout management (requires subprocess)

Core Loop (depends on Foundation):
  Task file parsing
    └── Priority ordering (extends task parsing)

  Iteration loop
    ├── Max iteration limits (loop control)
    ├── Completion detection (loop termination)
    ├── Progress persistence (loop state)
    └── Sleep intervals (loop timing)

  Git integration
    └── Checkpoint/rollback (extends git)

TUI Layer (depends on Core Loop):
  Status bar
    ├── Iteration counter (displays loop state)
    └── Current task display (displays task state)

  Output streaming panel
    └── Collapsible threads (extends output panel)

  Keyboard handling
    └── Navigation between panels

Advanced Features (depends on TUI):
  Lifecycle hooks
    └── Webhook notifications (specific hook type)

  Verification integration
    └── Acceptance testing (extends verification)

  Session management
    └── Auto-compaction (extends sessions)
```

## MVP Recommendation

For MVP, prioritize these features to be competitive:

### Must Have (Phase 1)
1. **Config file (TOML)** - foundation for all settings
2. **CLI flag overrides** - standard CLI behavior
3. **Task file parsing** - structured input
4. **Iteration loop** - core value proposition
5. **Progress persistence** - resume capability
6. **Completion detection** - know when done
7. **Max iterations** - safety mechanism
8. **Basic status output** - visibility
9. **Git auto-commit** - rollback safety

### Should Have (Phase 2)
1. **TUI with status bar** - differentiator
2. **Live streaming output** - user experience
3. **Single iteration mode** - human-in-loop option
4. **Dry-run mode** - preview capability
5. **Verification integration** - quality gates

### Defer to Post-MVP
- Collapsible threads (complex TUI)
- Lifecycle hooks (extensibility)
- Webhook notifications (remote monitoring)
- Sandbox mode (security isolation)
- Session management (resumable conversations)
- Parallel agents (architectural complexity)

## Sources

### Primary Sources (HIGH confidence)
- [kylemclaren/ralph GitHub](https://github.com/kylemclaren/ralph) - Reference implementation
- [Ralph Wiggum Loop Gist](https://gist.github.com/Mburdo/ce99c9b08601aaf771efaabf1260d4c0) - Original pattern documentation
- [Aider documentation](https://aider.chat/) - Industry-standard CLI coding tool
- [Aider git integration](https://aider.chat/docs/git.html) - Git workflow patterns
- [Aider lint-test docs](https://aider.chat/docs/usage/lint-test.html) - Verification integration

### Secondary Sources (MEDIUM confidence)
- [OpenCode GitHub](https://github.com/opencode-ai/opencode) - TUI patterns, session management
- [Plandex](https://plandex.ai/) - Enterprise features, diff sandbox
- [Ralph Agent Starter Kit](https://github.com/studioorange-ai/ralph-agent-starter-kit) - Webhook, sandbox features
- [wiggumz GitHub](https://github.com/mjtechguy/wiggumz) - Configuration patterns, safety features
- [Claude Code permissions guide](https://www.eesel.ai/blog/claude-code-permissions) - Permission modes
- [Cline comparison guide](https://cline.bot/blog/best-ai-coding-assistant-2025-complete-guide-to-cline-and-cursor) - IDE agent patterns

### Ecosystem Surveys (MEDIUM confidence)
- [Top 10 CLI Coding Agents](https://dev.to/forgecode/top-10-open-source-cli-coding-agents-you-should-be-using-in-2025-with-links-244m)
- [AI CLI Tools 2025](https://www.aimagicx.com/blog/best-ai-coding-agent-cli-tools-2025/)
- [Ralph Wiggum Loop challenges](https://tectontide.com/en/blog/ralph-wiggum-loop/)
- [Awesome Claude - Ralph Wiggum](https://awesomeclaude.ai/ralph-wiggum)

### Pattern Sources (MEDIUM confidence)
- [Gemini CLI session management](https://developers.googleblog.com/pick-up-exactly-where-you-left-off-with-session-management-in-gemini-cli/)
- [Claude Code hooks observability](https://keywordsai.co/blog/ai-code-assistant-observability-hooks)
