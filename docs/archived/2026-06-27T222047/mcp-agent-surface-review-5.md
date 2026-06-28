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

## Review (by Claude)

### Direction Verdict

Stop — Milestone 7 Agent Surface V1 is complete. This round did exactly the
bounded hardening I asked for in review 4, closes G1-G4 cleanly, and leaves no
actionable feedback. Under "Round Termination" the loop ends here: do not open a
round 6. The covered work is on the critical path and the milestone artifact (a
reviewed local agent-facing `get_evidence_bundle` stdio surface with strict
MCP-facing schemas and a deterministic fixture-backed smoke path) now exists and is
real-client tolerant.

I reproduced all verification independently rather than trusting the summary:

- `cargo fmt --check` -> clean;
- `cargo clippy --all-targets --all-features` -> no warnings;
- `cargo test --test mcp_stdio --test mcp_agent_surface` -> 4 stdio + 16 handler
  tests passed;
- `cargo test` -> full suite green;
- `cargo run --bin validate_fixtures` -> 0 errors, 0 warnings;
- `cargo run --bin generate_schemas` then `git status` -> clean (no schema drift).

### G1–G4 Verification (in code, not just summary)

- **G1 — closed with the correct boundary.** `deny_unknown_fields` is gone from
  `JsonRpcRequest` (`src/bin/janus_mcp.rs:9-10`) and `ToolCallParams` (`:19-20`),
  so envelope/`tools/call` `_meta` is accepted and ignored, while the inner
  `GetEvidenceBundleToolInput` keeps `deny_unknown_fields` so tool arguments stay
  strict. `janus_mcp_stdio_ignores_mcp_meta_fields` proves `_meta` at both the
  request and `params` level succeeds; `janus_mcp_stdio_keeps_tool_arguments_strict`
  proves `_meta` inside `arguments` is still rejected as `invalid_request`. This is
  precisely the envelope-tolerant / arguments-strict split I recommended.
- **G2 — resolved as accepted.** The design doc now records that V1 pins
  `2025-11-25` and defers cross-version negotiation (`docs/core/mcp-agent-surface.md:132-134`).
  In review 4 I said documenting the fixed version was acceptable for V1, so this
  matches.
- **G3 — closed.** `handle_line` parses JSON first (`-32700` on malformed) then
  deserializes the request object (`-32600` on a valid-JSON-but-invalid request),
  extracting `id` from the raw value first so the error response carries the right
  id (`src/bin/janus_mcp.rs:64-89`). `janus_mcp_stdio_reports_invalid_request_for_bad_envelope`
  confirms `-32600` with the correct id.
- **G4 — closed.** `pub const MCP_PROTOCOL_VERSION` lives in `src/mcp.rs:16` and is
  reused by both the server and `tests/mcp_stdio.rs`; no duplicated literal remains.

### Milestone Completeness

Checked against the design Definition Of Done and roadmap Milestone 7 acceptance;
all satisfied:

- `get_evidence_bundle` exposed through a reviewed agent-facing tool surface;
- input/output schemas committed and drift-tested; strict-validator concerns incl.
  array `items` covered;
- a local stdio MCP server handles a real `initialize` / `tools/list` /
  `tools/call` exchange and returns structured Evidence IR JSON, now without
  tripping on optional client envelope metadata;
- tool errors use stable categories with no Rust debug leakage;
- `scenario_id` required in the V1 schema, documented temporary, `context_selector`
  reserved;
- no new OTel protocol, persistence, dashboard, warm memory, mitigation, or RCA
  prose introduced;
- privacy scope field present (enforcement intentionally minimal);
- inspectable, source-backed evidence rather than an RCA prose contract;
- `cargo fmt` / `cargo test` / `cargo clippy --all-targets --all-features` /
  `cargo run --bin validate_fixtures` all pass.

### No New Findings

I looked for residual issues and found nothing that blocks completion. For the
record, two items are correctly out of V1 scope and need no action now: the server
returns `-32601` for unhandled methods such as MCP `ping`, and tool-error results
carry the structured `ToolError` as text content without a separate
`structuredContent` block. Both are acceptable for the V1 local smoke surface and
align with the design's stated scope; raising them would be scope creep.

### Answers To This Round's Review Focus

1. Yes — accepting wrapper metadata while keeping tool arguments strict closes the
   G1 real-client interop defect, with tests proving both directions.
2. Yes — documenting a fixed V1 protocol version is acceptable for G2; negotiation
   is reasonably deferred.
3. Yes — the parse-vs-invalid-request split closes G3 without adding unnecessary
   JSON-RPC machinery.
4. Yes — the shared `MCP_PROTOCOL_VERSION` closes G4.
5. Yes — Round Termination applies now: no actionable feedback remains and
   Milestone 7 V1 is complete.

### Process / Framework Check

- Baseline SHA `9da52a7` points to the pushed covered-code commit (`fix: tolerate
  mcp stdio metadata`), not to the review-5 commit `e9a99a2`; correct and frozen.
  Covered code pushed first, review doc as its own commit after.
- Header carries milestone / critical-path / progress / deferred; `## Verification`
  lists commands with results, which I reproduced. Compliant.
- This review reports no defects and no new requirements, and the milestone work is
  complete, so per "Round Termination" the topic stops here. The Implementor should
  not submit a review 6.
- Archiving is not automatic: per the framework I am not moving these files to
  `docs/archived/`; do that only when the User explicitly asks.
