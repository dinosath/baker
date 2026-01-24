//! MCP server handler for Baker.

use super::tools::{GenerateTool, ListTemplatesTool};
use async_trait::async_trait;
use rust_mcp_sdk::{
    mcp_server::ServerHandler,
    schema::{
        schema_utils::CallToolError, CallToolRequestParams, CallToolResult, ListToolsResult,
        PaginatedRequestParams, RpcError, TextContent,
    },
    McpServer,
};
use std::sync::Arc;

/// Baker MCP server handler.
#[derive(Default)]
pub struct BakerHandler;

/// Helper to create a text content response.
fn text_content(text: impl Into<String>) -> TextContent {
    TextContent::new(text.into(), None, None)
}

#[async_trait]
impl ServerHandler for BakerHandler {
    /// Handle requests to list available tools.
    async fn handle_list_tools_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            tools: vec![ListTemplatesTool::tool(), GenerateTool::tool()],
            meta: None,
            next_cursor: None,
        })
    }

    /// Handle requests to call a specific tool.
    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<CallToolResult, CallToolError> {
        match params.name.as_str() {
            "list_templates" => {
                match ListTemplatesTool::execute() {
                    Ok(templates) => {
                        let json = serde_json::to_string_pretty(&templates)
                            .unwrap_or_else(|_| "[]".to_string());
                        Ok(CallToolResult::text_content(vec![text_content(json)]))
                    }
                    Err(e) => Ok(CallToolResult::text_content(vec![text_content(format!("Error: {e}"))])),
                }
            }
            "generate" => {
                // Parse the arguments
                let args = params.arguments.ok_or_else(|| {
                    CallToolError::invalid_arguments("generate", Some("Missing arguments".to_string()))
                })?;

                let tool: GenerateTool = serde_json::from_value(serde_json::Value::Object(args))
                    .map_err(|e| CallToolError::invalid_arguments("generate", Some(format!("Invalid arguments: {e}"))))?;

                match tool.execute() {
                    Ok(msg) => Ok(CallToolResult::text_content(vec![text_content(msg)])),
                    Err(e) => Ok(CallToolResult::text_content(vec![text_content(format!("Error: {e}"))])),
                }
            }
            _ => Err(CallToolError::unknown_tool(params.name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_creation() {
        let _handler = BakerHandler::default();
    }
}
