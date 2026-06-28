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

## Review (by Claude)

### Direction Verdict

Continue. This is the first implementation round and it is a clean, well-scoped
phase-1 slice that is squarely on the milestone critical path. Milestone progress is
strong: the transport-independent MCP boundary (schema artifacts + handler + error
mapping) is exactly the approved slices 1-2, and it honors every forward-looking
item I named in review 2. No blocking defects; the findings below are all minor and
can be folded into the next slice. Next action: proceed to the stdio MCP server +
`initialize` / `tools/list` / `tools/call` smoke exchange, which the design's
Definition of Done still requires before the topic is complete.

I independently reproduced the verification rather than trusting the summary:

- `cargo test --test mcp_agent_surface` -> 13 passed;
- `cargo test` -> full suite green across all binaries/integration tests;
- `cargo run --bin generate_schemas` then `git status` -> clean, so the committed
  `schemas/mcp/*` match freshly generated output (no drift);
- `cargo clippy --all-targets --all-features` -> no warnings.

### Milestone Progress Assessment

- **B2 closed in code.** `GetEvidenceBundleToolInput` is a distinct type with a
  non-optional `scenario_id` (`src/mcp.rs:26-27`); the committed input schema lists
  `scenario_id` in root `required` (`schemas/mcp/get-evidence-bundle.input.schema.json:5-10`),
  and `mcp_input_schema_requires_scenario_id` asserts it. The `From` impl
  (`src/mcp.rs:186-200`) is field-exhaustive, so any future `EvidenceQuery` field
  addition breaks compilation until the conversion is updated — good compile-time
  safety.
- **Error mapping is total and leak-free.** The `match` over
  `GetEvidenceBundleError` (`src/mcp.rs:101-167`) has no wildcard arm, so all 12
  variants are handled and a new variant forces a compile error. The
  `internal_error` arm emits a fixed string, not `{error:?}`, so no Rust debug
  leaks as public API. The `context_unavailable` vs `requirement_unsatisfied` split
  is consistent: the three `hot_context_*` requirement strings in
  `is_context_requirement` (`src/mcp.rs:238-243`) exactly match those produced in
  `ensure_query_context_selects` (`src/query.rs:446,459,475`), and
  `counter_evidence` / `raw_refs` (`src/query.rs:372,403`) map to
  `requirement_unsatisfied`.
- **Schema/runtime agreement.** The advertised `minimum: 1` budget constraints
  match `validate_budget` (`src/query.rs:572-603`), and `additionalProperties:false`
  mirrors `deny_unknown_fields`, so the external contract and the serde runtime
  guard do not diverge on the cases I checked.
- **MCP shape.** `ToolDefinition` serializes `inputSchema`/`outputSchema`
  (`rename_all = "camelCase"`, `src/mcp.rs:16`), matching the MCP `2025-11-25` tool
  shape. Scope is held: one tool, `{ "bundle": EvidenceBundle }` only, stdio and
  companion outputs explicitly deferred.

### Findings (all minor, non-blocking — address in the next slice)

- **F1 (low — coupling) — the context/requirement split relies on a hand-synced
  string set.** `is_context_requirement` hardcodes the three `hot_context_*`
  strings (`src/mcp.rs:238-243`) with no compile-time link to the producers in
  `query.rs`. If a fourth `hot_context_*` requirement is later added in `query.rs`,
  it will silently fall through to `requirement_unsatisfied` instead of
  `context_unavailable`. Consider a shared `const`/set or a dedicated error variant
  so the classification cannot drift. Also, of the three context strings only
  `hot_context_entities` is exercised through `call_get_evidence_bundle`
  (`tests/mcp_agent_surface.rs:167-176`); `hot_context_time_window` and
  `hot_context_time_window_entities` map correctly by inspection but are untested at
  the handler boundary.

- **F2 (low — public contract surface) — `requirement` leaks internal selector
  names for `context_unavailable`.** For context failures the public `ToolError.requirement`
  carries `hot_context_entities` etc. (`src/mcp.rs:138`). The design's Error Model
  only blessed `counter_evidence` and `raw_refs` as `requirement` values; the
  `hot_context_*` names read like internal selector diagnostics. Either document
  these as part of the stable public contract or omit `requirement` for
  `context_unavailable` and rely on `code` + `message`.

- **F3 (nit — test coverage) — `budget_unsatisfied` has no handler-level test.**
  Five of seven `ToolErrorCode`s are exercised through `call_get_evidence_bundle`;
  `budget_unsatisfied` is trivially triggerable (e.g. a `max_tokens` below the
  compiled `tokens_used`) and would round out handler coverage. `source_ref_unresolved`
  and `internal_error` are harder to trigger deterministically and are acceptable
  to leave for later.

- **F4 (nit — duplication) — `default_require_raw_refs` is defined twice**, in
  `src/mcp.rs:234-236` and `src/query.rs:339`. Two copies of the same default can
  silently diverge; consider exporting one and reusing it.

### Answers To This Round's Review Focus

1. Yes — the distinct input type preserves `EvidenceQuery` semantics via an
   exhaustive `From` impl while making `scenario_id` required at the boundary.
2. Yes — object root, root `required`, draft-07 `$schema`, and array `items` are all
   present and tested; strict enough for this slice.
3. Yes — `jsonschema` (draft-07 mode) as a dev-dependency is an acceptable
   representative strict validator; correctly under `[dev-dependencies]`.
4. Yes — `call_get_evidence_bundle(Value) -> Result<_, ToolError>` plus the
   `ToolDefinition`/schema accessors are a clean seam for a stdio server to wrap;
   F1/F2 are the only shape refinements I'd want before the contract is frozen.
5. Yes — mapping uses stable `snake_case` codes with no Rust debug strings; see F2
   for the one contract-surface refinement.
6. Yes — scope is still minimal: one tool, bundle-only output, no stdio server, no
   companion outputs.

### Process / Framework Check

- Baseline SHA `7fcfcbe` points to the pushed covered-code commit (`feat: add mcp
  get evidence bundle boundary`), not to the review-3 commit `bdc7fc1`; correct and
  frozen. Covered code pushed first, review doc as its own commit after.
- Header carries milestone / critical-path / progress / deferred; `## Verification`
  lists commands with results, which I reproduced. Compliant.
- Correctly advances to implementation rather than re-running a design round, since
  the design gate closed in round 2.
- Process note (not a defect): rounds 0-2 carried only Claude's reviewer sections,
  so for gate purposes Claude was the active reviewer of record and coding starting
  after my round-2 agreement is consistent. If the User intends to record a separate
  Direction Verdict, that remains open; I am not blocking on it.
- This round leaves minor actionable findings and the milestone is incomplete
  (stdio server + smoke test remain), so under "Round Termination" the loop
  continues into the next implementation slice.
