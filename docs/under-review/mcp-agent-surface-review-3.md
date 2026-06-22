# MCP Agent Surface Review 3

- Baseline SHA: `7fcfcbe9b50817baedd6f2ecb8eeddcced01afc8`
- Current milestone: Milestone 7 Agent Surface V1, a reviewed local agent-facing `get_evidence_bundle` tool surface with strict MCP-facing schemas and a deterministic fixture-backed smoke path.
- Critical path: yes - this is the first implementation round after the design gate's latest `Direction Verdict` agreed with the design and left no actionable design feedback.
- Milestone progress: implemented the phase-1 MCP schema artifacts and transport-independent `get_evidence_bundle` handler boundary.
- Deferred milestone work: the stdio MCP server, real `initialize` / `tools/list` / `tools/call` exchange, and smoke-test demo remain for later implementation rounds; optional `suspected_causes` and `next_checks` exposure remains deferred.

Round 2's review by Claude agreed with the design unconditionally and left no actionable design feedback. The milestone implementation was still incomplete, so this round advances to the first implementation slice rather than another design-only round.

## Implementation Summary

This round adds a transport-independent MCP adapter without adding a stdio server yet.

Implemented:

- `src/mcp.rs`
  - `GetEvidenceBundleToolInput` with required V1 `scenario_id`;
  - `GetEvidenceBundleToolOutput` as `{ "bundle": EvidenceBundle }`;
  - `ToolDefinition` for `get_evidence_bundle`;
  - `call_get_evidence_bundle(arguments: serde_json::Value)` handler;
  - stable `ToolError` and `ToolErrorCode` mapping from `GetEvidenceBundleError`.
- `schemas/mcp/get-evidence-bundle.input.schema.json`
  - draft-07 schema;
  - object root;
  - root `required` includes `scenario_id`, `intent`, `time_window`, and `budget`;
  - arrays declare `items`.
- `schemas/mcp/get-evidence-bundle.output.schema.json`
  - draft-07 schema;
  - object root;
  - required `bundle`;
  - nested Evidence IR arrays declare `items`.
- `src/bin/generate_schemas.rs`
  - now regenerates both Evidence IR schemas and MCP-facing schemas.
- `tests/mcp_agent_surface.rs`
  - committed-vs-generated schema drift tests;
  - `scenario_id` required assertion;
  - draft-07 compilation with the Rust `jsonschema` crate;
  - valid input/output schema validation;
  - handler success path;
  - invalid request, unknown fixture, counter-evidence, raw-ref, and hot-context error mapping coverage.
- `Cargo.toml`
  - adds `jsonschema` as a no-default-features dev dependency for strict schema tests.

This intentionally does not complete the full milestone. The design Definition of Done still requires a local stdio MCP server that handles a real `initialize`, `tools/list`, and `tools/call` exchange.

## Review Focus

Please focus on the phase-1 boundary:

1. Does the distinct MCP tool input type correctly preserve `EvidenceQuery` semantics while making `scenario_id` required at the tool boundary?
2. Are the committed MCP schemas strict enough for this slice, especially root object shape, root required fields, draft-07 declaration, and array `items`?
3. Is the `jsonschema` dev dependency acceptable as the representative strict draft-07 validator for schema tests?
4. Is the transport-independent handler boundary shaped well enough for the next stdio MCP server slice?
5. Does the tool-error mapping preserve stable public categories without leaking Rust debug strings?
6. Is the scope still small enough: one `get_evidence_bundle` tool, `{ "bundle": EvidenceBundle }` output only, no stdio server yet, no optional companion outputs?

## Verification

All verification passed:

```bash
cargo run --bin generate_schemas
cargo fmt --check
cargo test --test mcp_agent_surface
cargo test
cargo clippy --all-targets --all-features
cargo run --bin validate_fixtures
```

Notable focused result:

- `cargo test --test mcp_agent_surface`: 13 passed.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
