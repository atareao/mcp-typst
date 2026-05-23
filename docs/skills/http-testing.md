# MCP Typst HTTP Testing

Guide for testing the MCP Typst server via Streamable HTTP transport.

## Endpoints

| Method | Endpoint | Purpose |
|--------|----------|---------|
| `POST` | `/mcp` | Streamable HTTP (initialize + tools/call) |

## Authentication Flow

1. Send `initialize` via `POST /mcp` → get `mcp-session-id` from response headers
2. Include `Mcp-Session-Id` header in all subsequent requests

### Required Headers

```
Content-Type: application/json
Accept: application/json, text/event-stream
```

The `Accept` header must include **both** `application/json` and `text/event-stream`; otherwise the server returns a 400 error.

## curl Examples

### Initialize session

```bash
RESPONSE=$(curl -s -D - "http://127.0.0.1:3007/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"client","version":"1.0"}}}')

SESSION=$(echo "$RESPONSE" | grep -i "mcp-session-id" | head -1 | sed 's/.*: //' | tr -d '\r')
```

### Compile with embedded PDF

```bash
curl -s "http://127.0.0.1:3007/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Mcp-Session-Id: $SESSION" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"compile","arguments":{"file":"/docs/doc.typ","embed":true}}}'
```

### Full script: compile and extract PDF

```bash
#!/bin/bash
DOC="${1:-document.typ}"
URL="http://127.0.0.1:3007/mcp"

RESPONSE=$(curl -s -D - "$URL" -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"script","version":"1.0"}}}')

SESSION=$(echo "$RESPONSE" | grep -i "mcp-session-id" | head -1 | sed 's/.*: //' | tr -d '\r')

RESULT=$(curl -s "$URL" -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Mcp-Session-Id: $SESSION" \
  -d "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"compile\",\"arguments\":{\"file\":\"$DOC\",\"embed\":true}}}")

echo "$RESULT" | python3 -c "
import sys, json, base64
data = json.loads(sys.stdin.read().strip().lstrip('data: '))
for block in data.get('result', {}).get('content', []):
    if block.get('type') == 'text':
        print(block['text'])
    elif block.get('type') == 'resource':
        pdf = base64.b64decode(block['resource']['blob'])
        with open('output.pdf', 'wb') as f:
            f.write(pdf)
        print(f'Saved output.pdf ({len(pdf)} bytes)')
"
```

## Response Format

The Streamable HTTP endpoint returns SSE-formatted responses:

```
data: {"id":2,"jsonrpc":"2.0","result":{"content":[...]}}
```

When `embed=true`, the content array contains:
- `type: "text"` — success message with file path
- `type: "resource"` — base64-encoded file with `mimeType` and `uri`
