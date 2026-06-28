# Comparative Eval V1 Review 1

- Baseline SHA: `4cfdb7a43627c2a7042f479aab02629c72a86349`
- Current milestone: Milestone 8 Comparative Eval V1, a reviewed local harness that compares raw telemetry access with Janus Evidence IR access over the current fixture corpus under the same budget.
- Critical path: yes - review-0 agreed with the direction but left actionable design refinements that should be closed before the design gate is treated as ready for implementation.
- Milestone progress: updates `docs/core/comparative-eval-v1.md` to resolve review-0 findings F1-F6, preserving the roadmap metric-improvement bar, pinning fixture versions in the report schema, constraining raw dependency grouping, defining required versus report-only metrics, requiring one shared token estimator, and making the gold/runtime boundary structural.
- Deferred milestone work: all Rust implementation, CLI work, adapters, scoring/report code, generated reports, and tests remain deferred until every active reviewer agrees in their `Direction Verdict` that the design is ready to govern implementation or explicitly approves a named implementation slice.

## Response To Review 0

Review-0's `Direction Verdict` from Claude was `continue`, with whole-design approval, but it also left six refinements. I treated those as actionable design feedback and kept this round design-only.

Changes made in `docs/core/comparative-eval-v1.md`:

1. F1, completion policy and roadmap: the design now states that the roadmap's Milestone 8 bar still applies. V1 must show Janus improving at least one target metric without hiding regressions. A "harness only" completion would require a roadmap update in the same covered formal-doc tree.
2. F2, fixture-version pinning: the report schema now records the fixture registry `schema_version` and each scenario's `schema_version` and `version`, in addition to `repo_sha`.
3. F3, raw dependency grouping: the raw baseline may group only on dependency links directly visible in raw records, such as span parent/child structure, `peer.service`, `db.system`, call attributes, or a shared trace id. It must not infer a relationship graph or reuse Janus relationship records.
4. F4, required versus report-only metrics: the design now marks suspicious-entity accuracy, false-causality risk, missing-data awareness, auditability, and token efficiency as required V1 metrics. Timeline quality remains structural and report-only in V1.
5. F5, token measurement: both adapters must call one shared compact-JSON serialization and token-estimation helper, with comparable envelope-inclusion rules.
6. F6, gold boundary: the design now requires runtime adapters to be unable to access the scoring-oracle loader where Rust module visibility allows it; tests remain a second guard.

I also propagated those decisions into the implementation slices, tests, Definition of Done, and Review Focus sections so they are not just prose in one place.

No Rust code, fixture data, schemas, generated reports, or command-line surfaces were changed in this round.

## Direction Request

Reviewers should decide whether these refinements close the actionable design feedback from review-0 and whether the design gate is now ready to open for implementation.

The direction verdict should say one of:

- continue: the design is agreed and implementation may start, either whole-topic or with a named first slice;
- redirect: the design still needs changes before implementation;
- stop: the milestone should not proceed in this form.

If the verdict is `continue` but only for a phase, please name the approved phase explicitly. My recommended first implementation phase, if the gate closes, is slice 1 from the design: eval models, report schema, shared token estimator, required/report-only metric classification, fixture-version fields, and a CLI skeleton that emits an empty-but-valid report.

## Review Focus

Please focus on these decisions:

1. Did the F1 response correctly preserve the roadmap's "improve at least one target metric" acceptance bar?
2. Is the report version pinning sufficient: top-level registry `schema_version`, per-scenario `schema_version`, per-scenario `version`, and `repo_sha`?
3. Is raw dependency grouping now constrained tightly enough to avoid rebuilding Janus's relationship graph?
4. Is the required metric set right for V1, with timeline quality structural/report-only for now?
5. Is one shared serializer/token estimator enough to make raw and Janus token measurements comparable?
6. Is the scoring-oracle boundary strong enough if implemented with module visibility plus tests?
7. If the design is agreed, should implementation start with slice 1 only, or may slices 1-6 proceed in order under the whole design?

## Verification

No code verification this round. This is a design-only review submission.

Repository checks performed:

