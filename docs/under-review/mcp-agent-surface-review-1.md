# MCP Agent Surface Review 1

- Baseline SHA: `e7b5657705237048fcf316a9dc165e3601d6b307`
- Current milestone: Milestone 7 Agent Surface V1, a reviewed local agent-facing `get_evidence_bundle` tool surface with strict MCP-facing schemas and a deterministic fixture-backed smoke path.
- Critical path: yes - this round resolves design-gate feedback that must close before any implementation can start.
- Milestone progress: updated `docs/core/mcp-agent-surface.md` to fold in review-0 findings F1-F4: stdio MCP as completion target, representative schema validation, honest V1 `scenario_id` requirements, and total tool-error mapping for evidence requirements.
- Deferred milestone work: all Rust implementation, schema artifacts, runtime handlers, and smoke tests remain deferred until every active reviewer agrees on the design direction in their `Direction Verdict`.

Round 0 received one reviewer verdict from Claude: continue, phase-by-phase, with clarifications required before the design gate closes. The review also noted that other active reviewers, notably the User, still need to record agreement. Because round 0 left actionable findings F1-F4, this round is design-only and submits the formal design update for review.

No code was implemented in this round.

## Response To Review 0

F1, runtime completion bar: addressed.

`docs/core/mcp-agent-surface.md` now states that stdio MCP is the proposed completion target. A protocol-shaped command remains acceptable only as an intermediate implementation slice for deterministic schema and handler tests. The Definition of Done now requires a stdio MCP server that handles a real `initialize`, `tools/list`, and `tools/call` exchange and returns structured Evidence IR JSON.

F2, schema acceptance must not be self-certifying: addressed.

The design now requires V1 MCP-facing schemas to explicitly declare draft-07 with `$schema`, matching current `schemars` output. It also requires validation with the Rust `jsonschema` crate in draft-07 mode, or an equivalent strict JSON Schema validator justified as representative of expected MCP/tool-use clients. The design records a concrete migration trigger for JSON Schema 2020-12: an MCP client, tool-use runtime, or strict validator rejects Janus's explicit draft-07 tool schemas or needs 2020-12-only features. Committed `schemas/mcp/` artifacts must be diffed against freshly generated output in tests or CI.

F3, `scenario_id` schema honesty: addressed.

The design now requires `scenario_id` in the V1 MCP tool input schema because the current fixture-backed runtime cannot answer without it. The field must be labeled as a temporary fixture/demo selector. The formal design reserves `context_selector` as the production replacement concept once Janus has live or persisted context selection.

F4, raw-ref error-model gap: addressed.

The design now uses `requirement_unsatisfied` for evidence requirements that cannot be met, with a stable `requirement` field such as `counter_evidence` or `raw_refs`. It also explicitly says server-side fixture replay, hot-store, derivation, evidence compiler, missing-fixture-bundle, and fixture-bundle-parse failures collapse to `internal_error` unless the request selector itself is invalid or points to an unknown fixture.

Additional tightening:

- The design now says MCP tool-call results should return the output envelope as structured content, with the same serialized JSON available as text content where needed for client compatibility.
- It states that future `suspected_causes` and `next_checks` exposure should be a surfacing and schema decision, not a second compiler path.

## Review Focus

Please focus this round on whether the design gate can close:

1. Do the formal design updates fully address F1-F4 from review 0?
2. Is stdio MCP as the completion target, with command-only as an intermediate slice, the right runtime bar?
3. Is required-in-V1 `scenario_id`, with reserved replacement concept `context_selector`, the right schema honesty trade-off?
4. Is `requirement_unsatisfied` with a `requirement` field the right public tool-error shape for counter-evidence and raw-ref failures?
5. Is explicit draft-07 plus a representative strict validator sufficient for V1, with 2020-12 migration deferred until a real client/runtime requires it?
6. If you agree with the updated design, please say whether implementation may begin phase-by-phase after this round, starting with schema artifacts and the transport-independent handler.

## Verification

No code verification this round. This is a design-only review submission.

Repository checks performed:

- `git diff --check` passed before committing the formal design update.
- `git status --short --branch` showed a clean worktree tracking `origin/mcp-agent-surface` before this review document was created.
- The covered formal design-doc change was committed and pushed first as `e7b5657 docs: clarify mcp agent surface design`.
- I checked the current official MCP tools and transports documentation while updating the design. The referenced spec version is `2025-11-25`; it lists stdio as a standard transport, permits explicit draft-07 tool schemas via `$schema`, and defines structured tool output with `structuredContent`.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
