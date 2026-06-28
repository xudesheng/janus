# MCP Agent Surface Design

Status: design for the `mcp-agent-surface` topic.

This document defines the Milestone 7 Agent Surface V1 slice. It is grounded in
[`what_and_why.md`](what_and_why.md), [`roadmap.md`](roadmap.md),
[`evidence-ir-schema.md`](evidence-ir-schema.md),
[`get-evidence-bundle-contract.md`](get-evidence-bundle-contract.md),
[`fixture-otel-simulator.md`](fixture-otel-simulator.md),
[`otel-ingest-prototype.md`](otel-ingest-prototype.md), and
[`evidence-compiler-ranking.md`](evidence-compiler-ranking.md). If this
document conflicts with `what_and_why.md`, the canonical design doc wins and
this document should be corrected.

External protocol references:

- Model Context Protocol specification: <https://modelcontextprotocol.io/specification>
- MCP tools specification: <https://modelcontextprotocol.io/specification/2025-11-25/server/tools>
- MCP architecture overview: <https://modelcontextprotocol.io/docs/learn/architecture>

MCP is an external integration protocol. Janus should comply with the parts it
uses, but Janus's normative contract remains the Evidence IR and investigation
primitive semantics defined in this repository.

## Why This Topic Is Next

The simulator and OTel JSON ingest topics are complete enough for local source
data to enter the hot-store boundary. The hot store, entity and relationship
context, derived context, and evidence compiler are also complete. Most
importantly, `get_evidence_bundle` now routes through fixture replay, derived
context, and compiled evidence rather than returning hand-authored gold bundles.

The next missing layer is not another ingestion preview. It is the first
agent-facing surface:

```text
agent tool call
      -> strict MCP-compatible input schema
      -> Janus investigation primitive
      -> strict structured output
      -> auditable Evidence IR JSON
```

Without this topic, Janus has a working internal evidence path but no reviewed
tool boundary an external agent can call. With it, the current simulator and
OTel JSON ingest work become demonstrable through an agent-oriented contract.

## Purpose

This topic should expose Janus investigation primitives to external agents in a
small, schema-first way.

The first surface should be a `get_evidence_bundle` tool backed by the existing
compiled evidence path. Optional additional tools may be added only if the
reviewed schema and runtime path remain small.

The tool should return structured evidence, not root-cause prose. Agents can
reason and communicate; Janus supplies bounded, source-backed, token-budgeted
evidence.

## Design Review Gate

No Rust implementation should start for this topic until every active reviewer
has agreed on the design direction in their `Direction Verdict`.

Reviewers should explicitly decide:

- whether `mcp-agent-surface` is the right next topic after
  `evidence-compiler-ranking`;
- whether the first milestone should be only one `get_evidence_bundle` tool, or
  may include a small read-only companion tool;
- whether the local implementation should start as a stdio MCP server, a
  protocol-shaped command for strict schema tests, or both;
- whether `scenario_id` may remain in the first tool schema as a fixture/demo
  selector, and what production replacement should be named;
- whether output should expose only `EvidenceBundle` first, or include
  `suspected_causes` and `next_checks` from the compiler result.

Current proposed V1 direction after review feedback is:

- expose only `get_evidence_bundle` in the first agent-facing surface;
- make stdio MCP the completion target;
- allow a protocol-shaped command only as an intermediate implementation slice;
- require `scenario_id` in the V1 tool input schema while labeling it as a
  temporary fixture/demo selector;
- reserve `context_selector` as the production replacement concept for
  selecting live or persisted Janus context;
- return only `{ "bundle": EvidenceBundle }` in V1;
- defer public `suspected_causes` and `next_checks` fields until reviewers
  approve the extra schema surface.

These choices are still subject to final reviewer agreement. The design gate is
closed only when every active reviewer agrees in their `Direction Verdict`.

## Scope

In scope:

- a stable agent-facing tool definition for `get_evidence_bundle`;
- strict input and output JSON Schemas suitable for external tool validators;
- schema tests that check array `items`, object roots, required fields, and no
  unknown top-level fields where Janus owns the schema;
- a small local MCP-compatible stdio server or a protocol-shaped command that
  can list and call the tool;
- tool-call runtime path backed by the existing compiled
  `get_evidence_bundle` implementation;
- structured error mapping from Janus errors to tool errors;
- a deterministic local smoke test that calls the tool and receives Evidence IR
  JSON;
- explicit demo relationship to fixture simulator and OTLP JSON ingest paths.

Out of scope:

