use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use rust_mcp_sdk::schema::{
    BlobResourceContents, CallToolResult, EmbeddedResource, EmbeddedResourceResource, TextContent,
    schema_utils::CallToolError,
};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::AppState;

#[mcp_tool(
    name = "compile",
    description = "Compile a Typst source file to PDF. Provide either a file path or inline Typst content."
)]
#[derive(Debug, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct Compile {
    /// Path to the .typ source file. If provided, `content` is ignored.
    file: Option<String>,
    /// Inline Typst source content. Used only if `file` is not provided.
    content: Option<String>,
    /// Output PDF file path. Defaults to the source filename with .pdf extension.
    output: Option<String>,
    /// Return the compiled PDF as a base64-encoded embedded resource. Default: false.
    embed: Option<bool>,
}

impl Compile {
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

        let output = self
            .output
            .as_deref()
            .unwrap_or_else(|| {
                if source_label == "file" {
                    source_arg.strip_suffix(".typ").unwrap_or(source_arg)
                } else {
                    "output"
                }
            })
            .to_owned()
            + ".pdf";

        let result = if source_label == "file" {
            run_typst_compile(source_arg, &output)
        } else {
            compile_from_content(source_arg, &output)
        };

        match result {
            Ok(_) => {
                if self.embed.unwrap_or(false) {
                    build_embedded_result(&output, "application/pdf")
                } else {
                    Ok(CallToolResult::text_content(vec![TextContent::from(
                        format!("✅ Document compiled successfully.\n\nOutput: {}", output),
                    )]))
                }
            }
            Err(e) => Ok(CallToolResult::text_content(vec![TextContent::from(
                format!("❌ Compilation failed: {}", e),
            )])),
        }
    }
}

