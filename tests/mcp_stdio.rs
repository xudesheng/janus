use janus::mcp::MCP_PROTOCOL_VERSION;
use serde_json::{Value, json};
use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn janus_mcp_stdio_handles_initialize_list_and_call() {
    let responses = run_janus_mcp(&[
        initialize_request(),
        tools_list_request(),
        tools_call_request(3),
    ]);

    assert_eq!(responses.len(), 3);
    assert_eq!(responses[0]["id"], json!(1));
    assert_eq!(
        responses[0]["result"]["protocolVersion"],
        MCP_PROTOCOL_VERSION
    );
    assert!(responses[0]["result"]["capabilities"]["tools"].is_object());

    assert_eq!(responses[1]["id"], json!(2));
    assert_eq!(
        responses[1]["result"]["tools"][0]["name"],
        "get_evidence_bundle"
    );
    assert!(responses[1]["result"]["tools"][0]["inputSchema"].is_object());
    assert!(responses[1]["result"]["tools"][0]["outputSchema"].is_object());

    assert_successful_bundle_response(&responses[2], 3);
}

#[test]
fn janus_mcp_stdio_ignores_mcp_meta_fields() {
    let responses = run_janus_mcp(&[tools_call_with_meta_request()]);

    assert_eq!(responses.len(), 1);
    assert_successful_bundle_response(&responses[0], 4);
}

#[test]
fn janus_mcp_stdio_reports_invalid_request_for_bad_envelope() {
    let responses = run_janus_mcp(&[json!({
        "jsonrpc": "2.0",
        "id": 5,
        "unexpected": true
    })]);

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0]["id"], json!(5));
    assert_eq!(responses[0]["error"]["code"], -32600);
    assert!(
        responses[0]["error"]["message"]
            .as_str()
            .is_some_and(|message| message.starts_with("invalid request:"))
    );
}

#[test]
fn janus_mcp_stdio_keeps_tool_arguments_strict() {
    let mut request = tools_call_request(6);
    request["params"]["arguments"]["_meta"] = json!({
        "ignored": false
    });

    let responses = run_janus_mcp(&[request]);

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0]["id"], json!(6));
    assert_eq!(responses[0]["result"]["isError"], true);
    assert!(
        responses[0]["result"]["content"][0]["text"]
            .as_str()
            .is_some_and(|text| text.contains("\"code\":\"invalid_request\""))
    );
}

fn run_janus_mcp(requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_janus_mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn janus_mcp");

    {
        let stdin = child.stdin.as_mut().expect("child stdin");
        for request in requests {
            writeln!(stdin, "{request}").unwrap();
        }
    }

    let output = child.wait_with_output().expect("wait for janus_mcp");

    assert!(
        output.status.success(),
        "janus_mcp exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn assert_successful_bundle_response(response: &Value, id: u64) {
    assert_eq!(response["id"], json!(id));
    assert_eq!(response["result"]["isError"], false);
    assert_eq!(
        response["result"]["structuredContent"]["bundle"]["question"],
        "Why did checkout start returning 5xx around 14:05 on 2026-06-01?"
    );
    assert!(
        response["result"]["structuredContent"]["bundle"]["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert_eq!(response["result"]["content"][0]["type"], "text");
    assert!(response["result"]["content"][0]["text"].is_string());
}

fn initialize_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {
                "name": "janus-test",
                "version": "0.0.0"
            }
        }
    })
}

fn tools_list_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    })
}

fn tools_call_request(id: u64) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {
            "name": "get_evidence_bundle",
            "arguments": {
                "scenario_id": "deploy-bad-rollout",
                "intent": {
                    "question": "Why did checkout start returning 5xx around 14:05 on 2026-06-01?"
                },
                "time_window": {
                    "start": "2026-06-01T14:00:00Z",
                    "end": "2026-06-01T14:15:00Z"
                },
                "budget": {
                    "max_items": 5,
                    "max_tokens": 586
                }
            }
        }
    })
}

fn tools_call_with_meta_request() -> Value {
    let mut request = tools_call_request(4);
    request["_meta"] = json!({
        "trace": "client-envelope"
    });
    request["params"]["_meta"] = json!({
        "progressToken": "janus-test-progress"
    });
    request
}