- production HTTP transport, auth, TLS, tenant isolation, or remote deployment;
- full MCP resources or prompts;
- UI widgets, dashboard panels, or MCP apps;
- new OTLP protocols or Collector receiver work;
- persistence, warm/cold memory, or compaction;
- LLM-generated root-cause explanations;
- mitigation execution or agent action orchestration;
- broad privacy enforcement beyond preserving and surfacing privacy fields.

## External Protocol Boundary

The first implementation should treat MCP as a thin adapter over Janus
contracts.

MCP-relevant constraints for this topic:

- tools have names, descriptions, input schemas, and optionally output schemas;
- tool input schemas are JSON Schema objects;
- V1 pins the MCP protocol version to `2025-11-25` in the local stdio
  `initialize` response; cross-version negotiation is deferred until Janus
  targets a client that requires a different protocol version;
- schema compatibility is a separate acceptance surface from local Rust
  correctness;
- transport details should stay behind a small adapter so a future HTTP or
  hosted server can replace local stdio.

The design should avoid making external protocol details the source of truth
for Janus behavior. If an MCP rule changes later, the adapter can change while
`EvidenceQuery`, `EvidenceBundle`, and compiler semantics remain stable.

## First Tool

Tool name:

```text
get_evidence_bundle
```

Purpose:

Return a bounded, source-backed Evidence IR bundle for a question or hypothesis
under a token and item budget.

Runtime path:

```text
tool arguments
      -> EvidenceQuery
      -> query validation
      -> fixture replay or future live context selection
      -> derived context
      -> evidence compiler
      -> EvidenceBundle
      -> tool structured output
```

The first implementation uses the existing fixture/demo `scenario_id` selector
because the current compiled path still uses fixture replay to build a fresh
store. The schema must label it as a demo selector, not the long-term production
query mechanism.

For V1, `scenario_id` is required in the MCP tool input schema because the
current runtime cannot answer without fixture-backed context selection. Marking
it optional at the agent boundary would make the tool schema misleading. The
schema description should state that `scenario_id` is temporary and will be
replaced by a production context selector once Janus has a live or persisted
context-selection path. The reserved production concept name is
`context_selector`.

Minimum V1 required input fields:

- `scenario_id`;
- `intent.question` and/or `intent.hypothesis`;
- `time_window.start`;
- `time_window.end`;
- `budget.max_items`;
- `budget.max_tokens`.

Minimum V1 optional input fields:

- `entities`;
- `require_counter_evidence`;
- `require_raw_refs`;
- `freshness`;
- `privacy_scope`.

Minimum output:

```json
{
  "bundle": {
    "...": "EvidenceBundle"
  }
}
```

The output envelope allows future metadata without changing the Evidence IR
shape. The `bundle` field should be exactly the current `EvidenceBundle`
contract. In MCP tool-call results, the envelope should be returned as
structured content, with the same serialized JSON also available as text content
when needed for client compatibility.

## Optional Companion Outputs

The evidence compiler now also produces `suspected_causes` and `next_checks`,
but public `get_evidence_bundle` returns only the bundle today.

This topic may choose one of two reviewed options:

1. Keep V1 output as `{ "bundle": EvidenceBundle }` only. This is the smallest
   surface and the preferred first slice.
2. Add optional fields to the tool output envelope:

   ```json
   {
     "bundle": {},
     "suspected_causes": [],
     "next_checks": []
   }
   ```

If option 2 is chosen, the compiler result path must be exposed without
duplicating query logic or re-running compilation inconsistently. The output
schemas for `suspected_causes` and `next_checks` must be generated or tested
with the same strictness as Evidence IR.

Because the compiler already produces `suspected_causes` and `next_checks`, a
future optional slice should be a surfacing and schema decision, not a second
compiler path.

Do not expose separate `rank_suspected_causes` or `suggest_next_checks` tools
until reviewers approve that the schema and runtime paths are mature enough.

## Schema Strategy

Existing schemas:

- `schemas/evidence-ir/evidence-item.schema.json`;
- `schemas/evidence-ir/evidence-bundle.schema.json`;
- `schemas/evidence-ir/evidence-query.schema.json`.

This topic should add MCP-facing schema artifacts under a separate directory,
for example:

```text
schemas/mcp/
  get-evidence-bundle.input.schema.json
  get-evidence-bundle.output.schema.json
  tools-list.schema.json          # optional, if the implementation emits one
```

