use std::sync::Arc;

use tokio::sync::Mutex;

use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use rust_mcp_sdk::schema::{CallToolResult, TextContent, schema_utils::CallToolError};

use crate::AppState;

#[mcp_tool(
    name = "export",
    description = "Export a Typst document to PNG or SVG format. Supports both file and inline content."
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct Export {
    /// Path to the .typ source file. If provided, `content` is ignored.
    file: Option<String>,
    /// Inline Typst source content. Used only if `file` is not provided.
    content: Option<String>,
    /// Output format: `png` or `svg`.
    format: String,
    /// Output file path. Defaults to source filename with .png/.svg extension.
    output: Option<String>,
    /// Resolution in pixels per page for PNG export. Ignored for SVG.
    pixel_per_page: Option<u32>,
    /// Return the exported file as a base64-encoded embedded resource. Default: false.
    embed: Option<bool>,
}

impl Export {
    pub async fn call_tool(
        &self,
        state: Arc<Mutex<AppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;

        let fmt = self.format.to_lowercase();
        if fmt != "png" && fmt != "svg" {
            return Ok(CallToolResult::text_content(vec![TextContent::from(
                "❌ Invalid format. Supported formats: `png`, `svg`.".to_string(),
            )]));
        }

        let (source_label, source_arg) = if let Some(ref path) = self.file {
            ("file", path.as_str())
        } else if let Some(ref content) = self.content {
            ("content", content.as_str())
        } else {
            return Ok(CallToolResult::text_content(vec![TextContent::from(
                "❌ Must provide either `file` or `content`.".to_string(),
            )]));
        };

        let ext = if fmt == "png" { ".png" } else { ".svg" };
        let output = crate::tools::compile::resolve_output_path(
            self.output.as_deref(),
            source_label,
            source_arg,
            state.output_path.as_deref(),
            ext,
        );

        let result = if source_label == "file" {
            run_typst_export(source_arg, &output, &fmt, self.pixel_per_page)
        } else {
            export_from_content(source_arg, &output, &fmt, self.pixel_per_page)
        };

        match result {
            Ok(_) => {
                if self.embed.unwrap_or(false) {
                    let mime_type = if fmt == "png" {
                        "image/png"
                    } else {
                        "image/svg+xml"
                    };
                    crate::tools::compile::build_embedded_result(&output, mime_type)
                } else {
                    Ok(CallToolResult::text_content(vec![TextContent::from(
                        format!(
                            "✅ Document exported successfully.\n\nFormat: {}\nOutput: {}",
                            fmt.to_uppercase(),
                            output
                        ),
                    )]))
                }
            }
            Err(e) => Ok(CallToolResult::text_content(vec![TextContent::from(
                format!("❌ Export failed: {}", e),
            )])),
        }
    }
}

fn run_typst_export(
    input: &str,
    output: &str,
    fmt: &str,
    pixel_per_page: Option<u32>,
) -> Result<(), String> {
    if let Some(parent) = std::path::Path::new(output).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    let mut cmd = std::process::Command::new("typst");
    cmd.args(["compile", input, output]);
    cmd.args(["--format", fmt]);

    if fmt == "png"
        && let Some(ppp) = pixel_per_page
    {
        cmd.args(["--pixel-per-page", &ppp.to_string()]);
    }

    let output_result = cmd
        .output()
        .map_err(|e| format!("Failed to run typst command: {}", e))?;

    if output_result.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output_result.stderr);
        Err(stderr.trim().to_string())
    }
}

fn export_from_content(
    content: &str,
    output: &str,
    fmt: &str,
    pixel_per_page: Option<u32>,
) -> Result<(), String> {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("mcp-typst-export-{}.typ", std::process::id()));
    let temp_path = temp_file
        .to_str()
        .ok_or("Failed to create temp file path")?;

    std::fs::write(&temp_file, content).map_err(|e| format!("Failed to write temp file: {}", e))?;

    let result = run_typst_export(temp_path, output, fmt, pixel_per_page);

    let _ = std::fs::remove_file(&temp_file);

    result
}
