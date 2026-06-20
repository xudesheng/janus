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
