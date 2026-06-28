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

## Review (by Claude)

### Direction Verdict

Continue — and I now **agree with the design unconditionally**. Both review-1
blockers (B1, B2) and both nits are resolved in the formal doc; I have no remaining
design findings. From this reviewer's standpoint the design gate is closed and
implementation may begin **phase-by-phase** (MCP schema artifacts -> transport-
independent handler -> stdio surface -> smoke test). The full gate still requires
every other active reviewer (notably the User) to record agreement; I can only
close my own.

This is a convergence round: it left no design defect open and the milestone's
*implementation* artifact does not yet exist, so under "Round Termination" the next
round should be the **first implementation round (phase 1)**, not another
design-only round and not a stop.

### Verification Of The Round-1 Fixes

I checked each against `docs/core/mcp-agent-surface.md` at the baseline tree, not
just the response summary:

- **B1 (duplicated schema-validation guidance) — resolved.** The formerly
  contradictory paragraph (lines 298-302) no longer permits "the chosen local
  validator"; it now defers to the strict contract above it and explicitly says
  "do not add a weaker local-validator-only acceptance path." Exactly one strict
  validator contract now stands (lines 270-281). Good.
- **B2 (`EvidenceQuery` reuse trap) — resolved.** Lines 262-268 now forbid
  generating the V1 input schema by blindly exporting `EvidenceQuery` (whose
  `scenario_id` is `Option<String>`, `src/query.rs:28`), require a distinct
  tool-input type / generation override / post-generation transform, and require a
  test asserting root `required` includes `scenario_id`. That closes the path back
  to the optionality F3 removed.
- **Nit 1 (stale `scenario_id` wording) — resolved.** Lines 167-170 now state the
  first implementation *uses* the fixture/demo selector and labels it temporary,
  consistent with the required-in-V1 paragraph below it.
- **Nit 2 (server-side error collapse) — resolved.** Lines 378-383 now state all
  server-side faults collapse to `internal_error` unless the selector is invalid
  or unknown, explicitly including fixture corpus loading and invalid fixture
  bundles. The mapping from all 12 `GetEvidenceBundleError` variants
  (`src/query.rs:76-108`) is now total with no implicit cases.

### Answers To This Round's Review Focus

1. Yes — the Schema Strategy now has a single strict validator contract; the B1
   contradiction is gone. 2. Yes — the B2 guard (distinct type/override/transform +
   a `required`-includes-`scenario_id` test) is enough to stop phase 1 from emitting
   an optional `scenario_id`. 3. Yes — both nits are resolved. 4. Yes — from my
   standpoint implementation may begin phase-by-phase after this round, starting
   with the MCP schema artifacts and the transport-independent handler, once all
   active reviewers have agreed.

### Guidance For The First Implementation Round

Not blockers — just what I will look for when reviewing phase 1, drawn straight
from the design's own Tests and DoD so there are no surprises:

- the committed `schemas/mcp/get-evidence-bundle.input.schema.json` lists
  `scenario_id` in root `required` (the B2 assertion), declares draft-07 via
  `$schema`, has an object root, and every array declares `items`;
- the committed-vs-freshly-generated diff test exists so the agent contract cannot
  drift from the Rust types;
- the strict validator chosen for the acceptance test is named and justified as
  representative (the `jsonschema` crate in draft-07 mode satisfies this);
- the handler maps every `GetEvidenceBundleError` variant to a stable tool error
  code with no `internal_error` leakage of Rust debug strings, including the
  `requirement_unsatisfied{requirement}` cases for both `counter_evidence` and
  `raw_refs`.

### Process / Framework Check

- Baseline SHA `8a94365` points to the pushed formal-doc commit (`docs: reconcile
  mcp agent surface schema guidance`), the pre-review-document tree, not to the
  review-2 commit `7723ec5`; frozen and correct.
- Covered formal-doc change pushed first, review-2 as its own commit after.
  Compliant.
- Header carries milestone / critical-path / progress / deferred; `## Verification`
  records "no code verification this round" for a design-only round. Compliant.
- This review leaves no actionable design feedback, but the milestone's
  implementation artifact is unbuilt, so the loop continues into implementation
  rather than stopping.
