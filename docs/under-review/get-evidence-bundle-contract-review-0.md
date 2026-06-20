# Get Evidence Bundle Contract Review 0

- Baseline SHA: `530c496decea93b7d577903de8c45aee17e12097`
- Current milestone: reviewer-approved Milestone 2 `get_evidence_bundle`
  walking-skeleton design in `docs/core/get-evidence-bundle-contract.md`
- Critical path: yes - implementation is explicitly blocked until all active
  reviewers agree on the design direction.
- Milestone progress: this round submits the Milestone 2 contract design for
  direction review and adds an explicit design gate plus implementation phasing
  guidance to the formal design doc.
- Deferred milestone work: Rust implementation, request schema generation,
  fixture-backed stub behavior, tests, and the emit path are deferred because the
  design has not yet received reviewer Direction Verdict approval.

This is the first review round for `get-evidence-bundle-contract`. There are no
prior review findings to answer.

The covered formal document is `docs/core/get-evidence-bundle-contract.md`. The
round is intentionally design-only. No Rust implementation should start until
all active reviewers agree on the design direction in their Direction Verdicts.

## Reviewer Focus

Please start with a Direction Verdict that states whether this design should
continue to implementation, redirect into another design round, or stop.

Focus review on these points:

1. Whether Milestone 2 is the right current milestone: a fixture-backed
   `get_evidence_bundle(EvidenceQuery) -> Result<EvidenceBundle, _>` walking
   skeleton, not real retrieval or ranking.
2. Whether the `EvidenceQuery` request shape is sufficient: intent,
   time window, budget, temporary `scenario_id`, optional entities,
   counter-evidence requirement, raw-ref requirement, freshness, and privacy
   scope.
3. Whether `scenario_id` is contained tightly enough as a Milestone 2
   fixture-only adapter, with a clear future path away from production query
   surfaces.
4. Whether the stub behavior is honest: it loads gold fixture bundles unchanged,
   validates contracts, rejects unsafe scenario ids, and returns a clear error
   for budgets smaller than the fixture bundle instead of pretending to optimize.
5. Whether the budget fields are correctly treated as semantic constraints
   rather than `LIMIT N`, even though budget-aware selection is deferred to
   Milestone 6.
6. Whether `require_counter_evidence`, `require_raw_refs`, source-reference
   preservation, and missing-data behavior are strong enough to protect the
   false-causality boundary established in `what_and_why.md`.
7. Whether request schema strictness is appropriate for this milestone:
   generated from Rust types, `snake_case`, unknown fields rejected, arrays with
   `items`, and known schema limitations covered by Rust validation helpers.
8. Whether the CLI-or-helper requirement is strict enough to prove executable
   JSON flow without overbuilding a user-facing command surface.
9. Whether implementation should proceed as one post-approval walking skeleton,
   or whether reviewers want phase-by-phase approval for the phases listed in
   the new Design Review Gate section.

## Implementation Boundary Proposed For Approval

If reviewers approve the direction, the next round may implement Milestone 2 as
one small walking skeleton. If any reviewer finds a design flaw in the request
contract, fixture adapter, stub error model, or schema boundary, the next round
should remain design-only or diagnosis-only.

This topic must not pull in real retrieval, ranking, storage engines, entity
resolution, anomaly detection, log clustering, registry-wide fixture validation,
live OpenTelemetry ingest, or MCP tool schemas.

## Verification

No code verification this round. This was a design-only review submission.

Checked locally:

- read `docs/review-framework.md` and `AGENTS.md` workflow guidance;
- read `docs/core/what_and_why.md`;
- read `docs/core/get-evidence-bundle-contract.md`;
- read referenced context in `docs/core/roadmap.md`,
  `docs/core/evidence-ir-schema.md`, `docs/core/evidence-spine.md`, and
  `docs/process/fixtures.md`;
- confirmed the worktree branch is `get-evidence-bundle-contract` tracking
  `origin/get-evidence-bundle-contract`;
- confirmed the baseline SHA is pushed before creating this review document.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

On the critical path: yes. A fixture-backed
`get_evidence_bundle(EvidenceQuery) -> Result<EvidenceBundle, _>` is the correct
next milestone per `roadmap.md` Milestone 2, and it builds directly on the
already-landed Milestone 1 Evidence IR types and narrow loader. The milestone is
the right size: it pins the request/response contract without dragging in
retrieval, ranking, storage, or MCP schemas, and the Out-of-scope list matches
the roadmap boundary ("real compiled and ranked path lands in Milestone 6").

I **agree with the overall design direction** — the milestone, the request shape,
the honest-stub framing, the `scenario_id`-as-temporary-adapter stance, and the
budget-as-semantics (not `LIMIT N`) principle are all correct and well aligned
with `what_and_why.md`, `roadmap.md`, and `fixtures.md`.

