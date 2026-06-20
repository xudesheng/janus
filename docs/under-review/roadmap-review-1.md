# Roadmap Review 1

- Baseline SHA: `cdb0eaa5a2726a06b3ba1d8c21898e3f4e5c64dd`
- Current milestone: finalize `docs/core/roadmap.md` as the formal roadmap for sequencing Janus implementation work.
- Critical path: yes - this round resolves the review-0 roadmap refinements before the project moves to `evidence-ir-schema`.
- Milestone progress: incorporated all six review-0 findings into `docs/core/roadmap.md`, tightening primitive coverage, milestone boundaries, and eval methodology.
- Deferred milestone work: none for the roadmap milestone. Implementation remains deferred to later topic-specific milestones.

This round responds to `docs/under-review/roadmap-review-0.md`.

## Response To Review 0

F1, `rank_suspected_causes` was orphaned:

- Added `rank_suspected_causes` as a Milestone 6 Evidence Compiler output.
- Added an acceptance criterion that `suspected_causes` fixture artifacts have a
  concrete generation path.
- Added `rank_suspected_causes` to the Milestone 7 agent surface candidate list.

F2, Milestone 5B over-promised causal classification:

- Changed Milestone 5B timeline work to emit candidate nearby-change markers
  rather than final causal or non-causal classifications.
- Moved causal and non-causal classification for nearby changes into Milestone 6,
  where time alignment, dependency direction, blast radius, and counter-evidence
  are available.

F3, M8 raw-access baseline needed a fairness guard:

- Updated Milestone 8 to require realistic raw query access under the same token
  budget using recency, label, and entity slices.
- Added an acceptance criterion that the raw baseline must be reviewed as an
  adversarial baseline, not a strawman designed to make Janus win.

F4, M8 scoring source and token measurement were underspecified:

- Added scoring against `scenario.json` `ground_truth`, including
  `primary_cause_entity`, `blast_radius`, and `not_the_cause`.
- Clarified that token cost must be measured from serialized raw-access and
  Janus-access material, not copied from hand-authored Evidence IR `token_cost`
  fields.

F5, `explain_symptom` was unmapped:

- Added a Milestone 7 boundary stating that `explain_symptom` is treated as a
  question-driven `get_evidence_bundle` workflow plus timeline context for this
  version.
- Left promotion to a standalone primitive conditional on the evidence contract
  needing a distinct surface later.

F6, the M1 narrow fixture loader's addressing was unclear:

- Clarified that Milestone 1 includes single-fixture resolution by scenario id or
  path, such as `fixtures/scenarios/<id>/expected.json`.
- Clarified that full source-ref validation, registry coverage, and fixture
  metadata validation remain Milestone 3 work.

## Reviewer Focus

Please check whether the roadmap now fully resolves F1-F6 without creating new
milestone drift.

Key points to review:

1. Is `rank_suspected_causes` now placed correctly in M6 and M7?
2. Is the M5B/M6 boundary around timelines and causal classification now clean?
3. Is the M8 raw-access baseline strong enough to make the eventual eval
   credible?
4. Are scoring and token-cost measurement now non-circular?
5. Is the `explain_symptom` mapping acceptable as a V1 boundary?
6. Is the M1 narrow loader scoped tightly enough while still supporting M2?

## Verification

No code verification this round. This is a documentation-only roadmap revision.

Commands run:

- `Get-Content -Raw -Encoding utf8 docs/under-review/roadmap-review-0.md`
- `Get-Content -Raw -Encoding utf8 docs/core/roadmap.md`
- `rg -n "rank_suspected_causes|suspected_causes|candidate nearby|non-causal classification|adversarial|ground_truth|token_cost|explain_symptom|single-fixture|real compiled" docs/core/roadmap.md`
- `Select-String -Path docs/core/roadmap.md -Pattern '[^\\x00-\\x7F]'`
- `git diff -- docs/core/roadmap.md`
- `git status --short --branch`

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

