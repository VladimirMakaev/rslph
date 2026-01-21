#!/bin/bash
# Run rslph eval with fake Claude for testing
# Usage: ./scripts/run-eval-fake-claude.sh [project]
# Example: ./scripts/run-eval-fake-claude.sh calculator

set -e

PROJECT="${1:-calculator}"

# Build test binaries (includes fake_claude)
echo "Building fake_claude..."
cargo test --no-run 2>/dev/null

# Find the fake_claude binary
FAKE_CLAUDE_BIN=$(find target/debug/deps -name 'fake_claude-*' -type f -perm +111 2>/dev/null | head -1)

if [ -z "$FAKE_CLAUDE_BIN" ]; then
    echo "Error: Could not find fake_claude binary. Run 'cargo test --no-run' first."
    exit 1
fi

echo "Using fake_claude: $FAKE_CLAUDE_BIN"

# Create scenario config
CONFIG_FILE=$(mktemp)
COUNTER_FILE=$(mktemp)

cat > "$CONFIG_FILE" << 'SCENARIO_EOF'
{
  "invocations": [
    {
      "events": [
        {"type":"system","uuid":"sys-001","timestamp":"2026-01-20T00:00:00Z"},
        {"type":"assistant","message":{"id":"msg-plan","role":"assistant","content":[{"type":"text","text":"# Progress: Calculator\n\n## Status\n\nIn Progress\n\n## Tasks\n\n### Phase 1: Implementation\n\n- [ ] Create calculator program\n\n## Testing Strategy\n\nRun the calculator with test inputs."}],"model":"claude-opus-4-5-20251101","stop_reason":"end_turn","usage":{"input_tokens":1000,"output_tokens":200}},"uuid":"plan-001","timestamp":"2026-01-20T00:00:01Z"},
        {"type":"result","subtype":"success","cost_usd":0.01,"duration_ms":2000,"duration_api_ms":1500}
      ],
      "exit_code": 0
    },
    {
      "events": [
        {"type":"system","uuid":"sys-002","timestamp":"2026-01-20T00:00:02Z"},
        {"type":"assistant","message":{"id":"msg-build","role":"assistant","content":[{"type":"tool_use","id":"toolu_001","name":"Write","input":{"file_path":"main.py","content":"#!/usr/bin/env python3\nimport sys\n\nexpr = input().strip()\nresult = eval(expr)\nif isinstance(result, float) and result.is_integer():\n    result = int(result)\nprint(result)\n"}},{"type":"tool_use","id":"toolu_002","name":"Bash","input":{"command":"chmod +x main.py"}},{"type":"tool_use","id":"toolu_003","name":"Write","input":{"file_path":"progress.md","content":"# Progress: Calculator\n\n## Status\n\nRALPH_DONE - All tasks complete\n\n## Tasks\n\n### Phase 1: Implementation\n\n- [x] Create calculator program\n"}},{"type":"text","text":"I've created a Python calculator that uses eval() for computation. All tasks complete."}],"model":"claude-opus-4-5-20251101","stop_reason":"end_turn","usage":{"input_tokens":2000,"output_tokens":500}},"uuid":"build-001","timestamp":"2026-01-20T00:00:03Z"},
        {"type":"result","subtype":"success","cost_usd":0.02,"duration_ms":3000,"duration_api_ms":2500}
      ],
      "exit_code": 0,
      "execute_tools": true
    }
  ],
  "counter_path": "COUNTER_PLACEHOLDER"
}
SCENARIO_EOF

# Replace counter placeholder with actual path
sed -i.bak "s|COUNTER_PLACEHOLDER|$COUNTER_FILE|g" "$CONFIG_FILE"
rm -f "${CONFIG_FILE}.bak"

# Initialize counter
echo "0" > "$COUNTER_FILE"

echo "Config: $CONFIG_FILE"
echo "Counter: $COUNTER_FILE"
echo ""
echo "Running: rslph eval $PROJECT --no-tui"
echo "=========================================="

# Run eval with fake Claude
FAKE_CLAUDE_CONFIG="$CONFIG_FILE" \
RSLPH_CLAUDE_PATH="$FAKE_CLAUDE_BIN" \
  cargo run -- eval "$PROJECT" --no-tui

# Cleanup
rm -f "$CONFIG_FILE" "$COUNTER_FILE"
