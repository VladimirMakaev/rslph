# Phase 7: E2E Testing Framework - Research

**Researched:** 2026-01-19
**Domain:** End-to-end testing infrastructure with fake Claude simulation
**Confidence:** MEDIUM

## Summary

This research investigates the technical foundations for building a comprehensive E2E testing framework for rslph. The framework requires a fake Claude process that outputs stream-json format matching the real Claude CLI, pytest fixtures for workspace isolation, and approaches for TUI testing.

Key findings:
1. Claude CLI's stream-json format uses JSONL with event types: user, assistant, system, result, summary - already partially implemented in `src/subprocess/stream_json.rs`
2. Python is the recommended language for fake Claude due to easy executable creation (shebang + chmod) and pytest integration
3. Ratatui provides built-in `TestBackend` for unit testing widgets, while `ratatui-testlib` offers PTY-based integration testing (still in early development)
4. pytest's `tmp_path_retention_policy = "failed"` configuration directly supports keeping test directories on failure

**Primary recommendation:** Use Python/pytest for test orchestration with the fake-claude package, leverage Rust's existing TestBackend for widget testing, and defer full TUI integration testing to phase 8 or later.

## Standard Stack

The established libraries/tools for this domain:

### Python Testing Stack (for fake-claude and E2E tests)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| pytest | 8.x | Test framework | Industry standard, fixture system, parametrization |
| tempfile | stdlib | Temp file management | Python stdlib, delete=False for persistent fake executable |
| subprocess | stdlib | Process invocation | Test rslph binary invocation |
| pytest-git | 1.8.0 | Git repo fixtures | Creates isolated git repos for testing VCS features |

### Rust Testing Stack (for TUI unit tests)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui TestBackend | 0.30+ | Widget unit testing | Built into ratatui, renders to buffer |
| tempfile | 3.x | Temp directory management | Already in dev-dependencies |
| assert_cmd | 2.x | CLI binary testing | Standard for Rust CLI testing |
| assert_fs | 1.x | Filesystem fixtures | Pairs with assert_cmd |

### Optional/Future
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| insta | 2.x | Snapshot testing | For visual TUI regression testing |
| ratatui-testlib | 0.1+ (future) | PTY-based TUI testing | When testing real terminal behavior |
| scrut | 0.4.x | CLI doc-tests | For documentation-as-tests approach |

**Installation (Python):**
```bash
pip install pytest pytest-git
```

**Installation (Rust):**
```bash
# Already have tempfile in dev-dependencies
cargo add --dev assert_cmd assert_fs insta
```

## Architecture Patterns

### Recommended Project Structure
```
tests/
  e2e/                    # Python E2E tests
    conftest.py           # pytest fixtures
    test_basic_loop.py    # Basic loop scenarios
    test_edge_cases.py    # Timeout, crash, malformed
    test_multi_invoke.py  # Multi-invocation scenarios
  fixtures/               # Shared test data
    sample_progress.md    # Sample progress files
    sample_config.toml    # Sample configs

fake_claude/              # Python package for fake CLI
  __init__.py             # Package init with fake_claude() factory
  scenario.py             # Scenario builder API
  executable.py           # Fake executable script generator
  stream_json.py          # stream-json output generators

src/                      # Rust TUI tests (inline)
  tui/
    widgets/
      progress_bar.rs     # Contains #[cfg(test)] mod tests
      status_bar.rs       # Contains #[cfg(test)] mod tests
```

### Pattern 1: Fake Claude Builder API
**What:** Fluent builder pattern for configuring fake Claude responses
**When to use:** All E2E tests that need deterministic Claude output
**Example:**
```python
# Source: CONTEXT.md decisions
def test_basic_task_completion():
    fake = (
        fake_claude()
        .on_invocation(1)
            .respond_with_text("I'll complete task 1")
            .uses_read("src/main.rs")
            .uses_write("PROGRESS.md", updated_progress)
        .next_invocation()
            .respond_with_text("Task 2 complete")
            .uses_bash("cargo test")
        .build()
    )

    result = subprocess.run(
        ["rslph", "build", "--claude-path", fake.executable_path],
        cwd=workspace.path
    )

    assert result.returncode == 0
    assert "RALPH_DONE" in (workspace.path / "PROGRESS.md").read_text()

    fake.cleanup()
```

### Pattern 2: Workspace Fixture with Builder
**What:** pytest fixture providing isolated workspace with fluent customization
**When to use:** All tests needing file system isolation
**Example:**
```python
# Source: pytest-git documentation + CONTEXT.md decisions
@pytest.fixture
def workspace(tmp_path):
    """Creates minimal valid rslph workspace."""
    ws = WorkspaceBuilder(tmp_path)
    ws.init_git()
    ws.write_config({"claude_path": "claude"})
    return ws

def test_custom_workspace(workspace):
    ws = (
        workspace
        .with_progress_file("PROGRESS.md", sample_progress)
        .with_source_file("src/main.rs", "fn main() {}")
    )
    # Test uses ws.path
```

