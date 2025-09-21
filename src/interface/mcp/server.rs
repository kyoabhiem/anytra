use crate::domain::models::{EnhancedPrompt, EnhancementOptions, Prompt};
use crate::usecases::enhance_prompt::EnhancePrompt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::select;
use tokio::time::sleep;
use tracing::{debug, error, info};

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[serde(default = "default_jsonrpc")]
    #[allow(dead_code)]
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

fn default_jsonrpc() -> String { "2.0".into() }

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolDescription {
    name: String,
    description: String,
    input_schema: Value,
}

#[derive(Debug, Deserialize)]
struct ToolCallParams {
    name: String,
    #[serde(default)]
    arguments: Value,
}

#[derive(Debug, Deserialize)]
struct EnhanceArgs {
    prompt: String,
    #[serde(default)] goal: Option<String>,
    #[serde(default)] style: Option<String>,
    #[serde(default)] tone: Option<String>,
    #[serde(default)] level: Option<u8>,
    #[serde(default)] audience: Option<String>,
    #[serde(default)] language: Option<String>,
}

pub async fn run_stdio_server(usecase: EnhancePrompt, shutdown_timeout: Duration) -> anyhow::Result<()> {
    let mut stdout = io::stdout();
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin).lines();

    info!("MCP stdio server ready");
    let shutting_down = false;

    loop {
        select! {
            line = reader.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if line.trim().is_empty() { continue; }
                        debug!(%line, "stdin line");
                        match serde_json::from_str::<JsonRpcRequest>(&line) {
                            Ok(req) => {
                                let resp = handle_request(&usecase, req).await;
                                let bytes = serde_json::to_vec(&resp)?;
                                stdout.write_all(&bytes).await?;
                                stdout.write_all(b"\n").await?;
                                stdout.flush().await?;
                                if shutting_down { break; }
                            }
                            Err(e) => {
                                let resp = JsonRpcResponse {
                                    jsonrpc: "2.0",
                                    id: None,
                                    result: None,
                                    error: Some(JsonRpcError { code: -32700, message: format!("parse error: {}", e), data: None }),
                                };
                                let bytes = serde_json::to_vec(&resp)?;
                                stdout.write_all(&bytes).await?;
                                stdout.write_all(b"\n").await?;
                                stdout.flush().await?;
                            }
                        }
                    }
                    Ok(None) => { // EOF
                        break;
                    }
                    Err(e) => {
                        error!(error=%e, "error reading stdin");
                        break;
                    }
                }
            }
        }
    }

    if shutting_down {
        sleep(shutdown_timeout).await;
    }

    Ok(())
}

