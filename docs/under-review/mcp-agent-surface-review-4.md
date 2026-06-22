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

## Review (by Claude)

### Direction Verdict

Continue — one short, bounded protocol-hardening round, then stop. This round
delivers the stdio MCP server that the design's Definition Of Done required and
cleanly closes review-3's F1-F4; the V1 DoD as written is met and all verification
reproduces green. I am not declaring the loop terminated only because of one real
interop defect (G1) that undermines the milestone's own stated purpose ("an
external agent can call Janus"): `tools/call` rejects MCP-standard `_meta`. That is
a focused fix, not a redirect. If the User scopes `_meta` robustness out of V1, the
alternative is to stop now and track G1 as a follow-up topic — that scoping call is
the User's.

Milestone progress is strong and on the critical path; I judged it before local
defects, below.

I reproduced the verification independently rather than trusting the summary:

- `cargo fmt --check` -> clean;
- `cargo clippy --all-targets --all-features` -> no warnings;
- `cargo test` -> full suite green, including `mcp_agent_surface` (16) and
  `mcp_stdio` (1);
- `cargo run --bin validate_fixtures` -> 0 errors (12 fixtures);
- `cargo run --bin generate_schemas` then `git status` -> clean, so committed
  `schemas/mcp/*` still match generated output (no drift).

### Milestone Progress Assessment

- **stdio completion target met.** `src/bin/janus_mcp.rs` serves newline-delimited
  JSON-RPC over stdin/stdout, handles `initialize` / `tools/list` / `tools/call`,
  advertises the existing tool definition, and `tests/mcp_stdio.rs` spawns the real
  compiled binary and drives a true three-message exchange asserting the
  capability response, tool shape, `structuredContent.bundle`, and text content.
  This satisfies the DoD line that command-only is insufficient.
- **Correct MCP/JSON-RPC semantics on the points that matter.** Notifications
  (no `id`) correctly yield no response (`src/bin/janus_mcp.rs:79`), so the standard
  post-initialize `notifications/initialized` will not choke the loop. Tool
  execution failures are returned as results with `isError: true`
  (`:165-175`), not JSON-RPC errors — the right distinction between protocol and
  tool errors. Success carries both `structuredContent` and text content, matching
  the design.
- **F1-F4 fully closed and verified in code:**
  - **F1:** requirement strings are now shared `pub const`s in `src/query.rs:22-26`
    reused by `is_context_requirement` (`src/mcp.rs:236-243`), giving a compile-time
    link instead of a hand-synced literal set; all three hot-context paths now have
    handler-boundary tests (`tests/mcp_agent_surface.rs:167-205`).
  - **F2:** `context_unavailable` sets `requirement: None` (`src/mcp.rs:140`), and
    each context test asserts `error.requirement == None`. The public requirement
    surface is now limited to `counter_evidence` / `raw_refs`
    (`is_public_evidence_requirement`).
  - **F3:** `unsupported_budget_maps_to_tool_budget_error` covers the
    `budget_unsatisfied` mapping (`tests/mcp_agent_surface.rs:207-218`).
  - **F4:** `default_require_raw_refs` is now `pub` in `query.rs` and reused by the
    adapter; the duplicate is gone.
- **DoD / roadmap acceptance:** tool exposed via reviewed surface; schemas committed
  + drift-tested; strict-validator concerns incl. array `items`; stdio
  initialize/list/call returns structured Evidence IR; stable error categories;
  `scenario_id` required + documented temporary with `context_selector` reserved;
  no new OTel/persistence/dashboard/RCA; all gate commands pass. Roadmap M7
  acceptance (external agent receives structured Evidence IR; strict-validator
  schemas; privacy_scope present; inspectable evidence not RCA prose) is met modulo
  G1.

### Findings

- **G1 (medium — interop; the one thing I'd fix before "real-client ready").**
  `ToolCallParams` is `#[serde(deny_unknown_fields)]` (`src/bin/janus_mcp.rs:22`),
  and `JsonRpcRequest` likewise (`:11`). MCP clients (including Claude's) routinely
  attach an optional `_meta` object to `tools/call` params (e.g.
  `params._meta.progressToken`). With `deny_unknown_fields`, such a call is rejected
  as `-32602 invalid tools/call params`, even though the tool arguments are valid.
  The deterministic local smoke passes only because the test client sends the
  minimal fields. Recommend accepting-and-ignoring `_meta` (or dropping
  `deny_unknown_fields`) on the JSON-RPC envelope and on `tools/call` params so a
  real external agent can actually call the surface. Note the inner tool
  `arguments` should stay strict (`GetEvidenceBundleToolInput` keeps
  `deny_unknown_fields`) — only the JSON-RPC/MCP envelope layer needs to tolerate
  spec-optional fields.

- **G2 (low — protocol negotiation).** `initialize` ignores the client's requested
  `protocolVersion` and always returns `2025-11-25` (`src/bin/janus_mcp.rs:101-112`).
  Acceptable for a local V1 smoke, but a client on a different protocol version gets
  no negotiation signal. Worth at least echoing the client's requested version or
  documenting the fixed version as a V1 limitation.

- **G3 (low — JSON-RPC correctness, tied to G1).** Because `JsonRpcRequest` is
  `deny_unknown_fields`, a structurally valid JSON object carrying an unexpected
  envelope member fails deserialization and is reported as `-32700 parse error`
  (`:70-75`). Per JSON-RPC 2.0 a well-formed JSON that is not a valid Request object
  should be `-32600 Invalid Request`, not `-32700`. Relaxing the envelope per G1
  largely removes this; otherwise consider the correct code.

- **G4 (nit — duplication).** The protocol version `2025-11-25` lives as
  `MCP_PROTOCOL_VERSION` in the server and is also hardcoded in `tests/mcp_stdio.rs`
  and the design doc. Minor; a single shared constant would prevent silent skew.

### Answers To This Round's Review Focus

1. Yes — the stdio server satisfies the DoD's local `initialize` / `tools/list` /
   `tools/call` invocation, proven by a real-process smoke test.
2. The result shape (`structuredContent` + text content + `isError`) is correct and
   acceptable for V1; the gap is envelope strictness (G1/G3), not result shape.
3. Yes — F1-F4 are fully addressed without widening the public contract; F2 actually
   narrows it correctly (requirement now omitted for context errors).
4. As scoped (deterministic local smoke), Milestone 7 V1 is functionally complete;
   but I would not call the surface real-client-ready until G1 is fixed. My
   recommendation is one focused hardening round (G1, plus the G3 nit), then stop —
   unless the User explicitly scopes `_meta` robustness out of V1, in which case
   stop now and track G1 separately.

### Process / Framework Check

- Baseline SHA `6060444` points to the pushed covered-code commit (`feat: add stdio
  mcp server`), not to the review-4 commit `1555cf3`; correct and frozen. Covered
  code pushed first, review doc as its own commit after.
- Header carries milestone / critical-path / progress / deferred; `## Verification`
  lists commands with results, which I reproduced. Compliant.
- This review leaves actionable feedback (G1), so under "Round Termination" the loop
  does not terminate yet: either a bounded round 5 fixes G1, or the User scopes it
  out and the topic stops with G1 recorded as a follow-up. I am not emitting an
  empty round; I am stating the one concrete item that gates a clean stop.