### Pattern 3: Stream-JSON Response Generation
**What:** Generate valid stream-json JSONL matching Claude CLI format
**When to use:** Fake Claude response output
**Example:**
```python
# Source: src/subprocess/stream_json.rs + claude-clean github
def generate_stream_json(
    event_type: str,  # "assistant", "user", "system", "result"
    content_blocks: list,  # [{"type": "text", "text": "..."}]
    model: str = "claude-opus-4.5",
    stop_reason: str = None,
    usage: dict = None
) -> str:
    """Generate a single stream-json line."""
    event = {
        "type": event_type,
        "message": {
            "role": event_type if event_type in ("assistant", "user") else None,
            "content": content_blocks,
            "model": model,
        },
        "uuid": str(uuid.uuid4()),
        "timestamp": datetime.now().isoformat()
    }
    if stop_reason:
        event["message"]["stop_reason"] = stop_reason
    if usage:
        event["message"]["usage"] = usage
    return json.dumps(event)
```

### Anti-Patterns to Avoid
- **Testing against real Claude:** Never use real Claude in tests - non-deterministic, slow, expensive
- **Mocking at wrong layer:** Don't mock internal rslph functions - test the actual binary
- **Tight coupling to JSON format:** Use helpers/constants for JSON field names, not hardcoded strings
- **Ignoring cleanup:** Always call cleanup() on fake processes, use context managers where possible

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Temp directories | Manual tempfile | pytest tmp_path | Automatic cleanup, retention policy support |
| Git repo in tests | Manual git init | pytest-git | Handles cleanup, provides GitPython API |
| Fake executable | Shell script | Python with shebang | Cross-platform, easier JSON generation |
| Stream-json parsing | Regex parsing | serde_json / json.loads | Already have type-safe structs |
| Buffer assertions | Manual string compare | TestBackend.assert_buffer | Provides diff output on failure |
| CLI binary invocation | Raw subprocess | assert_cmd | Better error messages, fluent API |

**Key insight:** The Python ecosystem has mature testing infrastructure. Use pytest fixtures and existing packages rather than building custom test harnesses.

## Common Pitfalls

### Pitfall 1: Stream-JSON Format Mismatch
**What goes wrong:** Fake Claude outputs JSON that doesn't match real Claude CLI format, causing parse failures
**Why it happens:** Claude CLI wraps Anthropic API responses in a different structure than raw API
**How to avoid:**
- Reference existing `stream_json.rs` tests for exact format
- Use event types: "user", "assistant", "system", "result", "summary"
- Include required fields: type, message.content[], uuid, timestamp
**Warning signs:** serde_json parse errors, missing text extraction, usage tracking failures

### Pitfall 2: Temporary File Cleanup Race
**What goes wrong:** Fake executable is deleted before subprocess finishes reading it
**Why it happens:** Python's tempfile with delete=True cleans up on close, not on subprocess exit
**How to avoid:**
- Use NamedTemporaryFile with delete=False
- Implement explicit cleanup() method called after subprocess completes
- Consider using temp directory instead of temp file
**Warning signs:** FileNotFoundError, "executable not found" errors

### Pitfall 3: Pytest tmp_path Cleanup Hides Failures
**What goes wrong:** Test fails but workspace files are deleted, can't debug
**Why it happens:** Default tmp_path_retention_policy is "all" but count is 3
**How to avoid:**
- Set `tmp_path_retention_policy = "failed"` in pytest.ini
- Use `--basetemp` to override location for debugging
- Set `RSLPH_TEST_KEEP_WORKSPACE=1` env var to always keep
**Warning signs:** Can't find test output files after failure

### Pitfall 4: TUI Test Terminal Size Mismatch
**What goes wrong:** Widget tests pass locally but fail in CI
**Why it happens:** Different terminal sizes between environments
**How to avoid:**
- Always specify explicit dimensions: TestBackend::new(80, 24)
- Use consistent dimensions across all tests
- Document expected dimensions in test comments
**Warning signs:** Truncated text, layout differences between environments

### Pitfall 5: Streaming Timing Assumptions
**What goes wrong:** Tests pass with instant output but fail with delays
**Why it happens:** rslph may have timeouts or buffering assumptions
**How to avoid:**
- Make streaming delays configurable in fake Claude
- Test both instant and delayed output scenarios
- Use small delays (10-100ms) to catch timing bugs
**Warning signs:** Timeout errors, partial output, buffering issues

## Code Examples

