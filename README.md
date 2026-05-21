# MCP Typst Server

A Model Context Protocol (MCP) server that enables AI assistants to compile, export, and query [Typst](https://typst.app/) documents. Written in Rust.

## Features

- **Compile** Typst documents to PDF from files or inline content
- **Template support** — prepend reusable templates before compilation
- **Export** documents to PNG or SVG formats
- **Query** document metadata and content via Typst selectors
- **List fonts** available to Typst, with optional filtering
- **Embedded output** — return compiled files as base64-encoded resources for direct consumption by AI clients
- **Dual transport** — stdio (default) or HTTP

## Tools

| Tool | Description |
|------|-------------|
| `compile` | Compile a `.typ` file or inline content to PDF |
| `compile_template` | Compile with a template (prepends `#import` to source) |
| `export` | Export to PNG or SVG (with `--pixel-per-page` for PNG) |
| `query` | Extract document metadata/content via Typst selectors |
| `list_fonts` | List available fonts, optionally filtered by name |
| `list_templates` | List `.typ` files in the configured template directory |

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) 1.85+ (Edition 2024)
- [Typst](https://typst.app/) CLI installed and on `$PATH`

### From Source

```bash
git clone https://github.com/atareao/mcp-typst.git
cd mcp-typst
cargo build --release
```

The binary will be at `target/release/mcp-typst`.

### With Docker

```bash
docker build -t atareao/mcp-typst .
```

The image includes Typst pre-installed.

## Usage

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `TYPST_TEMPLATE_PATH` | `./templates` | Directory containing `.typ` template files |
| `MCP_TRANSPORT` | `stdio` | Transport mode: `stdio` or `http` |
| `MCP_HOST` | `127.0.0.1` | HTTP bind address (HTTP transport only) |
| `MCP_PORT` | `3003` | HTTP port (HTTP transport only) |

### Stdio Transport (Default)

Pipe-based communication, ideal for local AI assistant integrations:

```bash
mcp-typst
```

### HTTP Transport

For remote or network-based access:

```bash
MCP_TRANSPORT=http MCP_PORT=3003 mcp-typst
```

### With Claude Desktop

Add to your Claude Desktop MCP configuration:

```json
{
  "mcpServers": {
    "typst": {
      "command": "mcp-typst",
      "env": {
        "TYPST_TEMPLATE_PATH": "/path/to/templates"
      }
    }
  }
}
```

### With Docker

```bash
docker run -i --rm \
  -e TYPST_TEMPLATE_PATH=/templates \
  -v /path/to/templates:/templates:ro \
  -v /path/to/output:/output \
  atareao/mcp-typst
```

## Examples

### Compile a file to PDF

```
compile(file="document.typ")
```

### Compile with inline content and embedded output

```
compile(content="#set text(font: \"Linux Libertine\")\n= Hello\nWorld", embed=true)
```

When `embed=true`, the response includes the PDF as a base64-encoded `EmbeddedResource`, allowing AI clients to access the file content directly without reading from disk.

### Compile with a template

```
compile_template(template="report", file="content.typ")
```

### Export to PNG

```
export(file="slides.typ", format="png", pixel_per_page=200)
```

### Query document headings

```
query(file="document.typ", selector="heading", field="body")
```

### List fonts matching "libertine"

```
list_fonts(search="libertine")
```

## Architecture

```
src/
├── main.rs              # Server entry point, transport setup
└── tools/
    ├── mod.rs           # Tool registry (tool_box! macro)
    ├── compile.rs       # PDF compilation
    ├── compile_template.rs  # Template-based compilation
    ├── export.rs        # PNG/SVG export
    ├── query.rs         # Document querying
    ├── list_fonts.rs    # Font listing
    └── list_templates.rs    # Template listing
```

The server uses [`rust-mcp-sdk`](https://crates.io/crates/rust-mcp-sdk) with the MCP 2025-11-25 protocol version, supporting both stdio and HTTP (Hyper-based) transports.

## Development

```bash
cargo build          # Build
cargo run            # Run
cargo test           # Run tests
cargo clippy         # Lint
cargo fmt            # Format
```
