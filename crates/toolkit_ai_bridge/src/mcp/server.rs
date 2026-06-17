use serde_json::{json, Value};

use crate::registry::BridgeRegistry;

use super::protocol::*;

/// MCP server that translates MCP JSON-RPC messages into BridgeRegistry calls.
///
/// This is one possible transport for the AI bridge. The server is
/// protocol-aware but delegates all actual work to the registry's providers.
/// When a new protocol replaces MCP, only this layer needs replacement.
pub struct McpServer {
    registry: BridgeRegistry,
    server_name: String,
    server_version: String,
    initialized: bool,
}

impl McpServer {
    pub fn new(registry: BridgeRegistry) -> Self {
        Self {
            registry,
            server_name: "3dRustToolkit".into(),
            server_version: "0.1.0".into(),
            initialized: false,
        }
    }

    pub fn with_info(mut self, name: impl Into<String>, version: impl Into<String>) -> Self {
        self.server_name = name.into();
        self.server_version = version.into();
        self
    }

    /// Process a single JSON-RPC request and return a response.
    pub fn handle_message(&mut self, request: &JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request),
            "initialized" => JsonRpcResponse::success(request.id.clone(), json!({})),
            "ping" => JsonRpcResponse::success(request.id.clone(), json!({})),
            "tools/list" => self.handle_tools_list(request),
            "tools/call" => self.handle_tools_call(request),
            "resources/list" => self.handle_resources_list(request),
            "resources/read" => self.handle_resources_read(request),
            _ => JsonRpcResponse::error(
                request.id.clone(),
                METHOD_NOT_FOUND,
                format!("unknown method: {}", request.method),
            ),
        }
    }

    /// Run the server over stdio (newline-delimited JSON-RPC).
    pub fn run_stdio(&mut self) -> std::io::Result<()> {
        use std::io::{self, BufRead, Write};

        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut stdout = stdout.lock();

        for line in stdin.lock().lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let response = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
                Ok(request) => self.handle_message(&request),
                Err(e) => JsonRpcResponse::error(
                    None,
                    PARSE_ERROR,
                    format!("parse error: {e}"),
                ),
            };

            let out = serde_json::to_string(&response).unwrap_or_default();
            writeln!(stdout, "{out}")?;
            stdout.flush()?;
        }

        Ok(())
    }

    fn handle_initialize(&mut self, request: &JsonRpcRequest) -> JsonRpcResponse {
        self.initialized = true;
        let result = InitializeResult {
            protocol_version: "2024-11-05".into(),
            capabilities: ServerCapabilities {
                resources: Some(ResourcesCapability {
                    subscribe: false,
                    list_changed: false,
                }),
                tools: Some(ToolsCapability {
                    list_changed: false,
                }),
            },
            server_info: ServerInfo {
                name: self.server_name.clone(),
                version: self.server_version.clone(),
            },
        };
        JsonRpcResponse::success(
            request.id.clone(),
            serde_json::to_value(result).unwrap(),
        )
    }

    fn handle_tools_list(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        let tools: Vec<Value> = self
            .registry
            .list_all_tools()
            .into_iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": t.input_schema,
                })
            })
            .collect();

        JsonRpcResponse::success(request.id.clone(), json!({ "tools": tools }))
    }

    fn handle_tools_call(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        let params = match &request.params {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(
                    request.id.clone(),
                    INVALID_PARAMS,
                    "missing params",
                )
            }
        };

        let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

        match self.registry.call_tool(name, arguments) {
            Ok(result) => {
                let content: Vec<Value> = result
                    .content
                    .into_iter()
                    .map(|c| serde_json::to_value(c).unwrap())
                    .collect();
                JsonRpcResponse::success(
                    request.id.clone(),
                    json!({
                        "content": content,
                        "isError": result.is_error,
                    }),
                )
            }
            Err(e) => JsonRpcResponse::success(
                request.id.clone(),
                json!({
                    "content": [{"type": "text", "text": e.to_string()}],
                    "isError": true,
                }),
            ),
        }
    }

    fn handle_resources_list(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        let resources: Vec<Value> = self
            .registry
            .list_all_resources()
            .into_iter()
            .map(|r| {
                json!({
                    "uri": r.uri,
                    "name": r.name,
                    "description": r.description,
                    "mimeType": r.mime_type,
                })
            })
            .collect();

        JsonRpcResponse::success(request.id.clone(), json!({ "resources": resources }))
    }

    fn handle_resources_read(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        let params = match &request.params {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(
                    request.id.clone(),
                    INVALID_PARAMS,
                    "missing params",
                )
            }
        };

        let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");

        match self.registry.read_resource(uri) {
            Ok(content) => JsonRpcResponse::success(
                request.id.clone(),
                json!({
                    "contents": [{
                        "uri": content.uri,
                        "mimeType": content.mime_type,
                        "text": content.text,
                    }]
                }),
            ),
            Err(e) => JsonRpcResponse::error(
                request.id.clone(),
                INVALID_PARAMS,
                e.to_string(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{AiProvider, ResourceContent, ResourceDescriptor, ToolDescriptor, ToolResult};
    use crate::error::{BridgeError, BridgeResult};

    struct TestProvider;

    impl AiProvider for TestProvider {
        fn namespace(&self) -> &str { "test" }
        fn description(&self) -> &str { "Test" }
        fn list_resources(&self) -> Vec<ResourceDescriptor> {
            vec![ResourceDescriptor::json("test://info", "Info", "Test info")]
        }
        fn list_tools(&self) -> Vec<ToolDescriptor> {
            vec![ToolDescriptor::new(
                "test.echo",
                "Echoes input",
                json!({"type": "object", "properties": {"msg": {"type": "string"}}}),
            )]
        }
        fn read_resource(&self, uri: &str) -> BridgeResult<ResourceContent> {
            match uri {
                "test://info" => ResourceContent::json(uri, &json!({"version": 1})),
                _ => Err(BridgeError::ResourceNotFound(uri.into())),
            }
        }
        fn call_tool(&self, name: &str, args: Value) -> BridgeResult<ToolResult> {
            match name {
                "test.echo" => {
                    let msg = args.get("msg").and_then(|v| v.as_str()).unwrap_or("?");
                    Ok(ToolResult::success_text(msg))
                }
                _ => Err(BridgeError::ToolNotFound(name.into())),
            }
        }
    }

    fn make_server() -> McpServer {
        let mut reg = BridgeRegistry::new();
        reg.register(TestProvider);
        McpServer::new(reg)
    }

    fn request(method: &str, params: Option<Value>) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params,
        }
    }

    #[test]
    fn initialize_returns_capabilities() {
        let mut server = make_server();
        let resp = server.handle_message(&request("initialize", None));
        let result = resp.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert!(result["capabilities"]["tools"].is_object());
        assert!(result["capabilities"]["resources"].is_object());
    }

    #[test]
    fn tools_list() {
        let mut server = make_server();
        let resp = server.handle_message(&request("tools/list", None));
        let tools = resp.result.unwrap()["tools"].as_array().unwrap().clone();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "test.echo");
    }

    #[test]
    fn tools_call_success() {
        let mut server = make_server();
        let resp = server.handle_message(&request(
            "tools/call",
            Some(json!({"name": "test.echo", "arguments": {"msg": "hello"}})),
        ));
        let result = resp.result.unwrap();
        assert_eq!(result["isError"], false);
        assert_eq!(result["content"][0]["text"], "hello");
    }

    #[test]
    fn resources_list() {
        let mut server = make_server();
        let resp = server.handle_message(&request("resources/list", None));
        let resources = resp.result.unwrap()["resources"].as_array().unwrap().clone();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0]["uri"], "test://info");
    }

    #[test]
    fn resources_read() {
        let mut server = make_server();
        let resp = server.handle_message(&request(
            "resources/read",
            Some(json!({"uri": "test://info"})),
        ));
        let result = resp.result.unwrap();
        let text = result["contents"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"version\""));
    }

    #[test]
    fn unknown_method_returns_error() {
        let mut server = make_server();
        let resp = server.handle_message(&request("bogus/method", None));
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, METHOD_NOT_FOUND);
    }

    #[test]
    fn ping_returns_empty() {
        let mut server = make_server();
        let resp = server.handle_message(&request("ping", None));
        assert!(resp.error.is_none());
    }
}
