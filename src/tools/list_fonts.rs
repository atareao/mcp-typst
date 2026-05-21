use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use rust_mcp_sdk::schema::{CallToolResult, TextContent, schema_utils::CallToolError};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::AppState;

#[mcp_tool(
    name = "list_fonts",
    description = "List fonts available to Typst. Optionally filter by name."
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct ListFonts {
    /// Optional search term to filter fonts by name (case-insensitive).
    search: Option<String>,
}

impl ListFonts {
    pub async fn call_tool(
        &self,
        state: Arc<Mutex<AppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let _state = state.lock().await;

        let result = run_typst_fonts_list();

        match result {
            Ok(fonts) => {
                let filtered: Vec<&str> = match &self.search {
                    Some(term) => {
                        let term_lower = term.to_lowercase();
                        fonts
                            .iter()
                            .filter(|f| f.to_lowercase().contains(&term_lower))
                            .map(|s| s.as_str())
                            .collect()
                    }
                    None => fonts.iter().map(|s| s.as_str()).collect(),
                };

                if filtered.is_empty() {
                    return Ok(CallToolResult::text_content(vec![TextContent::from(
                        match &self.search {
                            Some(term) => format!("🔍 No fonts matching '{}'.", term),
                            None => "📝 No fonts found.".to_string(),
                        },
                    )]));
                }

                let mut lines = vec![
                    match &self.search {
                        Some(term) => format!("🔍 Fonts matching '{}':", term),
                        None => "📝 Available fonts:".to_string(),
                    },
                    String::new(),
                ];
                for font in &filtered {
                    lines.push(format!("  - {}", font));
                }
                lines.push(String::new());
                lines.push(format!("Total: {} font(s)", filtered.len()));

                Ok(CallToolResult::text_content(vec![TextContent::from(
                    lines.join("\n"),
                )]))
            }
            Err(e) => Ok(CallToolResult::text_content(vec![TextContent::from(
                format!("❌ Failed to list fonts: {}", e),
            )])),
        }
    }
}

fn run_typst_fonts_list() -> Result<Vec<String>, String> {
    let output_result = std::process::Command::new("typst")
        .args(["fonts", "list"])
        .output()
        .map_err(|e| format!("Failed to run typst command: {}", e))?;

    if output_result.status.success() {
        let stdout = String::from_utf8_lossy(&output_result.stdout);
        let fonts: Vec<String> = stdout
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
        Ok(fonts)
    } else {
        let stderr = String::from_utf8_lossy(&output_result.stderr);
        Err(stderr.trim().to_string())
    }
}
