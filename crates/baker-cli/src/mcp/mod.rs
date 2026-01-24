//! MCP (Model Context Protocol) server support for Baker.
//!
//! This module provides an MCP server that exposes Baker's template functionality
//! as tools that can be used by AI assistants and other MCP clients.
//!
//! ## Tools
//!
//! - `list_templates`: Lists all installed templates with their descriptions and usage information
//! - `generate`: Generates a project from a template with provided answers
//!
//! ## Usage
//!
//! Start the MCP server with:
//! ```bash
//! baker mcp
//! ```
//!
//! The server communicates over stdio using the MCP protocol.

mod handler;
mod server;
mod tools;

pub use handler::BakerHandler;
pub use server::run_mcp_server;
pub use tools::{GenerateTool, ListTemplatesTool, QuestionInfo, TemplateInfo};
