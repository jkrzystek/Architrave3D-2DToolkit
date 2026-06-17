use crate::error::{BridgeError, BridgeResult};
use crate::provider::{AiProvider, ResourceContent, ResourceDescriptor, ToolDescriptor, ToolResult};
use serde_json::Value;

/// Collects all registered [`AiProvider`]s and routes requests by namespace.
///
/// This is the single integration point that protocol servers (MCP, future
/// protocols) use to access all module functionality.
pub struct BridgeRegistry {
    providers: Vec<Box<dyn AiProvider>>,
}

impl BridgeRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn register(&mut self, provider: impl AiProvider + 'static) {
        self.providers.push(Box::new(provider));
    }

    pub fn list_all_resources(&self) -> Vec<ResourceDescriptor> {
        self.providers
            .iter()
            .flat_map(|p| p.list_resources())
            .collect()
    }

    pub fn list_all_tools(&self) -> Vec<ToolDescriptor> {
        self.providers
            .iter()
            .flat_map(|p| p.list_tools())
            .collect()
    }

    pub fn read_resource(&self, uri: &str) -> BridgeResult<ResourceContent> {
        for provider in &self.providers {
            match provider.read_resource(uri) {
                Ok(content) => return Ok(content),
                Err(BridgeError::ResourceNotFound(_)) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(BridgeError::ResourceNotFound(uri.to_string()))
    }

    pub fn call_tool(&self, name: &str, arguments: Value) -> BridgeResult<ToolResult> {
        let namespace = name.split('.').next().unwrap_or("");
        for provider in &self.providers {
            if provider.namespace() == namespace {
                return provider.call_tool(name, arguments);
            }
        }
        Err(BridgeError::ToolNotFound(name.to_string()))
    }

    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    pub fn namespaces(&self) -> Vec<&str> {
        self.providers.iter().map(|p| p.namespace()).collect()
    }
}

impl Default for BridgeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ResourceContent, ToolResultContent};

    struct MockProvider;

    impl AiProvider for MockProvider {
        fn namespace(&self) -> &str { "mock" }
        fn description(&self) -> &str { "Test provider" }

        fn list_resources(&self) -> Vec<ResourceDescriptor> {
            vec![ResourceDescriptor::json("mock://status", "Status", "Mock status")]
        }

        fn list_tools(&self) -> Vec<ToolDescriptor> {
            vec![ToolDescriptor::new(
                "mock.ping",
                "Returns pong",
                serde_json::json!({"type": "object", "properties": {}}),
            )]
        }

        fn read_resource(&self, uri: &str) -> BridgeResult<ResourceContent> {
            match uri {
                "mock://status" => ResourceContent::json(uri, &serde_json::json!({"ok": true})),
                _ => Err(BridgeError::ResourceNotFound(uri.to_string())),
            }
        }

        fn call_tool(&self, name: &str, _args: Value) -> BridgeResult<ToolResult> {
            match name {
                "mock.ping" => Ok(ToolResult::success_text("pong")),
                _ => Err(BridgeError::ToolNotFound(name.to_string())),
            }
        }
    }

    #[test]
    fn register_and_list() {
        let mut registry = BridgeRegistry::new();
        registry.register(MockProvider);
        assert_eq!(registry.provider_count(), 1);
        assert_eq!(registry.list_all_resources().len(), 1);
        assert_eq!(registry.list_all_tools().len(), 1);
    }

    #[test]
    fn read_resource_routes() {
        let mut registry = BridgeRegistry::new();
        registry.register(MockProvider);
        let content = registry.read_resource("mock://status").unwrap();
        assert!(content.text.contains("true"));
    }

    #[test]
    fn read_unknown_resource_errors() {
        let mut registry = BridgeRegistry::new();
        registry.register(MockProvider);
        assert!(registry.read_resource("unknown://x").is_err());
    }

    #[test]
    fn call_tool_routes() {
        let mut registry = BridgeRegistry::new();
        registry.register(MockProvider);
        let result = registry.call_tool("mock.ping", Value::Null).unwrap();
        assert!(!result.is_error);
        match &result.content[0] {
            ToolResultContent::Text { text } => assert_eq!(text, "pong"),
        }
    }

    #[test]
    fn call_unknown_tool_errors() {
        let registry = BridgeRegistry::new();
        assert!(registry.call_tool("nope.x", Value::Null).is_err());
    }
}
