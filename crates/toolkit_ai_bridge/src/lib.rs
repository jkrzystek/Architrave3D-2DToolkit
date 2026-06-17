pub mod error;
pub mod provider;
pub mod registry;
pub mod mcp;
pub mod adapters;

pub use error::{BridgeError, BridgeResult};
pub use provider::{AiProvider, ResourceContent, ResourceDescriptor, ToolDescriptor, ToolResult, ToolResultContent};
pub use registry::BridgeRegistry;
pub use mcp::McpServer;
