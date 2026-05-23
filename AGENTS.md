# mcp-typst

- Rust 2024 edition, single binary crate (`src/main.rs`).
- MCP server using `rust-mcp-sdk` 0.9 + `rust-mcp-schema` 0.10. Follow mcp-todo import patterns:
  - `rust_mcp_schema::` for schema types like `CompleteResult`, `ListResourcesResult`, etc.
  - `rust_mcp_sdk::mcp_server::` for server runtime, handler traits, transport options.
  - `rust_mcp_sdk::schema::` for `CallToolError`, `CallToolResult`, `TextContent`, etc.
  - `rust_mcp_sdk::macros::` for `mcp_tool`, `JsonSchema` in tool files.
- Tools use the `tool_box!` macro from `rust_mcp_sdk::tool_box`.
- Standard commands: `cargo build`, `cargo run`, `cargo test`, `cargo clippy`, `cargo fmt`.
- Docker: `Dockerfile` multi-stage (builder `rust:alpine3.23` → runtime `alpine:3.23` con `typst` instalado).

## Environment variables

- `TYPST_TEMPLATE_PATH` — directory containing `.typ` template files (default: `./templates`).
- `MCP_TRANSPORT` — `stdio` (default) or `http`.
- `MCP_HOST` / `MCP_PORT` — HTTP bind address (default: `127.0.0.1:3007`).

## Tools

- `compile` — compile a `.typ` file or inline content to PDF.
- `list_templates` — list `.typ` files in the template directory.
- `compile_template` — compile with a template (prepends `#import` to source).
- `export` — export to PNG or SVG (with `--pixel-per-page` option for PNG).
- `query` — extract document metadata/content via Typst selectors (e.g., `heading`, `text`, `link`).
- `list_fonts` — list available fonts, optionally filtered by name.

## Requirements

- `typst` CLI must be installed and on `$PATH` for compilation to work.

<!-- SKILLS_START -->
<skill>
    <name>mcp-typst-http-testing</name>
    <description>Use when testing the MCP Typst server via HTTP. Covers session initialization, curl commands for all tools, response parsing, and embedded PDF extraction.</description>
    <location>file:///data/rust/mcp-typst/docs/skills/http-testing.md</location>
</skill>
<!-- SKILLS_END -->
