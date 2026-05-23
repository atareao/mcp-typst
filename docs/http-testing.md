# HTTP Testing Guide

The MCP server supports two HTTP transport modes: **Streamable HTTP** (default `/mcp` endpoint) and **SSE** (Server-Sent Events). This guide covers testing with Streamable HTTP.

## Quick Start

Start the server with HTTP transport:

```bash
MCP_TRANSPORT=http MCP_HOST=0.0.0.0 MCP_PORT=3007 mcp-typst
```

Or with Docker:

```bash
docker run -d --rm --name mcp-typst \
  -e MCP_TRANSPORT=http \
  -e MCP_HOST=0.0.0.0 \
  -e MCP_PORT=3007 \
  -p 3007:3007 \
  -v /path/to/docs:/docs \
  atareao/mcp-typst
```

## Testing with curl

### 1. Initialize a session

The Streamable HTTP transport requires a session. Send an `initialize` request to create one:

```bash
curl -s -D - "http://127.0.0.1:3007/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
      "protocolVersion": "2025-11-25",
      "capabilities": {},
      "clientInfo": {
        "name": "test-client",
        "version": "1.0"
      }
    }
  }'
```

The response includes a `mcp-session-id` header. Save it for subsequent requests:

```bash
RESPONSE=$(curl -s -D - "http://127.0.0.1:3007/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}')

SESSION=$(echo "$RESPONSE" | grep -i "mcp-session-id" | head -1 | sed 's/.*: //' | tr -d '\r')
echo "Session: $SESSION"
```

### 2. List available tools

```bash
curl -s "http://127.0.0.1:3007/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Mcp-Session-Id: $SESSION" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/list",
    "params": {}
  }'
```

### 3. Compile a document to PDF (embed=false)

```bash
curl -s "http://127.0.0.1:3007/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Mcp-Session-Id: $SESSION" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
      "name": "compile",
      "arguments": {
        "file": "/docs/document.typ",
        "output": "/docs/output.pdf",
        "embed": false
      }
    }
  }'
```

The response contains a text message with the output file path.

### 4. Compile with embedded PDF (embed=true)

```bash
curl -s "http://127.0.0.1:3007/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Mcp-Session-Id: $SESSION" \
  -d '{
    "jsonrpc": "2.0",
    "id": 4,
    "method": "tools/call",
    "params": {
      "name": "compile",
      "arguments": {
        "content": "#set page(paper: \"a4\", flipped: true)\n= Hello\nWorld",
        "embed": true
      }
    }
  }'
```

The response includes two content blocks:
- `type: "text"` — success message with the output path
- `type: "resource"` — the PDF as a base64-encoded blob (`mimeType: "application/pdf"`)

### 5. Decode the embedded PDF

```bash
curl -s "http://127.0.0.1:3007/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Mcp-Session-Id: $SESSION" \
  -d '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"compile","arguments":{"content":"= Hello","embed":true}}}' \
  | python3 -c "
import sys, json, base64
data = json.loads(sys.stdin.read().strip().lstrip('data: '))
for block in data.get('result', {}).get('content', []):
    if block.get('type') == 'resource':
        blob = block['resource']['blob']
        decoded = base64.b64decode(blob)
        print(f'PDF size: {len(decoded)} bytes')
        print(f'Header: {decoded[:8]}')
"
```

### 6. Export to PNG

```bash
curl -s "http://127.0.0.1:3007/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Mcp-Session-Id: $SESSION" \
  -d '{
    "jsonrpc": "2.0",
    "id": 6,
    "method": "tools/call",
    "params": {
      "name": "export",
      "arguments": {
        "file": "/docs/slides.typ",
        "format": "png",
        "pixel_per_page": 200,
        "embed": true
      }
    }
  }'
```

### 7. Query document headings

```bash
curl -s "http://127.0.0.1:3007/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Mcp-Session-Id: $SESSION" \
  -d '{
    "jsonrpc": "2.0",
    "id": 7,
    "method": "tools/call",
    "params": {
      "name": "query",
      "arguments": {
        "file": "/docs/document.typ",
        "selector": "heading",
        "field": "body"
      }
    }
  }'
```

### 8. List fonts

```bash
curl -s "http://127.0.0.1:3007/mcp" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Mcp-Session-Id: $SESSION" \
  -d '{
    "jsonrpc": "2.0",
    "id": 8,
    "method": "tools/call",
    "params": {
      "name": "list_fonts",
      "arguments": {
        "search": "libertine"
      }
    }
  }'
```

## Full Script: Compile and Extract PDF

```bash
#!/bin/bash
# compile-pdf.sh — Compile a .typ file and extract the embedded PDF

DOC="${1:-document.typ}"
URL="http://127.0.0.1:3007/mcp"

# Initialize session
RESPONSE=$(curl -s -D - "$URL" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"script","version":"1.0"}}}')

SESSION=$(echo "$RESPONSE" | grep -i "mcp-session-id" | head -1 | sed 's/.*: //' | tr -d '\r')

# Compile with embed
RESULT=$(curl -s "$URL" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -H "Mcp-Session-Id: $SESSION" \
  -d "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"compile\",\"arguments\":{\"file\":\"$DOC\",\"embed\":true}}}")

# Extract base64 blob and decode to PDF
echo "$RESULT" | python3 -c "
import sys, json, base64
data = json.loads(sys.stdin.read().strip().lstrip('data: '))
for block in data.get('result', {}).get('content', []):
    if block.get('type') == 'text':
        print(block['text'])
    elif block.get('type') == 'resource':
        blob = block['resource']['blob']
        pdf = base64.b64decode(blob)
        with open('output.pdf', 'wb') as f:
            f.write(pdf)
        print(f'PDF saved: output.pdf ({len(pdf)} bytes)')
"
```

Usage:

```bash
./compile-pdf.sh /docs/calendario.typ
```

## Notes

- The session ID is obtained from the `mcp-session-id` **response header** of the `initialize` call.
- All subsequent requests must include the session ID via the `Mcp-Session-Id` **request header**.
- The response format from the Streamable HTTP endpoint is SSE (`data: <json>`). For automated scripts, strip the `data: ` prefix.
- The `Accept: application/json, text/event-stream` header is required; without it the server returns a 400 error.
