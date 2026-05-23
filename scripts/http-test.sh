#!/bin/sh
# HTTP test script for MCP Typst
# Usage:
#   ./scripts/http-test.sh <port> tools/list
#   ./scripts/http-test.sh <port> list_fonts
#   ./scripts/http-test.sh <port> compile file=/path/to/doc.typ embed=true

PORT="${1:-3007}"
TOOL="${2:-list_fonts}"
shift 2 2>/dev/null

# Build arguments JSON
ARGS="{"
FIRST=1
for arg in "$@"; do
  [ "$FIRST" -eq 1 ] || ARGS="$ARGS,"
  FIRST=0
  KEY="${arg%%=*}"
  VAL="${arg#*=}"
  # simple heuristic: if it looks like a number or boolean, don't quote
  case "$VAL" in
    true|false|[0-9]*) ARGS="$ARGS\"$KEY\":$VAL" ;;
    *) ARGS="$ARGS\"$KEY\":\"$VAL\"" ;;
  esac
done
ARGS="$ARGS}"

# Initialize session
RESPONSE=$(curl -s -D - "http://127.0.0.1:$PORT/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"script","version":"1.0"}}}')
SESSION=$(echo "$RESPONSE" | grep -i "mcp-session-id" | head -1 | sed 's/.*: //' | tr -d '\r')

# Build tool call with proper quoting
cat << ENDJSON | curl -s "http://127.0.0.1:$PORT/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Mcp-Session-Id: $SESSION" \
  -d @-
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"$TOOL","arguments":$ARGS}}
ENDJSON
