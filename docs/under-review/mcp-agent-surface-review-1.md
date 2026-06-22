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

## Review (by Claude)

### Direction Verdict

Continue. I **agree with the design direction** and, from this one reviewer's
standpoint, the gate is ready to close and implementation may begin phase-by-phase
(schema artifacts -> transport-independent handler -> stdio surface -> smoke test)
**once one formal-doc contradiction is reconciled** (F2 residue, below) and the
other active reviewers (notably the User) also record agreement. The gate is not
mine alone to close.

Round 1 is correctly scoped as design-only and the four V1 decisions are now the
right ones: single `get_evidence_bundle` tool; stdio MCP as the completion bar
with command-only explicitly insufficient; `scenario_id` required in V1 and
`context_selector` reserved; `requirement_unsatisfied{requirement}` for evidence
requirements; draft-07 + a representative strict validator with a named 2020-12
migration trigger; `{ "bundle" }`-only output. The two extra tightenings
(structured `structuredContent` with text fallback; "surfacing, not a second
compiler path" for the optional slice) are accurate against the MCP `2025-11-25`
spec and the existing compiler, and improve the design.

### F1–F4 Assessment

- **F1 (runtime completion bar) — fully addressed.** Runtime Surface (lines
  306-309, 343-347) and DoD (lines 470-473) now make stdio MCP the completion
  target and state command-only is not sufficient by itself. This matches the
  roadmap Milestone 7 acceptance ("an external agent can call Janus"). Good.
- **F2 (schema acceptance not self-certifying) — substantively addressed, but
  with a residual contradiction (see B1).** The new paragraph (lines 265-276)
  plus schema-requirement (lines 289-290) and test (lines 438-439) correctly
  require an explicit draft-07 declaration, validation via the `jsonschema` crate
  (or a justified-representative validator), a committed-vs-generated diff, and a
  concrete 2020-12 trigger. That is exactly what F2 asked for.
- **F3 (`scenario_id` honesty) — addressed in the contract, with an
  implementation trap to avoid (see B2).** First Tool (lines 172-178), required
  field list (line 182), schema requirement (lines 285-286) and DoD (lines
  475-477) now make `scenario_id` required in V1 and reserve `context_selector`.
  Good.
- **F4 (raw-ref error gap) — fully addressed.** `requirement_unsatisfied` with a
  stable `requirement` field (lines 361-363, 372) plus the explicit
  collapse-to-`internal_error` rule (lines 376-380) and the new test (line 444)
  make the mapping from `GetEvidenceBundleError` total. I walked the 12 existing
  variants in `src/query.rs:76-108` against the categories; every one now has a
  home.

### Findings

- **B1 (blocker for a clean gate) — reconcile the duplicated schema-validation
  guidance.** The strict new F2 paragraph (lines 265-276) was added, but the old
  weaker paragraph (lines 293-300) was left in place and still says "verify the
  emitted schemas are accepted by **the chosen local validator**" — the precise
  self-certifying phrasing F2 was raised to remove. The formal doc is the contract
  and now contradicts itself: one place demands a justified-representative
  validator, another permits any local validator. Remove or merge lines 293-300
  into the new paragraph so a single, strict statement stands. This is a small
  edit to a formal doc (which the Implementor may still edit), not a direction
  change; once done my agreement is unconditional.

- **B2 (medium, forward-looking into phase 1) — "reuse `EvidenceQuery`" will emit
  `scenario_id` as optional unless deliberately overridden.** The design says the
  tool input schema "can reuse `EvidenceQuery`" (lines 262-263) while also
  requiring `scenario_id` (lines 285-286). But `EvidenceQuery.scenario_id` is
  `Option<String>` (`src/query.rs:28-29`), and the committed generated schema lists
  only `budget`, `intent`, `time_window` as `required`
  (`schemas/evidence-ir/evidence-query.schema.json:5-9`). So generating the
  `schemas/mcp/` input artifact straight from `EvidenceQuery` via `schemars` would
  re-introduce exactly the optionality F3 closed. Phase 1 must use a deliberate
  mechanism — a distinct tool-input type, a `schemars` attribute, or a documented
  post-generation override — and the committed-vs-generated diff test (lines
  289-290) must assert `scenario_id` is in the input schema's `required`. Worth a
  one-line note in the design so the implementor does not naively generate-and-
  commit. Not a gate blocker.

### Nits (non-blocking)

- Lines 167-170 still read "may keep the existing fixture/demo `scenario_id`
  selector" with optional-flavored framing, immediately above the now-decided
  "required in V1" paragraph (172-178). Not contradictory, but the older wording
  reads slightly stale; consider aligning it when touching B1.
- The `internal_error` collapse list (lines 376-380) names six server-side
  failures but not `FixtureCorpusLoad` or `InvalidFixtureBundle`
  (`src/query.rs:78,90`). Both are clearly covered by the catch-all definition
  ("unexpected store, replay, derivation, or compiler failure"), so the mapping is
  still total; mentioning them or making the rule explicitly "all server-side
  faults except invalid/unknown selector" would remove any ambiguity.

### Answers To This Round's Review Focus

1. F1, F3, F4 fully addressed; F2 addressed but for the B1 residue. 2. stdio-target
/ command-intermediate is the right bar — yes. 3. required-in-V1 `scenario_id` with
reserved `context_selector` — yes (mind B2 at implementation). 4.
`requirement_unsatisfied{requirement}` is the right shape — yes. 5. explicit
draft-07 + representative validator with a deferred 2020-12 trigger is sufficient
for V1 — yes. 6. From my standpoint implementation may begin phase-by-phase after
B1 is reconciled and all active reviewers agree.

### Process / Framework Check

- Baseline SHA `e7b5657` points to the pushed formal-doc commit (`docs: clarify
  mcp agent surface design`), the pre-review-document tree, not to the review-1
  commit `c524130`; frozen and correct.
- Covered formal-doc change pushed first, review-1 as its own commit after.
  Compliant.
- Header carries milestone / critical-path / progress / deferred; `## Verification`
  records "no code verification this round" for a design-only round. Compliant.
- Round 1's review leaves actionable feedback (B1, plus B2/nits), so under "Round
  Termination" the loop continues. B1 is a formal-doc reconciliation the
  Implementor can make directly; the next round can be a short design-only round
  confirming the cleanup (and recording final reviewer agreement) before any code
  lands.
