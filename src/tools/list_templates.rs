use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use rust_mcp_sdk::schema::{CallToolResult, TextContent, schema_utils::CallToolError};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::AppState;

#[mcp_tool(
    name = "list_templates",
    description = "List all available Typst templates in the configured template directory."
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct ListTemplates {}

impl ListTemplates {
    pub async fn call_tool(
        &self,
        state: Arc<Mutex<AppState>>,
    ) -> Result<CallToolResult, CallToolError> {
        let state = state.lock().await;
        let template_path = &state.template_path;

        let entries = match std::fs::read_dir(template_path) {
            Ok(entries) => entries,
            Err(e) => {
                return Ok(CallToolResult::text_content(vec![TextContent::from(
                    format!(
                        "❌ Cannot read template directory '{}': {}",
                        template_path, e
                    ),
                )]));
            }
        };

        let mut templates: Vec<String> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "typ")
                && let Some(name) = path.file_stem().and_then(|n| n.to_str())
            {
                templates.push(name.to_string());
            }
        }

        if templates.is_empty() {
            return Ok(CallToolResult::text_content(vec![TextContent::from(
                format!(
                    "📂 No templates found in '{}'.\n\nSet TYPST_TEMPLATE_PATH to a directory containing .typ template files.",
                    template_path
                ),
            )]));
        }

        templates.sort();

        let mut lines = vec![
            format!("📂 Templates in '{}':", template_path),
            String::new(),
        ];
        for name in &templates {
            lines.push(format!("  - {}", name));
        }
        lines.push(String::new());
        lines.push(format!("Total: {} template(s)", templates.len()));

        Ok(CallToolResult::text_content(vec![TextContent::from(
            lines.join("\n"),
        )]))
    }
}
