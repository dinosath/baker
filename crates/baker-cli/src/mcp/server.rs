//! MCP server implementation for Baker.

use super::handler::BakerHandler;
use rust_mcp_sdk::{
    error::SdkResult,
    mcp_server::{server_runtime, McpServerOptions, ServerRuntime, ToMcpServerHandler},
    schema::{
        Implementation, InitializeResult, ProtocolVersion, ServerCapabilities,
        ServerCapabilitiesTools,
    },
    McpServer, StdioTransport, TransportOptions,
};
use std::sync::Arc;

/// Run the Baker MCP server over stdio.
pub async fn run_mcp_server() -> SdkResult<()> {
    // Define server details
    let server_details = InitializeResult {
        server_info: Implementation {
            name: "baker".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            title: Some("Baker Project Scaffolding".into()),
            description: Some(
                "MCP server for Baker - a project scaffolding tool. \
                 Use list_templates to discover available templates and \
                 generate to create new projects from templates."
                    .into(),
            ),
            icons: vec![],
            website_url: Some("https://github.com/aliev/baker".into()),
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        meta: None,
        instructions: Some(
            "Baker is a project scaffolding tool. First use list_templates to see available \
             templates and their required variables. Then use generate to create a new project \
             with the answers to the template questions."
                .into(),
        ),
        protocol_version: ProtocolVersion::V2025_11_25.into(),
    };

    // Create stdio transport
    let transport = StdioTransport::new(TransportOptions::default())?;

    // Create handler
    let handler = BakerHandler::default();

    // Create and start server
    let server: Arc<ServerRuntime> = server_runtime::create_server(McpServerOptions {
        server_details,
        transport,
        handler: handler.to_mcp_server_handler(),
        task_store: None,
        client_task_store: None,
    });

    server.start().await
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_server_version() {
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
    }
}
