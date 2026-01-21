#!/bin/bash
# Run rslph eval with fake Claude for testing
# Usage: ./scripts/run-eval-fake-claude.sh [project]
# Example: ./scripts/run-eval-fake-claude.sh calculator

set -e

PROJECT="${1:-calculator}"

# Build test binaries (includes fake_claude)
echo "Building fake_claude..."
cargo test --no-run 2>/dev/null

# Find the fake_claude binary (most recently modified, use absolute path)
FAKE_CLAUDE_BIN=$(find "$(pwd)/target/debug/deps" -name 'fake_claude-*' -type f -perm +111 2>/dev/null | xargs ls -t 2>/dev/null | head -1)

if [ -z "$FAKE_CLAUDE_BIN" ]; then
    echo "Error: Could not find fake_claude binary. Run 'cargo test --no-run' first."
    exit 1
fi

echo "Using fake_claude: $FAKE_CLAUDE_BIN"

# Create temp files
CONFIG_FILE=$(mktemp)
COUNTER_FILE=$(mktemp)

# The Python calculator code (properly escaped for JSON)
CALC_CODE='#!/usr/bin/env python3\nimport sys\nexpr = input().strip()\nresult = eval(expr)\nif isinstance(result, float) and result.is_integer():\n    result = int(result)\nprint(result)'

# Progress file content for planning phase
PROGRESS_INIT='# Progress: Calculator\n\n## Status\n\nIn Progress\n\n## Tasks\n\n### Phase 1: Implementation\n\n- [ ] Create calculator program\n\n## Testing Strategy\n\nRun tests.'

# Progress file content with RALPH_DONE for build phase
PROGRESS_DONE='# Progress: Calculator\n\n## Status\n\nRALPH_DONE - Complete\n\n## Tasks\n\n### Phase 1: Implementation\n\n- [x] Create calculator program\n\n## Testing Strategy\n\nRun tests.'

# Create the scenario config
# Note: Build iteration text output IS the progress file - rslph parses and writes it
cat > "$CONFIG_FILE" << EOF
{
  "invocations": [
    {
      "events": [
        {"type":"system","uuid":"s1","timestamp":"2026-01-20T00:00:00Z"},
        {"type":"assistant","message":{"id":"m1","role":"assistant","content":[{"type":"text","text":"$PROGRESS_INIT"}],"model":"claude-opus-4-5-20251101","stop_reason":"end_turn","usage":{"input_tokens":1000,"output_tokens":200}},"uuid":"m1","timestamp":"2026-01-20T00:00:01Z"},
        {"type":"result","subtype":"success","cost_usd":0.01,"duration_ms":1000,"duration_api_ms":800}
      ],
      "exit_code": 0
    },
    {
      "events": [
        {"type":"system","uuid":"s2","timestamp":"2026-01-20T00:00:02Z"},
        {"type":"assistant","message":{"id":"m2","role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Write","input":{"file_path":"main.py","content":"$CALC_CODE"}},{"type":"tool_use","id":"t2","name":"Bash","input":{"command":"chmod +x main.py"}},{"type":"text","text":"$PROGRESS_DONE"}],"model":"claude-opus-4-5-20251101","stop_reason":"end_turn","usage":{"input_tokens":2000,"output_tokens":500}},"uuid":"m2","timestamp":"2026-01-20T00:00:03Z"},
        {"type":"result","subtype":"success","cost_usd":0.02,"duration_ms":2000,"duration_api_ms":1500}
      ],
      "exit_code": 0,
      "execute_tools": true
    }
  ],
  "counter_path": "$COUNTER_FILE"
}
EOF

# Initialize counter
echo "0" > "$COUNTER_FILE"

echo "Config: $CONFIG_FILE"
echo ""
echo "Running: rslph eval $PROJECT --no-tui"
echo "=========================================="

# Run eval with fake Claude
FAKE_CLAUDE_CONFIG="$CONFIG_FILE" \
RSLPH_CLAUDE_PATH="$FAKE_CLAUDE_BIN" \
  cargo run -- eval "$PROJECT" --no-tui

EXIT_CODE=$?

# Cleanup
rm -f "$CONFIG_FILE" "$COUNTER_FILE"

exit $EXIT_CODE