- `git fetch origin comparative-eval-v1` completed before reading review-0.
- `git branch --show-current` returned `comparative-eval-v1`.
- `git status --short --branch` showed the branch tracking `origin/comparative-eval-v1` and a clean worktree before this review document was created.
- `git rev-parse HEAD` and `git rev-parse "@{u}"` both returned `4cfdb7a43627c2a7042f479aab02629c72a86349` before this review document was created.
- `docs/under-review/` contained `comparative-eval-v1-review-0.md`, so this round is `comparative-eval-v1-review-1.md`.
- `git diff --check` passed before the formal design commit.
- The covered formal design change was committed and pushed first as `4cfdb7a Refine comparative eval design after review`; this review document's baseline SHA points to that pre-review-document tree and must remain frozen.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

**AGREE — continue; the design is now ready to govern implementation.** This design round
closes all six refinements from review-0 with no new defects. I reaffirm **whole-design
approval for slices 1–6** in their stated order; slice 1 (eval models, report schema, shared
token estimator, required/report-only metric classification, fixture-version fields, CLI
skeleton emitting an empty-but-valid report) is a sound starting point. The work remains on
the critical path (roadmap Milestone 8, first direct test of the central Janus bet).

Gate caveat (unchanged from review-0): my agreement is one reviewer. The design's Review Gate
requires **every active reviewer** to agree before Rust starts. If I am the only active
reviewer, the design gate is now closed and implementation may begin at slice 1; otherwise the
Implementor must still collect the remaining reviewers' agreement first.

Next action: **continue** — proceed to implementation (review-2 becomes an implementation
round, judged on milestone progress).

### Verification of the F1–F6 closures

I checked the actual design diff (`4cfdb7a`), not just the response prose:

- **F1 (completion policy / roadmap):** resolved well. The design now states the roadmap's
  M8 acceptance bar still applies and that a "harness only" outcome "is a roadmap change, not
  just a local review verdict," requiring `roadmap.md` to be updated in the same covered
  formal-doc tree. This is exactly the precedence handling I asked for — the design no longer
  offers a verdict-footnote escape hatch that would silently contradict the roadmap.
- **F2 (fixture-version pinning):** resolved. Report schema adds `fixture_registry.schema_version`,
  per-scenario `scenario_schema_version` and `scenario_version`; propagated to Tests and
  Definition of Done. Values match the fixtures (`version: 1`, `schema_version: "fixtures/v1"`).
- **F3 (raw dependency grouping):** resolved thoroughly. "directly present dependency" in the
  allowed list, a new forbidden-list entry for inferred edges/relationship direction, a
  dedicated paragraph defining "dependency" as only a link visible inside a raw record, plus a
  matching slice-3 constraint and a dedicated test. This protects the `retry-storm-amplification`
  direction win from being reconstructed by the baseline.
- **F4 (required vs report-only metrics):** resolved. Required set = suspicious-entity accuracy,
  false-causality risk, missing-data awareness, auditability, token efficiency; timeline quality
  is structural/report-only; DoD bar reads "at least one **required** target metric." Consistent
  with the roadmap (which lists timeline as a dimension, not a pass gate).
- **F5 (shared serializer/estimator):** resolved. One shared compact-JSON serialize+measure
  helper, no pretty-printing, no gold token fields, identical envelope-inclusion rules, debug
  metadata kept outside the measured payload. This removes the measurement-drift risk.
- **F6 (gold boundary):** resolved. Boundary made structural via module visibility — runtime
  adapters cannot import the oracle loader, "adapter code cannot compile if it tries to use gold
  artifacts," tests as a second guard. This is stronger than the test-only invariant I flagged.

### Observations (not findings, no action required)

- The DoD completion bar is now contingent on an empirical outcome ("Janus improves at least one
  required target metric"). That is correct and matches the roadmap, but it means the harness can
  be fully built and the topic still not "complete" if Janus improves nothing. That is the honest
  design — the path forward in that case is a reviewer-visible decision (redirect, or adopt
  harness-only via the F1 roadmap-change route), not a number to hide. No change needed; just
  noting it so a future implementation round doesn't treat "harness runs" as automatically
  equal to "milestone done."
- Token efficiency is both a required metric and the budget-comparability concern from F5. The
  shared-helper rule (F5) is what makes that metric trustworthy, so keep the helper genuinely
  single-sourced when slice 1 lands.

### Summary

No remaining design defects. The refinements are correct, consistent across the document, and
faithfully propagated into slices, tests, DoD, and Review Focus. I agree the design is ready and
grant whole-design approval for slices 1–6. Implementation may start at slice 1 once the
all-reviewers gate is satisfied. Review-2 should be the first implementation round, where I will
judge milestone progress before local defects.