The tool input schema can reuse `EvidenceQuery` semantics but must be committed
as a tool-facing artifact so compatibility tests can validate it independently.
Because Rust `EvidenceQuery.scenario_id` is optional at the internal query
boundary, the MCP V1 input schema must not be generated by blindly exporting
`EvidenceQuery`. It should use a distinct V1 tool-input type, a deliberate
schema-generation override, or an explicit post-generation transform, and tests
must assert that root `required` includes `scenario_id`.

V1 MCP-facing schemas should explicitly declare draft-07 with `$schema`, matching
the repository's current `schemars` output. The implementation should not rely
only on Janus's Rust deserialization or ad hoc schema inspection. It should
validate committed MCP schemas with the Rust `jsonschema` crate in draft-07 mode
or an equivalent strict JSON Schema validator, and the review should justify the
chosen validator as representative of the MCP/tool-use clients Janus expects to
serve.

The concrete migration trigger for JSON Schema 2020-12 is an MCP client,
tool-use runtime, or strict validator that rejects Janus's explicit draft-07
tool schemas or requires 2020-12-only features. Until then, V1 should keep the
dialect narrow, explicit, and tested instead of migrating all project schemas.

Schema requirements:

- the tool input schema root is `type: object`;
- arrays declare `items`;
- required fields are explicit;
- enum values are explicit;
- integer budgets have positive minimums;
- `scenario_id` is required in the V1 MCP tool input schema and clearly
  described as a temporary fixture/demo selector;
- the output schema declares the `bundle` object and its nested arrays;
- schema generation is repeatable and tested;
- committed `schemas/mcp/` artifacts are diffed against freshly generated output
  in tests or CI;
- strict validators should not need to infer array item types.

The repository currently generates draft-07 schemas through `schemars`. MCP
documentation recommends JSON Schema 2020-12 in current versions, but this topic
should not migrate all project schemas just to expose one tool. The emitted
dialect, representative validator, and 2020-12 migration trigger are defined
above; do not add a weaker local-validator-only acceptance path.

## Runtime Surface

The first runtime should be small and local. Acceptable implementation shapes:

Review feedback makes stdio MCP the proposed completion target. A
protocol-shaped command is acceptable only as an intermediate implementation
slice for deterministic schema and handler tests. The topic is not complete with
a command-only surface unless reviewers explicitly change this bar.

### Option A: stdio MCP server

Add a binary such as:

```bash
cargo run --bin janus_mcp
```

Minimum behavior:

- handle initialize/list/call interactions required by the chosen local MCP
  smoke test;
- advertise `get_evidence_bundle`;
- validate incoming arguments against Janus request validation;
- call the existing Rust primitive;
- return structured JSON output.

### Option B: protocol-shaped command

Add a binary such as:

```bash
cargo run --bin janus_tool -- tools/list
cargo run --bin janus_tool -- tools/call get_evidence_bundle --input query.json
```

Minimum behavior:

- emit the same tool definition and schemas that a stdio MCP server would use;
- call the same Rust handler;
- support deterministic tests without a long-running process.

Option B is acceptable as a first slice only if the implementation records what
remains before a real MCP stdio server. It does not satisfy the topic Definition
Of Done by itself. The topic is complete only when an external agent can call
Janus through a stdio MCP-compatible local surface unless reviewers explicitly
change this completion bar in a later design round.

## Error Model

Tool errors should preserve Janus contract failures without leaking Rust debug
strings as the API.

Suggested error categories:

- `invalid_request`: JSON does not match the tool schema or query validation
  fails;
- `fixture_not_found`: current demo selector does not match a known fixture;
- `context_unavailable`: query time/entity selectors match no hot context;
- `budget_unsatisfied`: requested budget cannot fit required evidence;
- `requirement_unsatisfied`: a requested evidence requirement cannot be
  satisfied, with a stable `requirement` field such as `counter_evidence` or
  `raw_refs`;
- `source_ref_unresolved`: compiled evidence has an unresolved source ref;
- `internal_error`: unexpected store, replay, derivation, or compiler failure.

Each error should include:

- stable machine-readable code;
- human-readable message;
- optional path or field;
- optional requirement name when `code` is `requirement_unsatisfied`;
- no panic/debug backtrace;
- no root-cause prose.

All server-side faults should collapse to `internal_error` unless the request
selector itself is invalid or points to an unknown fixture. This includes fixture
corpus loading, invalid fixture bundles, fixture replay, hot-store failures,
derivation failures, evidence compiler failures, missing fixture bundles, and
fixture-bundle parse failures. That keeps tool errors actionable for agents
without leaking Janus internals as public API.

## Demo Path

This topic should make the current local demo concrete:

