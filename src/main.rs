use std::sync::Arc;

use async_trait::async_trait;
use rust_mcp_schema::{
    CompleteResult, ListResourceTemplatesResult, ListResourcesResult, ReadResourceRequestParams,
    ReadResourceResult,
};
use rust_mcp_sdk::mcp_server::{
    HyperServerOptions, McpServerOptions, ServerHandler, ServerRuntime, ToMcpServerHandler,
    hyper_server, server_runtime,
};
use rust_mcp_sdk::schema::{
    CallToolRequestParams, CallToolResult, CompleteRequestParams, Implementation, InitializeResult,
    ListToolsResult, PaginatedRequestParams, ProtocolVersion, RpcError, ServerCapabilities,
    ServerCapabilitiesTools, schema_utils::CallToolError,
};
use rust_mcp_sdk::{McpServer, StdioTransport, TransportOptions};
use tokio::sync::Mutex;

/// Shared application state available to all tool handlers.
pub struct AppState {
    /// Directory containing `.typ` template files.
    pub template_path: String,
    /// Optional directory for compiled output files (PDF, PNG, SVG).
    pub output_path: Option<String>,
}

pub struct TypstServerHandler {
    pub state: Arc<Mutex<AppState>>,
}

#[async_trait]
impl ServerHandler for TypstServerHandler {
    async fn handle_list_tools_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            meta: None,
            next_cursor: None,
            tools: tools::TypstTools::tools(),
        })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let tool_params = tools::TypstTools::try_from(params).map_err(CallToolError::new)?;

        let state = self.state.clone();

        match tool_params {
            tools::TypstTools::Compile(tool) => tool.call_tool(state).await,
            tools::TypstTools::ListTemplates(tool) => tool.call_tool(state).await,
            tools::TypstTools::CompileTemplate(tool) => tool.call_tool(state).await,
            tools::TypstTools::Export(tool) => tool.call_tool(state).await,
            tools::TypstTools::Query(tool) => tool.call_tool(state).await,
            tools::TypstTools::ListFonts(tool) => tool.call_tool(state).await,
        }
    }

    async fn handle_list_resources_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ListResourcesResult, RpcError> {
        Ok(ListResourcesResult {
            meta: None,
            next_cursor: None,
            resources: vec![],
        })
    }

    async fn handle_list_resource_templates_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ListResourceTemplatesResult, RpcError> {
        Ok(ListResourceTemplatesResult {
            meta: None,
            next_cursor: None,
            resource_templates: vec![],
        })
    }

    async fn handle_read_resource_request(
        &self,
        _params: ReadResourceRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ReadResourceResult, RpcError> {
        Err(RpcError::method_not_found())
    }

    async fn handle_complete_request(
        &self,
        _params: CompleteRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<CompleteResult, RpcError> {
        Err(RpcError::method_not_found())
    }
}

fn build_server_details() -> InitializeResult {
    InitializeResult {
        server_info: Implementation {
            name: "MCP Typst Server".into(),
            version: "0.1.0".into(),
            title: Some("MCP Typst Server".into()),
            description: Some("Compile Typst documents to PDF with template support.".into()),
            icons: vec![],
            website_url: None,
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            resources: None,
            completions: None,
            tasks: None,
            ..Default::default()
        },
        meta: None,
        instructions: Some("Use this server to compile Typst documents to PDF. Templates can be stored in a configurable directory.".into()),
        protocol_version: ProtocolVersion::V2025_11_25.into(),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let template_path =
        std::env::var("TYPST_TEMPLATE_PATH").unwrap_or_else(|_| "./templates".to_string());

    let output_path = std::env::var("TYPST_OUTPUT_PATH").ok();

    let state = Arc::new(Mutex::new(AppState {
        template_path,
        output_path,
    }));

    let transport = std::env::var("MCP_TRANSPORT").unwrap_or_else(|_| "stdio".to_string());

    match transport.as_str() {
        "stdio" => run_stdio(state).await,
        "http" => run_http(state).await,
        _ => {
            eprintln!("Unknown transport: {}. Use 'stdio' or 'http'", transport);
            std::process::exit(1);
        }
    }
}

async fn run_stdio(state: Arc<Mutex<AppState>>) {
    tracing::info!("Starting MCP Typst server (stdio transport)");

    let server_details = build_server_details();
    let transport = match StdioTransport::new(TransportOptions::default()) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to create transport: {}", e);
            std::process::exit(1);
        }
    };

    let handler = TypstServerHandler { state };

    let server: Arc<ServerRuntime> = server_runtime::create_server(McpServerOptions {
        server_details,
        transport,
        handler: handler.to_mcp_server_handler(),
        task_store: None,
        client_task_store: None,
        message_observer: None,
    });

    if let Err(e) = server.start().await {
        tracing::error!("Server error: {}", e);
        std::process::exit(1);
    }
}

async fn run_http(state: Arc<Mutex<AppState>>) {
    let port: u16 = std::env::var("MCP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3007);

    let host = std::env::var("MCP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

    tracing::info!(
        "Starting MCP Typst server (HTTP transport) on {}:{}",
        host,
        port
    );

    let server_details = build_server_details();
    let handler = TypstServerHandler { state };

    let server = hyper_server::create_server(
        server_details,
        handler.to_mcp_server_handler(),
        HyperServerOptions {
            host,
            port,
            ..Default::default()
        },
    );

    if let Err(e) = server.start().await {
        tracing::error!("Server error: {}", e);
        std::process::exit(1);
    }
}

mod tools;
