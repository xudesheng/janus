use serde_json::{Value, json};
use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn janus_mcp_stdio_handles_initialize_list_and_call() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_janus_mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn janus_mcp");

    {
        let stdin = child.stdin.as_mut().expect("child stdin");
        writeln!(stdin, "{}", initialize_request()).unwrap();
        writeln!(stdin, "{}", tools_list_request()).unwrap();
        writeln!(stdin, "{}", tools_call_request()).unwrap();
    }

    let output = child.wait_with_output().expect("wait for janus_mcp");

    assert!(
        output.status.success(),
        "janus_mcp exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let responses: Vec<Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();

    assert_eq!(responses.len(), 3, "stdout was:\n{stdout}");
    assert_eq!(responses[0]["id"], json!(1));
    assert_eq!(responses[0]["result"]["protocolVersion"], "2025-11-25");
    assert!(responses[0]["result"]["capabilities"]["tools"].is_object());

    assert_eq!(responses[1]["id"], json!(2));
    assert_eq!(
        responses[1]["result"]["tools"][0]["name"],
        "get_evidence_bundle"
    );
    assert!(responses[1]["result"]["tools"][0]["inputSchema"].is_object());
    assert!(responses[1]["result"]["tools"][0]["outputSchema"].is_object());

    assert_eq!(responses[2]["id"], json!(3));
    assert_eq!(responses[2]["result"]["isError"], false);
    assert_eq!(
        responses[2]["result"]["structuredContent"]["bundle"]["question"],
        "Why did checkout start returning 5xx around 14:05 on 2026-06-01?"
    );
    assert!(
        responses[2]["result"]["structuredContent"]["bundle"]["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert_eq!(responses[2]["result"]["content"][0]["type"], "text");
    assert!(responses[2]["result"]["content"][0]["text"].is_string());
}

fn initialize_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
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

fn tools_call_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 3,
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