async fn handle_request(usecase: &EnhancePrompt, req: JsonRpcRequest) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" | "mcp/initialize" => JsonRpcResponse {
            jsonrpc: "2.0",
            id: req.id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": { "list": true, "call": true }
                },
                "serverInfo": { "name": "anytra", "version": env!("CARGO_PKG_VERSION") }
            })),
            error: None,
        },

        "tools/list" => {
            let tool = ToolDescription {
                name: "enhance_prompt".into(),
                description: "Enhance a user prompt for clarity, constraints, and specificity".into(),
                input_schema: json!({
                    "$schema": "http://json-schema.org/draft-07/schema#",
                    "type": "object",
                    "required": ["prompt"],
                    "properties": {
                        "prompt": { "type": "string", "description": "The raw prompt to enhance" },
                        "goal": { "type": ["string", "null"], "description": "Desired outcome" },
                        "style": { "type": ["string", "null"], "description": "Writing style (concise, formal, etc.)" },
                        "tone": { "type": ["string", "null"], "description": "Tone (neutral, persuasive, etc.)" },
                        "level": { "type": ["integer", "null"], "minimum": 1, "maximum": 5, "description": "Enhancement strength 1-5" },
                        "audience": { "type": ["string", "null"], "description": "Target audience" },
                        "language": { "type": ["string", "null"], "description": "Output language, e.g., en, id" }
                    }
                }),
            };
            JsonRpcResponse { jsonrpc: "2.0", id: req.id, result: Some(json!({ "tools": [tool] })), error: None }
        }

        "tools/call" => {
            let params: Result<ToolCallParams, _> = serde_json::from_value(req.params.clone());
            match params {
                Ok(p) => {
                    if p.name != "enhance_prompt" {
                        return JsonRpcResponse { jsonrpc: "2.0", id: req.id, result: None, error: Some(JsonRpcError { code: -32601, message: format!("unknown tool: {}", p.name), data: None }) };
                    }
                    let args: Result<EnhanceArgs, _> = serde_json::from_value(p.arguments);
                    match args {
                        Ok(a) => {
                            let opt = EnhancementOptions { goal: a.goal, style: a.style, tone: a.tone, level: a.level, audience: a.audience, language: a.language };
                            let res = usecase.execute(Prompt { text: a.prompt }, opt).await;
                            match res {
                                Ok(EnhancedPrompt { text, rationale: _, .. }) => JsonRpcResponse {
                                    jsonrpc: "2.0",
                                    id: req.id,
                                    result: Some(json!({
                                        "content": [ { "type": "text", "text": text } ]
                                    })),
                                    error: None,
                                },
                                Err(e) => JsonRpcResponse {
                                    jsonrpc: "2.0",
                                    id: req.id,
                                    result: Some(json!({
                                        "content": [ { "type": "text", "text": format!("tool error: {}", e) } ],
                                        "isError": true
                                    })),
                                    error: None,
                                },
                            }
                        }
                        Err(e) => JsonRpcResponse { jsonrpc: "2.0", id: req.id, result: None, error: Some(JsonRpcError { code: -32602, message: format!("invalid arguments: {}", e), data: None }) },
                    }
                }
                Err(e) => JsonRpcResponse { jsonrpc: "2.0", id: req.id, result: None, error: Some(JsonRpcError { code: -32602, message: format!("invalid params: {}", e), data: None }) },
            }
        }

        "ping" => JsonRpcResponse { jsonrpc: "2.0", id: req.id, result: Some(json!({"message": "pong"})), error: None },

        "shutdown" => JsonRpcResponse { jsonrpc: "2.0", id: req.id, result: Some(json!({"ok": true})), error: None },

        unknown => JsonRpcResponse { jsonrpc: "2.0", id: req.id, result: None, error: Some(JsonRpcError { code: -32601, message: format!("unknown method: {}", unknown), data: None }) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm::{LLMError, LLMProvider};
    use crate::domain::models::{EnhancedPrompt, EnhancementOptions, Prompt};
    use async_trait::async_trait;
    use serde_json::json;

    struct MockProvider;

    #[async_trait]
    impl LLMProvider for MockProvider {
        async fn enhance(&self, prompt: Prompt, _options: EnhancementOptions) -> Result<EnhancedPrompt, LLMError> {
            Ok(EnhancedPrompt {
                text: format!("Enhanced: {} - this is a longer text with enough words to pass validation", prompt.text),
                rationale: Some("Test rationale".to_string()),
                confidence: None,
            })
        }
    }

    struct FailingProvider;

    #[async_trait]
    impl LLMProvider for FailingProvider {
        async fn enhance(&self, _prompt: Prompt, _options: EnhancementOptions) -> Result<EnhancedPrompt, LLMError> {
            Err(LLMError::RequestFailed("Test failure".to_string()))
        }
    }

    #[test]
    fn test_default_jsonrpc() {
        assert_eq!(default_jsonrpc(), "2.0");
    }

    #[test]
    fn test_jsonrpc_response_creation() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0",
            id: Some(json!(123)),
            result: Some(json!({"test": "value"})),
            error: None,
        };

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(123)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_jsonrpc_error_creation() {
        let error = JsonRpcError {
            code: -32601,
            message: "Method not found".to_string(),
            data: Some(json!({"method": "unknown"})),
        };

        assert_eq!(error.code, -32601);
        assert_eq!(error.message, "Method not found");
        assert!(error.data.is_some());
    }

    #[test]
    fn test_tool_description_creation() {
        let tool = ToolDescription {
            name: "enhance_prompt".to_string(),
            description: "Enhance a prompt".to_string(),
            input_schema: json!({"type": "object"}),
        };

        assert_eq!(tool.name, "enhance_prompt");
        assert_eq!(tool.description, "Enhance a prompt");
        assert!(tool.input_schema.is_object());
    }

    #[test]
    fn test_tool_call_params_creation() {
        let params = ToolCallParams {
            name: "enhance_prompt".to_string(),
            arguments: json!({"prompt": "test"}),
        };

        assert_eq!(params.name, "enhance_prompt");
        assert!(params.arguments.is_object());
    }

    #[test]
    fn test_enhance_args_creation() {
        let args = EnhanceArgs {
            prompt: "Test prompt".to_string(),
            goal: Some("Test goal".to_string()),
            style: None,
            tone: Some("professional".to_string()),
            level: Some(3),
            audience: None,
            language: Some("en".to_string()),
        };

        assert_eq!(args.prompt, "Test prompt");
        assert_eq!(args.goal.as_deref(), Some("Test goal"));
        assert!(args.style.is_none());
        assert_eq!(args.tone.as_deref(), Some("professional"));
        assert_eq!(args.level, Some(3));
        assert!(args.audience.is_none());
        assert_eq!(args.language.as_deref(), Some("en"));
    }

    #[tokio::test]
    async fn test_handle_initialize() {
        let provider = Box::new(MockProvider);
        let usecase = EnhancePrompt::new(provider);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: json!({}),
        };

        let response = handle_request(&usecase, req).await;

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(1)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        if let Some(result) = response.result {
            assert!(result.get("protocolVersion").is_some());
            assert!(result.get("capabilities").is_some());
            assert!(result.get("serverInfo").is_some());
        }
    }

    #[tokio::test]
    async fn test_handle_tools_list() {
        let provider = Box::new(MockProvider);
        let usecase = EnhancePrompt::new(provider);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: json!({}),
        };

        let response = handle_request(&usecase, req).await;

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(2)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        if let Some(result) = response.result {
            if let Some(tools) = result.get("tools") {
                if let Some(tools_array) = tools.as_array() {
                    assert_eq!(tools_array.len(), 1);
                    if let Some(tool) = tools_array.get(0) {
                        assert_eq!(tool.get("name").unwrap(), "enhance_prompt");
                        assert!(tool.get("description").is_some());
                        assert!(tool.get("input_schema").is_some());
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn test_handle_tools_call_success() {
        let provider = Box::new(MockProvider);
        let usecase = EnhancePrompt::new(provider);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "tools/call".to_string(),
            params: json!({
                "name": "enhance_prompt",
                "arguments": {
                    "prompt": "test prompt",
                    "goal": "test goal"
                }
            }),
        };

        let response = handle_request(&usecase, req).await;

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(3)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        if let Some(result) = response.result {
            if let Some(content) = result.get("content") {
                if let Some(content_array) = content.as_array() {
                    assert!(!content_array.is_empty());
                    if let Some(first_item) = content_array.get(0) {
                        assert_eq!(first_item.get("type").unwrap(), "text");
                        let text = first_item.get("text").unwrap().as_str().unwrap();
                        assert!(text.contains("Enhanced: test prompt"));
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn test_handle_tools_call_error() {
        let provider = Box::new(FailingProvider);
        let usecase = EnhancePrompt::new(provider);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(4)),
            method: "tools/call".to_string(),
            params: json!({
                "name": "enhance_prompt",
                "arguments": {
                    "prompt": "test prompt"
                }
            }),
        };

        let response = handle_request(&usecase, req).await;

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(4)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        if let Some(result) = response.result {
            if let Some(content) = result.get("content") {
                if let Some(content_array) = content.as_array() {
                    assert!(!content_array.is_empty());
                    if let Some(first_item) = content_array.get(0) {
                        assert_eq!(first_item.get("type").unwrap(), "text");
                        let text = first_item.get("text").unwrap().as_str().unwrap();
                        assert!(text.contains("tool error"));
                        assert!(text.contains("Test failure"));
                    }
                }
            }
            assert_eq!(result.get("isError").unwrap(), true);
        }
    }

    #[tokio::test]
    async fn test_handle_unknown_tool() {
        let provider = Box::new(MockProvider);
        let usecase = EnhancePrompt::new(provider);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(5)),
            method: "tools/call".to_string(),
            params: json!({
                "name": "unknown_tool",
                "arguments": {}
            }),
        };

        let response = handle_request(&usecase, req).await;

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(5)));
        assert!(response.result.is_none());
        assert!(response.error.is_some());

        if let Some(error) = response.error {
            assert_eq!(error.code, -32601);
            assert!(error.message.contains("unknown tool"));
        }
    }

    #[tokio::test]
    async fn test_handle_ping() {
        let provider = Box::new(MockProvider);
        let usecase = EnhancePrompt::new(provider);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(6)),
            method: "ping".to_string(),
            params: json!({}),
        };

        let response = handle_request(&usecase, req).await;

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(6)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        if let Some(result) = response.result {
            assert_eq!(result.get("message").unwrap(), "pong");
        }
    }

    #[tokio::test]
    async fn test_handle_shutdown() {
        let provider = Box::new(MockProvider);
        let usecase = EnhancePrompt::new(provider);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(7)),
            method: "shutdown".to_string(),
            params: json!({}),
        };

        let response = handle_request(&usecase, req).await;

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(7)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        if let Some(result) = response.result {
            assert_eq!(result.get("ok").unwrap(), true);
        }
    }

    #[tokio::test]
    async fn test_handle_unknown_method() {
        let provider = Box::new(MockProvider);
        let usecase = EnhancePrompt::new(provider);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(8)),
            method: "unknown_method".to_string(),
            params: json!({}),
        };

        let response = handle_request(&usecase, req).await;

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(8)));
        assert!(response.result.is_none());
        assert!(response.error.is_some());

        if let Some(error) = response.error {
            assert_eq!(error.code, -32601);
            assert!(error.message.contains("unknown method"));
        }
    }

    #[tokio::test]
    async fn test_handle_mcp_initialize() {
        let provider = Box::new(MockProvider);
        let usecase = EnhancePrompt::new(provider);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(9)),
            method: "mcp/initialize".to_string(),
            params: json!({}),
        };

        let response = handle_request(&usecase, req).await;

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(9)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_jsonrpc_request_deserialization() {
        let json_str = r#"{
            "jsonrpc": "2.0",
            "id": 123,
            "method": "tools/list",
            "params": {}
        }"#;

        let req: JsonRpcRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.id, Some(json!(123)));
        assert_eq!(req.method, "tools/list");
        assert!(req.params.is_object());
    }

    #[test]
    fn test_enhance_args_deserialization() {
        let json_str = r#"{
            "prompt": "Test prompt",
            "goal": "Test goal",
            "level": 3
        }"#;

        let args: EnhanceArgs = serde_json::from_str(json_str).unwrap();
        assert_eq!(args.prompt, "Test prompt");
        assert_eq!(args.goal.as_deref(), Some("Test goal"));
        assert_eq!(args.level, Some(3));
        assert!(args.style.is_none());
        assert!(args.tone.is_none());
        assert!(args.audience.is_none());
        assert!(args.language.is_none());
    }
}
