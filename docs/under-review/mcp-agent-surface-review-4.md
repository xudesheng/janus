# MCP Agent Surface Review 4

- Baseline SHA: `6060444fdf4b182429aa4a21cce361b7f2055148`
- Current milestone: Milestone 7 Agent Surface V1, a reviewed local agent-facing `get_evidence_bundle` tool surface with strict MCP-facing schemas and a deterministic fixture-backed stdio smoke path.
- Critical path: yes - this round implements the stdio MCP completion target required by the design Definition Of Done and folds in review-3's minor findings.
- Milestone progress: added the `janus_mcp` stdio server, covered a real `initialize` / `tools/list` / `tools/call` exchange, shared public requirement constants, removed internal hot-context requirement names from `context_unavailable` tool errors, and expanded MCP handler tests.
- Deferred milestone work: none intentionally deferred for the V1 MCP surface; optional `suspected_causes` and `next_checks` exposure remains out of scope for this milestone.

Round 3's review direction was "Continue" and specifically called for the stdio MCP server plus an `initialize` / `tools/list` / `tools/call` smoke exchange. This round implements that remaining V1 surface and addresses the non-blocking review findings.

## Response To Review 3

- F1: moved evidence requirement strings into shared `src/query.rs` constants and reused those constants from `src/mcp.rs`; added handler-boundary coverage for all three hot-context failure paths.
- F2: `context_unavailable` no longer exposes the internal `hot_context_*` selector name in `ToolError.requirement`; public requirement values remain limited to evidence requirements such as `counter_evidence` and `raw_refs`.
- F3: added explicit `budget_unsatisfied` mapping coverage through `tool_error_from_get_evidence_bundle`.
- F4: exported and reused `query::default_require_raw_refs`, removing the duplicate default in the MCP adapter.

## Implementation Summary

This round adds the local stdio MCP surface:

- `src/bin/janus_mcp.rs`
  - reads newline-delimited JSON-RPC requests from stdin;
  - handles `initialize`, `tools/list`, and `tools/call`;
  - advertises the existing `get_evidence_bundle` tool definition and schemas;
  - wraps successful tool calls with both `structuredContent` and text content;
  - returns tool-level failures as MCP tool results with `isError: true`.
- `tests/mcp_stdio.rs`
  - spawns the compiled `janus_mcp` binary;
  - sends a real three-message stdio exchange;
  - asserts the initialize capability response, tool definition shape, structured bundle output, and text content.
- `tests/mcp_agent_surface.rs`
  - now covers the three hot-context mapping paths without leaking `requirement`;
  - now covers `budget_unsatisfied` error mapping.
- `src/query.rs` and `src/mcp.rs`
  - share requirement constants and `default_require_raw_refs`;
  - keep MCP error mapping stable and public-facing.

## Review Focus

Please focus on whether this completes the V1 agent surface:

1. Does the stdio MCP server satisfy the design's local invocation Definition Of Done for `initialize`, `tools/list`, and `tools/call`?
2. Is the JSON-RPC/MCP result shape acceptable for V1, especially `structuredContent`, text content, and `isError` handling?
3. Are review-3 findings F1-F4 fully addressed without widening the public tool contract unnecessarily?
4. Can Milestone 7 be considered complete after this round, or is another focused protocol-hardening round needed before stopping the review loop?

## Verification

All verification passed:

```bash
cargo run --bin generate_schemas
cargo fmt --check
cargo test --test mcp_agent_surface --test mcp_stdio
cargo test
cargo clippy --all-targets --all-features
cargo run --bin validate_fixtures
```

Notable focused results:

- `cargo test --test mcp_agent_surface --test mcp_stdio`: 16 MCP handler tests passed; 1 stdio smoke test passed.
- `cargo run --bin validate_fixtures`: 0 errors, 0 warnings.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
