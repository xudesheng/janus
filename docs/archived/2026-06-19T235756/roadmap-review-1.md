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

## Review (by Claude)

### Direction Verdict

**Stop the roadmap review loop — milestone complete; proceed to `evidence-ir-schema`.**
This round was on the critical path and it finishes the roadmap milestone: all six
review-0 findings are resolved, verified against the round-1 baseline tree
(`cdb0eaa`) by diffing it against the round-0 baseline (`01aba42`). No material new
drift was introduced, and the changes did not loosen the evidence-substrate
boundary. Per Round Termination, round 0's actionable feedback has been addressed
and the milestone (finalize `docs/core/roadmap.md`) is complete with no new
defects, so a `review-2` is **not** warranted. Treat the roadmap as finalized and
open the first implementation topic.

### Verification of F1–F6 (against the diff, not just the response text)

- **F1 `rank_suspected_causes` — resolved.** Added as an M6 deliverable (links
  candidate causes to supporting/counter-evidence and trap notes), an M6
  acceptance criterion (`suspected_causes` fixtures have a concrete generation
  path), and to the M7 surface list. No longer orphaned.
- **F2 M5B/M6 boundary — resolved cleanly.** M5B now emits "candidate
  nearby-change markers" and its acceptance explicitly drops final
  causal/non-causal classification; that classification moved to M6 (with an
  acceptance criterion that nearby changes are classified "only after Evidence
  Compiler reasoning, not by the Milestone 5B timeline builder alone"). The
  split now matches where the false-causality reasoning actually lives.
- **F3 raw-access baseline fairness — resolved (the important one).** M8 now
  requires "realistic raw query access … using recency, label, and entity slices
  rather than a naive full dump or arbitrary `LIMIT N`", plus an acceptance
  criterion that the baseline is "reviewed as an adversarial baseline, not a
  strawman designed to make Janus win." This is what makes the eventual eval
  credible.
- **F4 scoring source / token measurement — resolved.** M8 scores against
  `scenario.json` `ground_truth` (`primary_cause_entity`, `blast_radius`,
  `not_the_cause`), and token cost is "measured from serialized … material, not
  copied from hand-authored Evidence IR `token_cost` fields." The circularity is
  closed.
- **F5 `explain_symptom` — resolved.** M7 boundary maps it to a question-driven
  `get_evidence_bundle` workflow plus timeline context, with promotion to a
  standalone primitive left conditional. Acceptable V1 boundary.
- **F6 M1 loader addressing — resolved.** Single-fixture resolution by scenario
  id/path is now explicitly in M1 scope; source-ref/registry/metadata validation
  stays in M3. M2's "by scenario" selection no longer implicitly pulls M3 forward.
- **Bonus:** an M2 boundary was added ("returns hand-authored gold bundles; the
  real compiled and ranked path lands in Milestone 6"), which closes the
  stub-vs-real `get_evidence_bundle` ambiguity I noted under Q2 last round. Good.

### Optional, non-blocking notes (do NOT open a round for these)

- **Timeline artifact ownership across M5B/M6.** With classification moved to M6,
  the fixtures' fully-labeled `timeline` (e.g. the `non-causal-change` marker in
  `coincidental-deploy-trap`) is only reproducible after M6; M5B produces the
  unlabeled/candidate timeline. The acceptance criteria already imply this; a
  single sentence saying the labeled timeline artifact is an M6 output would make
  it explicit. Cosmetic.
- **Eval metric set vs `what_and_why.md`.** M8's metrics are a sound
  machine-scorable subset, but `what_and_why.md`'s evaluation list also names
  `time-to-useful-hypothesis` and `human-rated evidence quality`. Worth one line
  in M8 marking those as deliberately deferred (not silently dropped), to keep
  roadmap↔design alignment honest. Not required for V1.

Both are refinements the Implementor can fold in directly if desired; neither is a
defect and neither justifies another review round.

### Process note

Commit-and-review gate is compliant for round 1: the round-0 reviewer section was
committed (`169f46f`), the covered roadmap refinements landed as their own commit
(`cdb0eaa`) before the review-1 request (`6909360`), the review-1 baseline points
to that pre-review-document tree, and the working `roadmap.md` matches it (empty
diff against `cdb0eaa`).

### Next action

Stop. The roadmap is finalized; begin implementation at `evidence-ir-schema`
(Milestone 1), which should open its own `evidence-ir-schema-review-0.md`.

