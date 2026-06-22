use janus::mcp::{
    GET_EVIDENCE_BUNDLE_TOOL_NAME, call_get_evidence_bundle, get_evidence_bundle_tool_definition,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};

const MCP_PROTOCOL_VERSION: &str = "2025-11-25";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonRpcRequest {
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolCallParams {
    name: String,
    #[serde(default)]
    arguments: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    serve_stdio(io::stdin().lock(), io::stdout().lock())?;
    Ok(())
}

fn serve_stdio<R: BufRead, W: Write>(reader: R, mut writer: W) -> io::Result<()> {
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        if let Some(response) = handle_line(&line) {
            serde_json::to_writer(&mut writer, &response)?;
            writeln!(writer)?;
            writer.flush()?;
        }
    }

    Ok(())
}

fn handle_line(line: &str) -> Option<JsonRpcResponse> {
    match serde_json::from_str::<JsonRpcRequest>(line) {
        Ok(request) => handle_request(request),
        Err(error) => Some(error_response(
            Value::Null,
            -32700,
            format!("parse error: {error}"),
        )),
    }
}

fn handle_request(request: JsonRpcRequest) -> Option<JsonRpcResponse> {
    let id = request.id?;

    if request.jsonrpc != "2.0" {
        return Some(error_response(
            id,
            -32600,
            "invalid JSON-RPC version".to_string(),
        ));
    }

    match request.method.as_str() {
        "initialize" => Some(success_response(id, initialize_result())),
        "tools/list" => Some(success_response(id, tools_list_result())),
        "tools/call" => Some(handle_tools_call(id, request.params)),
        method => Some(error_response(
            id,
            -32601,
            format!("method not found: {method}"),
        )),
    }
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": MCP_PROTOCOL_VERSION,
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "janus",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

fn tools_list_result() -> Value {
    json!({
        "tools": [
            get_evidence_bundle_tool_definition()
        ]
    })
}

fn handle_tools_call(id: Value, params: Option<Value>) -> JsonRpcResponse {
    let Some(params) = params else {
        return error_response(id, -32602, "missing tools/call params".to_string());
    };

    let params = match serde_json::from_value::<ToolCallParams>(params) {
        Ok(params) => params,
        Err(error) => {
            return error_response(id, -32602, format!("invalid tools/call params: {error}"));
        }
    };

    if params.name != GET_EVIDENCE_BUNDLE_TOOL_NAME {
        return error_response(id, -32602, format!("unknown tool: {}", params.name));
    }

    let result = match call_get_evidence_bundle(params.arguments) {
        Ok(output) => {
            let structured = serde_json::to_value(output).expect("tool output should serialize");
            tool_success_result(structured)
        }
        Err(error) => {
            tool_error_result(serde_json::to_value(error).expect("tool error should serialize"))
        }
    };

    success_response(id, result)
}

fn tool_success_result(structured_content: Value) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string(&structured_content)
                    .expect("structured content should serialize")
            }
        ],
        "structuredContent": structured_content,
        "isError": false
    })
}

fn tool_error_result(error: Value) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string(&error).expect("tool error should serialize")
            }
        ],
        "isError": true
    })
}

fn success_response(id: Value, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(result),
        error: None,
    }
}

fn error_response(id: Value, code: i32, message: String) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: None,
        error: Some(JsonRpcError { code, message }),
    }
}
