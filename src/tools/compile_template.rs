use std::sync::Arc;

use tokio::sync::Mutex;

use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use rust_mcp_sdk::schema::{CallToolResult, TextContent, schema_utils::CallToolError};

use crate::AppState;

#[mcp_tool(
    name = "compile_template",
    description = "Compile a Typst document using a template. The template is imported into the document before compilation."
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct CompileTemplate {
    /// Template name (without .typ extension). Must exist in the configured template directory.
    template: String,
    /// Path to the .typ source file. If provided, `content` is ignored.
    file: Option<String>,
    /// Inline Typst source content. Used only if `file` is not provided.
    content: Option<String>,
    /// Output PDF file path. Defaults to the source filename with .pdf extension.
    output: Option<String>,
    /// Return the compiled PDF as a base64-encoded embedded resource. Default: false.
    embed: Option<bool>,
}

impl CompileTemplate {
    pub async fn call_tool(
        &self,
        state: Arc<Mutex<AppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let template_path = &state.template_path;

        let template_file = std::path::Path::new(template_path)
            .join(&self.template)
            .with_extension("typ");

        if !template_file.exists() {
            return Ok(CallToolResult::text_content(vec![TextContent::from(
                format!(
                    "❌ Template '{}' not found at '{}'. Use 'list_templates' to see available templates.",
                    self.template,
                    template_file.display()
                ),
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

        let output = crate::tools::compile::resolve_output_path(
            self.output.as_deref(),
            source_label,
            source_arg,
            state.output_path.as_deref(),
            ".pdf",
        );

        let source_content = if source_label == "file" {
            match std::fs::read_to_string(source_arg) {
                Ok(c) => c,
                Err(e) => {
                    return Ok(CallToolResult::text_content(vec![TextContent::from(
                        format!("❌ Failed to read source file '{}': {}", source_arg, e),
                    )]));
                }
            }
        } else {
            source_arg.to_owned()
        };

        let combined = format!(
            "#import \"{}\": *\n\n{}",
            template_file.to_string_lossy().replace('\\', "/"),
            source_content
        );

        let result = compile_from_content(&combined, &output);

        match result {
            Ok(_) => {
                if self.embed.unwrap_or(false) {
                    crate::tools::compile::build_embedded_result(&output, "application/pdf")
                } else {
                    Ok(CallToolResult::text_content(vec![TextContent::from(
                        format!(
                            "✅ Document compiled with template '{}'.\n\nOutput: {}",
                            self.template, output
                        ),
                    )]))
                }
            }
            Err(e) => Ok(CallToolResult::text_content(vec![TextContent::from(
                format!("❌ Compilation failed: {}", e),
            )])),
        }
    }
}

fn compile_from_content(content: &str, output: &str) -> Result<(), String> {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("mcp-typst-{}.typ", std::process::id()));
    let temp_path = temp_file
        .to_str()
        .ok_or("Failed to create temp file path")?;

    std::fs::write(&temp_file, content).map_err(|e| format!("Failed to write temp file: {}", e))?;

    let result = run_typst_compile(temp_path, output);

    let _ = std::fs::remove_file(&temp_file);

    result
}

fn run_typst_compile(input: &str, output: &str) -> Result<(), String> {
    if let Some(parent) = std::path::Path::new(output).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    let cmd = std::process::Command::new("typst")
        .args(["compile", input, output])
        .output()
        .map_err(|e| format!("Failed to run typst command: {}", e))?;

    if cmd.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&cmd.stderr);
        Err(stderr.trim().to_string())
    }
}