### Claude CLI Stream-JSON Format
```json
// Source: src/subprocess/stream_json.rs tests (verified)
// User message event
{"type":"user","message":{"role":"user","content":"Hello"},"uuid":"abc","timestamp":"2026-01-18T00:00:00Z"}

// Assistant text response
{"type":"assistant","message":{"id":"123","role":"assistant","content":[{"type":"text","text":"Hello world"}],"model":"claude-opus-4.5","stop_reason":"end_turn","usage":{"input_tokens":100,"output_tokens":50}},"uuid":"abc","timestamp":"2026-01-18T00:00:00Z"}

// Tool use response
{"type":"assistant","message":{"id":"123","role":"assistant","content":[{"type":"thinking","thinking":"Let me read the file"},{"type":"tool_use","id":"tool1","name":"Read","input":{"file_path":"/tmp/test"}}],"model":"claude-opus-4.5","stop_reason":"tool_use","usage":{"input_tokens":100,"output_tokens":50}}}

// Content block types: "text", "tool_use", "thinking", "tool_result"
```

### Python Fake Executable Pattern
```python
# Source: Python tempfile docs + CONTEXT.md decisions
import tempfile
import os
import stat
import json
import sys

def create_fake_claude_executable(responses: list[dict]) -> str:
    """Create a temporary Python script that acts as fake Claude CLI."""
    script_content = f'''#!/usr/bin/env python3
import sys
import json
import time

# Pre-configured responses (invocation index -> response data)
RESPONSES = {json.dumps(responses)}
INVOCATION_FILE = "{tempfile.gettempdir()}/fake_claude_invocation_count"

def get_invocation_number():
    try:
        with open(INVOCATION_FILE, "r") as f:
            count = int(f.read().strip())
    except FileNotFoundError:
        count = 0
    count += 1
    with open(INVOCATION_FILE, "w") as f:
        f.write(str(count))
    return count

def main():
    invocation = get_invocation_number()
    if invocation <= len(RESPONSES):
        response = RESPONSES[invocation - 1]
        for line in response.get("lines", []):
            print(json.dumps(line))
            sys.stdout.flush()
            time.sleep(response.get("delay", 0))
    sys.exit(response.get("exit_code", 0) if invocation <= len(RESPONSES) else 0)

if __name__ == "__main__":
    main()
'''

    # Create temp file with Python shebang
    fd, path = tempfile.mkstemp(suffix=".py", prefix="fake_claude_")
    os.write(fd, script_content.encode())
    os.close(fd)

    # Make executable
    os.chmod(path, os.stat(path).st_mode | stat.S_IEXEC | stat.S_IXUSR)

    return path
```

### Ratatui TestBackend Widget Testing
```rust
// Source: ratatui docs + existing codebase patterns
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_progress_bar_rendering() {
        let backend = TestBackend::new(40, 1);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|frame| {
            let area = frame.area();
            let widget = ProgressBar::new(0.75);
            frame.render_widget(widget, area);
        }).unwrap();

        let buffer = terminal.backend().buffer();
        // Assert specific cells
        assert!(buffer.get(0, 0).symbol() == "[");
        assert!(buffer.get(38, 0).symbol() == "]");

        // Or use assert_buffer for full comparison
        let expected = Buffer::with_lines(vec![
            "[==============================        ]",
        ]);
        terminal.backend().assert_buffer(&expected);
    }
}
```

### pytest.ini Configuration
```ini
# Source: pytest documentation
[pytest]
testpaths = tests/e2e
python_files = test_*.py
python_functions = test_*

# Keep tmp_path directories for failed tests only
tmp_path_retention_policy = failed
tmp_path_retention_count = 5

# Custom markers
markers =
    slow: marks tests as slow (deselect with '-m "not slow"')
    tui: marks tests that require TUI testing infrastructure
```

