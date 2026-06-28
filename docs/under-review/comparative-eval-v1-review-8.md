# Comparative Eval V1 Review 8

- Baseline SHA: `9cf69a9313a4f3d03bb36f158d712f83c17d5b25`
- Current milestone: Milestone 8 Comparative Eval V1, a repeatable local harness that compares raw telemetry access with Janus Evidence IR access over the fixture corpus under the same budget and reports wins and regressions honestly.
- Critical path: yes - this round implements slice 6 documentation/examples, the only remaining work called out by review-7.
- Milestone progress: added a stable process guide for running the comparative eval, reading text and JSON reports, interpreting required/report-only metrics, using `--fail-on-regression`, and understanding the expected raw-win allowlist; linked that guide from the core design.
- Deferred milestone work: none. All six implementation slices are now complete; remaining work is reviewer confirmation and any user-requested archive or merge handling.

## Response To Review 7

Review-7's `Direction Verdict` was `CONTINUE - slice 5 is complete, correct, and honest; the milestone success bar is now empirically met. Proceed to slice 6 (docs), which is the only remaining work.`

I kept this round to documentation, as requested:

1. Added [`docs/process/comparative-eval-v1.md`](../process/comparative-eval-v1.md), a usage guide for the current harness.
2. Documented the main command, selector flags, budget overrides, output path, text summary fields, JSON report sections, metric roles, and regression-gate behavior.
3. Made the no-single-score reading explicit: `required_avg` is a compact required-metric summary over the current synthetic fixture corpus, not the whole Janus product claim.
4. Documented the expected raw-win allowlist and where expected, unexpected, and stale allowlist entries appear in `summary.regression_gates`.
5. Addressed review-7's specific `missing-data-gap` ask: the doc says the missing-data subgroup ties on `missing_data_awareness` and that the raw win is driven by auditability/token-efficiency deltas, not by Janus failing to surface uncertainty.
6. Linked the new process guide from the CLI section in [`docs/core/comparative-eval-v1.md`](../core/comparative-eval-v1.md).

No scorer, adapter, or gate behavior changed in this round.

## Implementation Summary

This round adds the operational documentation needed to use the completed V1 harness without relying on review history.

The new guide covers:

- full-corpus and filtered eval commands;
- generated report location and git-ignore policy;
- how to read the text summary;
- required metrics versus the report-only timeline metric;
- JSON sections reviewers and CI should inspect;
- exact `--fail-on-regression` failure conditions;
- current expected raw wins: `traffic-shift-hotspot` and `missing-data-gap`;
- the `missing-data-gap` interpretation from review-7;
- a change checklist for future fixture, scorer, adapter, or gate edits.

The current all-fixture gate still passes:

- aggregate required average: Janus `0.877`, raw `0.828`, delta `0.049`;
- regression gates: `passed=true`, expected raw wins `2`, unexpected raw wins `0`;
- scenario winners: Janus won 10, raw won 2 (`traffic-shift-hotspot`, `missing-data-gap`).

## Review Focus

Please focus on these questions:

1. Does the new process guide complete slice 6 without overstating what the aggregate score means?
2. Is the expected raw-win allowlist documented transparently enough for V1?
3. Is the `missing-data-gap` explanation clear enough to avoid misreading that fixture as an uncertainty-handling failure?
4. Are the command examples and JSON/text report-reading notes sufficient for contributors and CI users?
5. If there are no findings, should this topic stop per Round Termination because all milestone work is complete?

## Verification

Commands run successfully on baseline `9cf69a9313a4f3d03bb36f158d712f83c17d5b25`:

- `cargo fmt --check`
- `cargo test comparative_eval`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `cargo run --bin validate_fixtures`
- `cargo run --bin compare_evidence_access -- --all --fail-on-regression`
- `git check-ignore -v target/eval/comparative-eval-v1.json`
- `git diff --check`

Observed results:

- `cargo test comparative_eval` passed 18 comparative eval tests.
- `cargo test` passed all 38 library tests plus current integration and doc tests.
- `cargo clippy --all-targets --all-features` passed cleanly.
- `cargo run --bin validate_fixtures` reported 12 fixtures, 2 false-causality traps, 0 errors, and 0 warnings.
- `cargo run --bin compare_evidence_access -- --all --fail-on-regression` exited `0` and recorded `repo_sha: 9cf69a9313a4f3d03bb36f158d712f83c17d5b25`.
- Aggregate required average: Janus `0.877`, raw `0.828`, delta `0.049`.
- Regression gates: `passed=true`, expected raw wins `2`, unexpected raw wins `0`.
- Scenario winners: Janus won 10, raw won 2 (`traffic-shift-hotspot`, `missing-data-gap`).
- `git check-ignore` confirmed `target/eval/comparative-eval-v1.json` is ignored through `target/`.
- `git diff --check` passed.

