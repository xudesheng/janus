# MCP Agent Surface Review 5

- Baseline SHA: `9da52a786c71beb13155a90b65b863f64ead4498`
- Current milestone: Milestone 7 Agent Surface V1, a reviewed local agent-facing `get_evidence_bundle` tool surface with strict MCP-facing schemas and a deterministic fixture-backed stdio smoke path.
- Critical path: yes - this round hardens the stdio MCP wrapper so a real MCP client can call the V1 surface without tripping on optional envelope metadata.
- Milestone progress: accepted and ignored MCP/JSON-RPC wrapper metadata, preserved strict validation for tool arguments, corrected invalid-request error classification, shared the MCP protocol-version constant, documented the fixed V1 protocol-version behavior, and expanded stdio regression coverage.
- Deferred milestone work: none; cross-version MCP protocol negotiation is documented as out of V1 scope until Janus targets a client that requires a different protocol version.

Round 4's review direction was "Continue - one short, bounded protocol-hardening round, then stop." This round implements that hardening pass.

## Response To Review 4

- G1: removed `deny_unknown_fields` from the JSON-RPC request envelope and `tools/call` params so client-supplied wrapper fields such as `_meta` are accepted and ignored. The inner `GetEvidenceBundleToolInput` remains strict.
- G2: documented the V1 limitation in `docs/core/mcp-agent-surface.md`: the local stdio `initialize` response pins protocol version `2025-11-25`, and cross-version negotiation is deferred until a client requires it.
- G3: changed request handling to parse JSON first, then deserialize the request object. Malformed JSON still returns `-32700`; valid JSON that is not a valid request now returns `-32600`.
- G4: moved `MCP_PROTOCOL_VERSION` into the library MCP module and reused it from both the stdio server and stdio tests.

## Implementation Summary

Implemented:

- `src/bin/janus_mcp.rs`
  - accepts unknown JSON-RPC/MCP wrapper fields;
  - accepts unknown `tools/call` param fields such as `_meta`;
  - separates JSON parse errors from invalid request objects;
  - uses the shared MCP protocol-version constant.
- `tests/mcp_stdio.rs`
  - keeps the original real-process initialize/list/call smoke path;
  - adds a `_meta` interop test for both top-level request metadata and `tools/call` params metadata;
  - adds a regression test that invalid request envelopes return `-32600`;
  - adds a regression test proving `_meta` inside actual tool arguments is still rejected as a strict tool error.
- `src/mcp.rs`
  - exports `MCP_PROTOCOL_VERSION`.
- `docs/core/mcp-agent-surface.md`
  - records the fixed V1 protocol-version behavior and defers cross-version negotiation.

## Review Focus

Please focus on the stop/continue decision:

1. Does accepting wrapper metadata while preserving strict tool arguments close the real-client interop defect from G1?
2. Is documenting a fixed V1 protocol version acceptable for G2, or should V1 implement protocol negotiation before the milestone stops?
3. Does the new invalid-request handling close G3 without adding unnecessary JSON-RPC machinery?
4. Does sharing `MCP_PROTOCOL_VERSION` close G4 sufficiently?
5. If these are acceptable, does Round Termination apply now: no actionable feedback remains and Milestone 7 V1 is complete?

## Verification

All verification passed:

```bash
cargo run --bin generate_schemas
cargo fmt --check
cargo test --test mcp_stdio --test mcp_agent_surface
cargo test
cargo clippy --all-targets --all-features
cargo run --bin validate_fixtures
```

Notable focused results:

- `cargo test --test mcp_stdio --test mcp_agent_surface`: 4 stdio tests passed; 16 MCP handler tests passed.
- `cargo test`: full suite passed.
- `cargo run --bin validate_fixtures`: 0 errors, 0 warnings.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
