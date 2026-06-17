use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::BridgeResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDescriptor {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
}

impl ResourceDescriptor {
    pub fn json(uri: impl Into<String>, name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            description: description.into(),
            mime_type: "application/json".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

impl ToolDescriptor {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContent {
    pub uri: String,
    pub mime_type: String,
    pub text: String,
}

impl ResourceContent {
    pub fn json(uri: impl Into<String>, value: &Value) -> BridgeResult<Self> {
        Ok(Self {
            uri: uri.into(),
            mime_type: "application/json".into(),
            text: serde_json::to_string_pretty(value)?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: Vec<ToolResultContent>,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolResultContent {
    #[serde(rename = "text")]
    Text { text: String },
}

impl ToolResult {
    pub fn success_json(value: &Value) -> BridgeResult<Self> {
        Ok(Self {
            content: vec![ToolResultContent::Text {
                text: serde_json::to_string_pretty(value)?,
            }],
            is_error: false,
        })
    }

    pub fn success_text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolResultContent::Text { text: text.into() }],
            is_error: false,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![ToolResultContent::Text {
                text: message.into(),
            }],
            is_error: true,
        }
    }
}

/// Protocol-agnostic interface for exposing module functionality to AI systems.
///
/// Each module adapter implements this trait to declare what resources (readable
/// data) and tools (callable actions) it provides. The bridge registry collects
/// providers and a protocol server (MCP, or any future protocol) maps protocol
/// messages to these methods.
pub trait AiProvider: Send + Sync {
    /// Unique namespace for this provider (e.g. "document", "camera", "graph").
    fn namespace(&self) -> &str;

    /// Human-readable description of what this provider exposes.
    fn description(&self) -> &str;

    /// List all resources this provider can serve.
    fn list_resources(&self) -> Vec<ResourceDescriptor>;

    /// List all tools this provider exposes.
    fn list_tools(&self) -> Vec<ToolDescriptor>;

    /// Read a specific resource by URI.
    fn read_resource(&self, uri: &str) -> BridgeResult<ResourceContent>;

    /// Call a tool by name with JSON arguments.
    fn call_tool(&self, name: &str, arguments: Value) -> BridgeResult<ToolResult>;
}