```bash
cargo run --bin janus_mcp
```

or, for the command-shaped slice:

```bash
cargo run --bin janus_tool -- tools/call get_evidence_bundle --input examples/queries/deploy-bad-rollout.json
```

The demo input should use an existing fixture scenario such as
`deploy-bad-rollout` or `coincidental-deploy-trap`. The output should be
structured JSON with source-backed Evidence IR.

Relationship to simulator and OTel ingest:

- fixture simulator remains the deterministic replay path used by the current
  compiled query;
- OTLP JSON ingest remains a local input adapter, not the MCP surface itself;
- a later live-ingest topic can replace V1's required `scenario_id` with
  `context_selector` over persisted or in-memory live data;
- this topic proves that once data is in Janus, an agent can call a stable tool
  and receive evidence.

## Suggested Implementation Slices After Design Approval

No slice should start until reviewers agree on the design direction or approve
that slice explicitly.

Recommended slices:

1. Tool schema model and artifacts: define tool input/output envelopes, generate
   or commit schema artifacts, and add strict schema compatibility tests.
2. Tool handler boundary: map JSON arguments to `EvidenceQuery`, call
   `get_evidence_bundle`, and map errors to stable tool errors without adding a
   transport yet.
3. Local invocation surface: implement a stdio MCP server or reviewed
   protocol-shaped command that can list and call `get_evidence_bundle`.
4. Smoke-test demo: add one committed query example and an integration test that
   calls the local surface and verifies valid structured Evidence IR JSON.
5. Optional companion output: expose `suspected_causes` and `next_checks` only
   if reviewers approve the extra surface and strict schemas are ready.

These are implementation slices, not separate milestones.

## Tests

Add tests that prove:

- the tool input schema root is an object;
- every array schema declares `items`;
- generated or committed MCP-facing schemas match the Rust types;
- committed MCP-facing schemas validate under the explicitly chosen strict
  draft-07 JSON Schema validator;
- the V1 tool input schema root `required` list includes `scenario_id`;
- `get_evidence_bundle` tool arguments deserialize into `EvidenceQuery`;
- invalid tool arguments produce a stable `invalid_request` error;
- invalid `scenario_id` produces a stable selector error;
- budget and counter-evidence failures map to stable tool errors;
- raw-ref requirement failures map to a stable tool error;
- a valid fixture-backed tool call returns an output envelope with a valid
  `EvidenceBundle`;
- source refs in returned evidence remain resolvable through the existing query
  path;
- a stdio MCP server can complete an `initialize`, `tools/list`, and
  `tools/call` smoke exchange for `get_evidence_bundle`;
- a protocol-shaped command, if implemented first, lists the same tool
  definition and calls the same handler as the stdio server.

Existing verification should continue to pass:

```bash
cargo fmt
cargo test
cargo clippy --all-targets --all-features
cargo run --bin validate_fixtures
```

## Definition Of Done

This topic is complete when:

- `get_evidence_bundle` is exposed through a reviewed agent-facing tool surface;
- tool input and output schemas are committed or generated repeatably;
- schema tests cover strict tool-validator concerns, including array `items`;
- a local stdio MCP server can handle a real `initialize`, `tools/list`, and
  `tools/call` exchange and return structured Evidence IR JSON;
- a protocol-shaped command alone is not sufficient for topic completion unless
  a later review explicitly changes the completion bar;
- tool errors use stable categories instead of raw Rust debug output;
- `scenario_id` is required in the V1 tool schema and clearly documented as a
  temporary fixture/demo selector, with `context_selector` reserved as its
  production replacement concept;
- no new OTel protocol, persistence layer, dashboard, warm memory, mitigation
  execution, or RCA prose generator is introduced;
- `cargo fmt`, `cargo test`, `cargo clippy --all-targets --all-features`, and
  `cargo run --bin validate_fixtures` pass.

## Review Focus

Reviewers should focus on:

1. Whether `mcp-agent-surface` is the right topic after
   `evidence-compiler-ranking`, given that simulator and OTel JSON ingest are
   already complete.
2. Whether the first surface should expose only `get_evidence_bundle`.
3. Whether stdio MCP as the completion target and command-only as an
   intermediate slice is the right runtime bar.
4. Whether the schema strategy is strict enough for external tool validators
   without forcing a broad schema-dialect migration.
5. Whether required-in-V1 `scenario_id` is acceptable as a temporary demo
   selector and whether `context_selector` is the right reserved replacement
   concept.
6. Whether the surface returns inspectable evidence and avoids becoming an RCA
   prose API.