The covered formal-doc work was committed and pushed first as `9cf69a9 Document comparative eval usage`; this review document's baseline SHA points to that pre-review-document tree and must remain frozen.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

**STOP — the topic is complete. Slice 6 finishes the last remaining work cleanly, every
Definition-of-Done item is satisfied, and there are no findings. Per Round Termination, do NOT
submit review-9; report completion and await the User's next instruction (e.g. archive or merge).**

This is the terminal implementation round. Review-7 left no actionable defects — only the
docs-only slice 6 remained — and this round delivers exactly that, with no scorer/adapter/gate
behavior change. The milestone success bar was already met empirically in review-7 (Janus improves
4/5 required metrics with none regressing, both traps clean, regressions visible); slice 6 makes the
completed harness usable without relying on review history. The milestone is done.

Next action: **stop** (positive completion).

### Milestone Progress (judged first)

- **Docs-only, as scoped.** Commit `9cf69a9` touches only `docs/process/comparative-eval-v1.md`
  (new) and a 3-line link from `docs/core/comparative-eval-v1.md`. No code changed since `c731177`,
  which I reproduced green in review-7. I re-ran the gate here: `--all --fail-on-regression` exits
  `0`, 18 `comparative_eval` tests pass.
- **The guide is accurate and complete.** It documents the command, selectors, budget overrides,
  output path/git-ignore, the text fields, the JSON sections (including both subgroup summaries and
  `regression_gates`), metric roles, and the exact four `--fail-on-regression` fail conditions —
  all matching the code I verified across review-5/7.
- **It does not overstate (review-7's core ask).** "Do not read `required_avg` as the whole Janus
  product claim … a compact summary over the current synthetic fixture corpus" is explicit, and the
  guide directs readers to the metric deltas, trap/missing-data subgroups, raw winners, and
  auditability behavior. The `missing-data-gap` interpretation (ties on `missing_data_awareness`;
  the raw win is auditability/token-efficiency, not an uncertainty failure) is documented correctly.
- **Allowlist governance is sound.** The guide lists where expected/unexpected/stale allowlist
  entries surface and states "do not expand the expected raw-win allowlist as a convenience" — good
  guardrail against the failure mode I worried about in review-7.
- **Framework-compliant formal doc.** The new process guide is self-contained: it links only to the
  formal core design doc, never into `docs/under-review/`, and does not treat any review document as
  a source of truth. The core→process link is a permitted formal-to-formal reference.

### Answers to the round's Review Focus

1. Yes — slice 6 is complete and does not overstate the aggregate; the no-single-score framing is
   explicit and correct.
2. Yes — the allowlist is documented transparently, with the report fields that expose it and an
   anti-creep instruction.
3. Yes — the `missing-data-gap` explanation clearly separates the (tied) uncertainty metric from the
   auditability/token-efficiency drivers, so it will not be misread.
4. Yes — the command examples, text-field guide, JSON-section guide, and change checklist are
   sufficient for contributors and CI users.
5. **Yes — stop.** All milestone work is complete and this review raises no defects or new
   requirements, so Round Termination applies: no review-9.

### Completion Confirmation (Definition of Done)

Every DoD item in the design is satisfied: a repeatable local command over all/selected fixtures;
both paths under the same budget; raw baseline documented, deterministic, and non-strawman; Janus
access via compiled Evidence IR (not gold); ground truth/expected used only as scoring oracles;
per-scenario and aggregate scores; false-causality traps reported separately; measured token cost
from serialized material for both paths; auditability and missing-data awareness scored; honest
wins-and-regressions reporting; no LLM judge / ingest / persistence / dashboard / RCA prose; and
`cargo fmt`, `cargo test`, `cargo clippy --all-targets --all-features`, and `validate_fixtures` all
pass. The roadmap Milestone 8 acceptance is met: Janus improves multiple required metrics without
hiding regressions, and trap fixtures are not masked behind the aggregate.

### Summary

`comparative-eval-v1` is complete. The harness fairly compares raw vs. Janus Evidence IR access
under one budget, scores five required metrics plus a report-only timeline metric, gates regressions
at subgroup/aggregate level with a transparent allowlist, and now ships a self-contained usage
guide. Across eight rounds the one substantive risk — the review-5 counter-evidence materialization
— was caught and cleanly reverted, leaving a defensible baseline. No further review rounds should be
opened. Archiving or merging this topic is a User decision; I have not archived anything.
