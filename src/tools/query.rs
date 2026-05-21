use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use rust_mcp_sdk::schema::{CallToolResult, TextContent, schema_utils::CallToolError};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::AppState;

#[mcp_tool(
    name = "query",
    description = "Query a Typst document to extract metadata or content fields. Uses Typst's query CLI to select elements and return field values."
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct Query {
    /// Path to the .typ source file. If provided, `content` is ignored.
    file: Option<String>,
    /// Inline Typst source content. Used only if `file` is not provided.
    content: Option<String>,
    /// Selector expression (e.g., `heading`, `text`, `link`, `figure`).
    selector: String,
    /// Field to extract from matched elements (e.g., `body`, `level`, `destination`).
    field: String,
    /// Output format: `json` (default) or `yaml`.
    format: Option<String>,
}

impl Query {
    pub async fn call_tool(
        &self,
        state: Arc<Mutex<AppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let _state = state.lock().await;

        let (source_label, source_arg) = if let Some(ref path) = self.file {
            ("file", path.as_str())
        } else if let Some(ref content) = self.content {
            ("content", content.as_str())
        } else {
            return Ok(CallToolResult::text_content(vec![TextContent::from(
                "❌ Must provide either `file` or `content`.".to_string(),
            )]));
        };

        let fmt = self.format.as_deref().unwrap_or("json");
        if fmt != "json" && fmt != "yaml" {
            return Ok(CallToolResult::text_content(vec![TextContent::from(
                "❌ Invalid format. Supported formats: `json`, `yaml`.".to_string(),
            )]));
        }

        let result = if source_label == "file" {
            run_typst_query(source_arg, &self.selector, &self.field, fmt)
        } else {
            query_from_content(source_arg, &self.selector, &self.field, fmt)
        };

        match result {
            Ok(output) => {
                let lines = [
                    format!(
                        "📋 Query results for selector `{}`, field `{}`:",
                        self.selector, self.field
                    ),
                    String::new(),
                    output,
                ];
                Ok(CallToolResult::text_content(vec![TextContent::from(
                    lines.join("\n"),
                )]))
            }
            Err(e) => Ok(CallToolResult::text_content(vec![TextContent::from(
                format!("❌ Query failed: {}", e),
            )])),
        }
    }
}

fn run_typst_query(
    input: &str,
    selector: &str,
    field: &str,
    format: &str,
) -> Result<String, String> {
    let mut cmd = std::process::Command::new("typst");
    cmd.args(["query", input, selector, field]);
    cmd.args(["--format", format]);

    let output_result = cmd
        .output()
        .map_err(|e| format!("Failed to run typst command: {}", e))?;

    if output_result.status.success() {
        let stdout = String::from_utf8_lossy(&output_result.stdout);
        let trimmed = stdout.trim();
        if trimmed.is_empty() {
            Ok("(no results found)".to_string())
        } else {
            Ok(trimmed.to_string())
        }
    } else {
        let stderr = String::from_utf8_lossy(&output_result.stderr);
        Err(stderr.trim().to_string())
    }
}

fn query_from_content(
    content: &str,
    selector: &str,
    field: &str,
    format: &str,
) -> Result<String, String> {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("mcp-typst-query-{}.typ", std::process::id()));
    let temp_path = temp_file
        .to_str()
        .ok_or("Failed to create temp file path")?;

    std::fs::write(&temp_file, content).map_err(|e| format!("Failed to write temp file: {}", e))?;

    let result = run_typst_query(temp_path, selector, field, format);

    let _ = std::fs::remove_file(&temp_file);

    result
}