### Workspace Fixture Implementation
```python
# Source: pytest-git docs + pytest tmp_path docs
import pytest
from pathlib import Path
import subprocess

@pytest.fixture
def rslph_binary():
    """Build and return path to rslph binary."""
    result = subprocess.run(
        ["cargo", "build", "--release"],
        cwd=Path(__file__).parent.parent.parent,
        capture_output=True
    )
    assert result.returncode == 0, f"Build failed: {result.stderr.decode()}"
    return Path(__file__).parent.parent.parent / "target" / "release" / "rslph"

@pytest.fixture
def workspace(tmp_path, git_repo):
    """Create isolated workspace with git and minimal config."""
    class Workspace:
        def __init__(self, path: Path, git):
            self.path = path
            self.git = git
            # Create minimal config
            config_path = path / ".rslph" / "config.toml"
            config_path.parent.mkdir(parents=True, exist_ok=True)
            config_path.write_text('[rslph]\nclaude_path = "claude"\n')

        def write_progress(self, content: str) -> Path:
            path = self.path / "PROGRESS.md"
            path.write_text(content)
            return path

        def write_file(self, rel_path: str, content: str) -> Path:
            path = self.path / rel_path
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content)
            return path

    return Workspace(git_repo.workspace, git_repo)
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| TestBackend only | ratatui-testlib (PTY) | 2025 (in development) | Real terminal testing possible |
| cram .t files | scrut Markdown tests | 2024+ | Tests as documentation |
| Manual temp cleanup | pytest retention_policy | pytest 7.3+ | Automatic failure preservation |
| Shell script fakes | Python fakes | Always | Cross-platform, typed |

**Deprecated/outdated:**
- `tmpdir` fixture: Use `tmp_path` instead (returns pathlib.Path)
- pytest-tmpdir plugin: Superseded by built-in tmp_path
- Manual TestBackend buffer inspection: Use assert_buffer()

## Open Questions

Things that couldn't be fully resolved:

1. **Exact tool_result format in stream-json**
   - What we know: Content blocks include "tool_result" type
   - What's unclear: Exact structure of tool_result content (is it stringified JSON or structured?)
   - Recommendation: Capture real Claude CLI output to document; fake can start with minimal fields

2. **System and summary event timing**
   - What we know: Event types include "system" and "summary"
   - What's unclear: When exactly these appear in the stream (start only? end only?)
   - Recommendation: Can be added later based on observed behavior

3. **ratatui-testlib maturity**
   - What we know: Exists, provides PTY testing, in early development (MVP planned)
   - What's unclear: API stability, production readiness
   - Recommendation: Defer PTY-based testing; use TestBackend for now

4. **Multi-process test isolation**
   - What we know: Each test gets separate tmp_path
   - What's unclear: Race conditions when multiple tests use fake Claude invocation counter
   - Recommendation: Include test ID in invocation counter file path

## Sources

### Primary (HIGH confidence)
- src/subprocess/stream_json.rs - Existing implementation with tests
- [pytest tmp_path documentation](https://docs.pytest.org/en/stable/reference/reference.html) - Retention policy options
- [ratatui TestBackend](https://docs.rs/ratatui/0.26.2/ratatui/backend/struct.TestBackend.html) - Widget testing API
- [Python tempfile docs](https://docs.python.org/3/library/tempfile.html) - Executable creation

### Secondary (MEDIUM confidence)
- [pytest-git PyPI](https://pypi.org/project/pytest-git/) - Git fixture API
- [assert_cmd lib.rs](https://lib.rs/crates/assert_cmd) - Rust CLI testing patterns
- [Anthropic streaming API](https://platform.claude.com/docs/en/api/messages-streaming) - Event type reference
- [claude-clean GitHub](https://github.com/ariel-frischer/claude-clean) - Stream-json event types

### Tertiary (LOW confidence)
- [ratatui-testlib lib.rs](https://lib.rs/crates/ratatui-testlib) - Still in MVP development
- [scrut documentation](https://facebookincubator.github.io/scrut/docs/) - Alternative to pytest for CLI tests
- [cram GitHub](https://github.com/aiiie/cram) - Original .t test format reference

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Well-established tools (pytest, tempfile, TestBackend)
- Stream-json format: HIGH - Verified against existing codebase implementation
- Architecture patterns: MEDIUM - Based on decisions + standard practices
- Pitfalls: MEDIUM - Combination of documentation and experience
- TUI testing: LOW - ratatui-testlib still in early development

**Research date:** 2026-01-19
**Valid until:** 2026-02-19 (30 days - stable domain)

## Test Language Recommendation

Based on research comparing cram, scrut, Python/pytest, and Rust cargo test:

| Criteria | Python/pytest | Rust cargo test | scrut/cram |
|----------|--------------|-----------------|------------|
| Fake Claude creation | Excellent (shebang) | Poor (requires compilation) | Poor (shell only) |
| Subprocess testing | Excellent (subprocess) | Good (std::process) | Good (native) |
| Fixture management | Excellent (fixtures) | Manual setup | None |
| Temp directory | Excellent (tmp_path) | Good (tempfile crate) | Manual |
| Git integration | Excellent (pytest-git) | Manual | Manual |
| Parallel tests | Built-in (pytest-xdist) | Built-in | Limited |
| Assertion readability | High | Medium | High |
| Integration with rslph | External process | Internal + process | External process |

**Recommendation:**
- **Use Python/pytest** for E2E tests with fake Claude
- **Use Rust cargo test** for TUI widget unit tests (with TestBackend)
- **Defer scrut** - Nice for documentation but adds complexity; evaluate for phase 8