fn run_typst_compile(input: &str, output: &str) -> Result<(), String> {
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

pub fn build_embedded_result(
    output: &str,
    mime_type: &str,
) -> Result<CallToolResult, CallToolError> {
    let bytes = std::fs::read(output).map_err(|e| {
        CallToolError::new(std::io::Error::other(format!(
            "Failed to read output file '{}': {}",
            output, e
        )))
    })?;
    let base64_data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);

    Ok(CallToolResult {
        content: vec![
            TextContent::from(format!(
                "✅ Document compiled successfully.\n\nOutput: {}",
                output
            ))
            .into(),
            EmbeddedResource::new(
                EmbeddedResourceResource::BlobResourceContents(BlobResourceContents {
                    uri: format!(
                        "file://{}",
                        std::path::absolute(output).unwrap_or_default().display()
                    ),
                    mime_type: Some(mime_type.to_string()),
                    blob: base64_data,
                    meta: None,
                }),
                None,
                None,
            )
            .into(),
        ],
        is_error: None,
        meta: None,
        structured_content: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_mcp_sdk::schema::ContentBlock;

    struct TempFile(std::path::PathBuf);

    impl TempFile {
        fn new(path: &str) -> Self {
            let _ = std::fs::remove_file(path);
            Self(std::path::PathBuf::from(path))
        }

        fn path(&self) -> &str {
            self.0.to_str().unwrap()
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn build_embedded_result_should_return_text_and_embedded_resource() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test-embed-result.bin");
        let test_content = b"%PDF-1.4 fake pdf content";
        std::fs::write(&test_file, test_content).unwrap();

        let result = build_embedded_result(test_file.to_str().unwrap(), "application/pdf").unwrap();

        assert_eq!(result.content.len(), 2);
        assert!(result.is_error.is_none());

        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn build_embedded_result_should_contain_success_message() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test-embed-msg.bin");
        std::fs::write(&test_file, b"test").unwrap();

        let result = build_embedded_result(test_file.to_str().unwrap(), "application/pdf").unwrap();

        if let ContentBlock::TextContent(tc) = &result.content[0] {
            assert!(tc.text.contains("compiled successfully"));
        } else {
            panic!("First content block should be TextContent");
        }

        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn build_embedded_result_should_encode_base64_correctly() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test-embed-base64.bin");
        let test_content = b"%PDF-1.4 fake pdf content";
        std::fs::write(&test_file, test_content).unwrap();

        let result = build_embedded_result(test_file.to_str().unwrap(), "application/pdf").unwrap();

        if let ContentBlock::EmbeddedResource(er) = &result.content[1] {
            if let EmbeddedResourceResource::BlobResourceContents(brc) = &er.resource {
                let decoded =
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &brc.blob)
                        .unwrap();
                assert_eq!(decoded, test_content);
            } else {
                panic!("Resource should be BlobResourceContents");
            }
        } else {
            panic!("Second content block should be EmbeddedResource");
        }

        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn build_embedded_result_should_return_error_when_file_not_found() {
        let result = build_embedded_result("/nonexistent/path/file.pdf", "application/pdf");
        assert!(result.is_err());
    }

    #[test]
    fn build_embedded_result_should_set_png_mime_type() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test-embed-png.png");
        std::fs::write(&test_file, b"\x89PNG fake png content").unwrap();

        let result = build_embedded_result(test_file.to_str().unwrap(), "image/png").unwrap();

        if let ContentBlock::EmbeddedResource(er) = &result.content[1] {
            if let EmbeddedResourceResource::BlobResourceContents(brc) = &er.resource {
                assert_eq!(brc.mime_type, Some("image/png".to_string()));
            } else {
                panic!("Expected BlobResourceContents");
            }
        } else {
            panic!("Expected EmbeddedResource");
        }

        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn build_embedded_result_should_set_svg_mime_type() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test-embed-svg.svg");
        std::fs::write(&test_file, b"<svg></svg>").unwrap();

        let result = build_embedded_result(test_file.to_str().unwrap(), "image/svg+xml").unwrap();

        if let ContentBlock::EmbeddedResource(er) = &result.content[1] {
            if let EmbeddedResourceResource::BlobResourceContents(brc) = &er.resource {
                assert_eq!(brc.mime_type, Some("image/svg+xml".to_string()));
            } else {
                panic!("Expected BlobResourceContents");
            }
        } else {
            panic!("Expected EmbeddedResource");
        }

        let _ = std::fs::remove_file(&test_file);
    }

    #[tokio::test]
    async fn compile_from_content_should_create_pdf_file() {
        let output = TempFile::new("/tmp/test-compile-output.pdf");

        let result = compile_from_content("= Hello\nWorld", output.path());

        assert!(result.is_ok(), "Compilation should succeed: {:?}", result);
        assert!(
            std::path::Path::new(output.path()).exists(),
            "PDF file should exist"
        );
    }

    #[tokio::test]
    async fn compile_with_embed_true_should_return_embedded_pdf() {
        let output = TempFile::new("/tmp/test-compile-embed.pdf");

        let compile = Compile {
            file: None,
            content: Some("= Test\nEmbedded test".to_string()),
            output: Some(output.path().to_string()),
            embed: Some(true),
        };

        let state = Arc::new(Mutex::new(AppState {
            template_path: "./templates".to_string(),
        }));

        let result = compile.call_tool(state).await.unwrap();

        assert_eq!(result.content.len(), 2);
        assert!(result.is_error.is_none());

        if let ContentBlock::EmbeddedResource(er) = &result.content[1] {
            if let EmbeddedResourceResource::BlobResourceContents(brc) = &er.resource {
                assert_eq!(brc.mime_type, Some("application/pdf".to_string()));
                assert!(!brc.blob.is_empty());

                let decoded =
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &brc.blob)
                        .unwrap();
                assert!(
                    decoded.starts_with(b"%PDF"),
                    "Should be a valid PDF, starts with: {:?}",
                    &decoded[..4.min(decoded.len())]
                );
            } else {
                panic!("Expected BlobResourceContents");
            }
        } else {
            panic!("Expected EmbeddedResource");
        }
    }

    #[tokio::test]
    async fn compile_with_embed_false_should_return_text_only() {
        let output = TempFile::new("/tmp/test-compile-no-embed.pdf");

        let compile = Compile {
            file: None,
            content: Some("= Test\nNo embed test".to_string()),
            output: Some(output.path().to_string()),
            embed: Some(false),
        };

        let state = Arc::new(Mutex::new(AppState {
            template_path: "./templates".to_string(),
        }));

        let result = compile.call_tool(state).await.unwrap();

        assert_eq!(result.content.len(), 1);

        if let ContentBlock::TextContent(tc) = &result.content[0] {
            assert!(tc.text.contains("compiled successfully"));
            assert!(tc.text.contains(output.path()));
        } else {
            panic!("Expected TextContent");
        }
    }
}
