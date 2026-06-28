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