However, I am **not yet clearing implementation to start.** Two contract points
(M1, M2 below) are genuinely undefined in the doc and they sit exactly on the
request/response contract this milestone exists to freeze. Per this topic's own
Implementation Boundary ("if any reviewer finds a design flaw in the request
contract, stub error model, or schema boundary, the next round should remain
design-only or diagnosis-only"), these must be pinned down in
`docs/core/get-evidence-bundle-contract.md` first.

**Next action: continue, conditional.** Resolve M1 and M2 in a design revision
(this can be a short diagnosis-only round that updates the doc); fold in S1–S3 if
the implementor agrees. Once the doc reflects those, this design is approved and
coding may begin as one walking skeleton. I am not requesting a fundamental
redirect — the shape is right; the contract just needs two holes closed.

### Must-fix before coding

**M1 — "Budget smaller than the fixture bundle" is undefined.** Stub Behavior,
the Error Model (`unsupported budget for fixture stub`), and Test #5 all hinge on
comparing the request budget to "the fixture bundle budget," but the doc never
says which numbers are compared. The candidates give different answers:

- request `max_tokens` vs bundle `budget.max_tokens` (declared ceiling), or
- request `max_tokens` vs bundle `budget.tokens_used` (what the gold bundle
  actually consumes), and likewise
- request `max_items` vs bundle `budget.max_items` vs `items.len()`.

This is concrete with the chosen test fixtures: `deploy-bad-rollout` has
`max_items: 6, max_tokens: 600, tokens_used: 250` over 5 items;
`coincidental-deploy-trap` has `max_items: 7, max_tokens: 800, tokens_used: 380`
over 5 items. A request of `max_tokens: 400` passes against `tokens_used` but
fails against the declared `max_tokens` — opposite verdicts. The honest semantic
for a stub that returns the gold bundle unchanged is: error when the gold bundle
would not fit, i.e. compare request `max_tokens` against bundle
`budget.tokens_used` and request `max_items` against `items.len()` (not against
the bundle's own declared ceilings). Whatever is chosen, the doc must state the
exact comparison so Test #5 is unambiguous.

**M2 — "Return the loaded bundle unchanged, except for optional metadata that is
explicitly part of the response contract" contradicts the frozen M1 bundle.**
`EvidenceBundle` shipped in Milestone 1 with `#[serde(deny_unknown_fields)]` and
no field for echoing query intent/budget (`src/evidence.rs`). There is no
"optional metadata" slot today, so this clause is either a no-op or it silently
implies modifying the Milestone 1 `EvidenceBundle` type **and**
`schemas/evidence-ir/evidence-bundle.schema.json` — which would reopen a frozen
contract and is out of scope for this topic. Resolve by either: (a) stating the
bundle is returned byte-for-byte unchanged in M2 with no added metadata (my
recommendation — keeps M1 frozen), or (b) explicitly naming the metadata field,
acknowledging it changes the M1 type + schema, and confirming that is in scope.
As written it is an open door.

### Should-fix

**S1 — `scenario_id` is both *required* and *temporary*, which fights Contract
Principle 1.** The doc's headline principle is that the stable request contract is
what later retrieval "must honor," yet `scenario_id` is a required field that the
doc also says later milestones "should remove or isolate." Removing a required
field in M6 is a breaking schema change to the very contract this milestone is
meant to stabilize. Recommend either making `scenario_id` optional (so its later
removal is non-breaking) or explicitly documenting it as an accepted, known
breaking-change point and stating that the *rest* of `EvidenceQuery` is the stable
surface. Either is fine; the tension should not be left implicit.

**S2 — Request `freshness` (`any | settled | changing`) must be a distinct type
from the response `EvidenceFreshness` (`settled | changing`).** The added `any`
default is the right call for a request preference, but it means there are two
freshness vocabularies. The doc should name the request enum separately (e.g.
`FreshnessPreference`) so `any` does not leak into the response IR enum. Worth one
sentence to prevent an implementor reusing `EvidenceFreshness`.

**S3 — Pin where `scenario_id` safety is enforced and which error variant it
maps to.** The existing loader already rejects path separators and traversal
(`validate_scenario_id` in `src/fixtures.rs`, returning
`FixtureLoadError::InvalidScenarioId`), so Test #4 is partly satisfied today. The
new Error Model lists `invalid query` and `fixture load error` as separate
variants. State whether `scenario_id` validation runs in query validation
(→ `invalid query`) before the load, or is delegated to the loader
(→ `fixture load error`). This keeps the error contract callers see consistent.

### What is well handled (no change needed)

- Scope/out-of-scope and the Definition of Done match `roadmap.md` Milestone 2,
  including `cargo clippy --all-targets --all-features` in DoD #9.
- `scenario_id` containment language (Principle 6 + the table note) is clear about
  it not being the production query mechanism.
- Counter-evidence and raw-ref preservation are real safety properties here: both
  test fixtures already carry `weakens`/`contradicts` counter-evidence items
  (`deploy-bad-rollout` ev-5; `coincidental-deploy-trap` ev-3/ev-4), and
  `source_refs` is already a required non-empty field in M1, so the
  `require_raw_refs` guard is genuine contract pressure even if no current fixture
  can trip it. Honest, and acceptable.
- Schema strictness (generated from Rust, `snake_case`, `deny_unknown_fields` on
  request structs, arrays declare `items`, "question-or-hypothesis" left to a
  Rust validation helper) is consistent with the Milestone 1 schema approach and
  the framework's array-`items` check.

### Phasing answer (Reviewer Focus #9)

One post-approval walking skeleton is fine; I do not need phase-by-phase approval
for the three phases in the Design Review Gate. They are small and sequential, and
splitting them would add review overhead without reducing risk. Land them
together once M1/M2 are resolved.
