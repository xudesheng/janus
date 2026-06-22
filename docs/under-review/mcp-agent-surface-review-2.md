# MCP Agent Surface Review 2

- Baseline SHA: `8a943656595b2b19c0cc021a22cc78768613c155`
- Current milestone: Milestone 7 Agent Surface V1, a reviewed local agent-facing `get_evidence_bundle` tool surface with strict MCP-facing schemas and a deterministic fixture-backed smoke path.
- Critical path: yes - this round resolves the remaining design-gate blocker before any implementation can start.
- Milestone progress: updated `docs/core/mcp-agent-surface.md` to reconcile the duplicated schema-validator guidance from review 1 and to record the phase-1 `scenario_id` schema-generation guard.
- Deferred milestone work: all Rust implementation, schema artifacts, runtime handlers, and smoke tests remain deferred until every active reviewer agrees on the design direction in their `Direction Verdict`.

Round 1 received one reviewer verdict from Claude: continue, with the gate ready to close after one formal-doc contradiction was reconciled and active reviewers agreed. It left actionable feedback B1 and B2, plus nits. This round is design-only and submits the formal design cleanup for review.

No code was implemented in this round.

## Response To Review 1

B1, duplicated schema-validation guidance: addressed.

The older weaker guidance that said to verify schemas against "the chosen local validator" has been merged into the stricter review-1 language. The formal design now has a single contract: V1 MCP schemas explicitly declare draft-07, are validated with the Rust `jsonschema` crate in draft-07 mode or an equivalent justified-representative strict validator, and migrate to 2020-12 only when a real MCP client, tool-use runtime, or strict validator requires it. The design explicitly says not to add a weaker local-validator-only acceptance path.

B2, `EvidenceQuery` reuse trap: addressed.

The design now says the tool input schema can reuse `EvidenceQuery` semantics, but must not be produced by blindly exporting Rust `EvidenceQuery`, because internal `EvidenceQuery.scenario_id` is optional. The implementation must use a distinct V1 tool-input type, a deliberate schema-generation override, or an explicit post-generation transform. Tests must assert that the V1 tool input schema root `required` list includes `scenario_id`.

Nits: addressed.

- The stale "may keep" wording around `scenario_id` now says the first implementation uses the fixture/demo selector and labels it as temporary.
- The error mapping now says all server-side faults collapse to `internal_error` unless the selector itself is invalid or unknown, including fixture corpus loading and invalid fixture bundles.

## Review Focus

Please focus this round on whether the design gate can close:

1. Does the updated Schema Strategy remove the B1 contradiction and leave only one strict validator contract?
2. Is the B2 guard enough to prevent phase 1 from accidentally generating an MCP input schema where `scenario_id` is optional?
3. Are the stale `scenario_id` wording and server-side error-collapse nits resolved?
4. If you agree, please record whether implementation may begin phase-by-phase after this round, starting with MCP schema artifacts and the transport-independent handler.

## Verification

No code verification this round. This is a design-only review submission.

Repository checks performed:

- `git diff --check` passed before committing the formal design update.
- `git status --short --branch` showed a clean worktree tracking `origin/mcp-agent-surface` before this review document was created.
- The covered formal design-doc change was committed and pushed first as `8a94365 docs: reconcile mcp agent surface schema guidance`.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
