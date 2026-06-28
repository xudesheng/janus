# Comparative Eval V1 Review 7

- Baseline SHA: `c731177ccbdc8ac9a92565cd7dd1500d4aa0b268`
- Current milestone: Milestone 8 Comparative Eval V1, a repeatable local harness that compares raw telemetry access with Janus Evidence IR access over the fixture corpus under the same budget and reports wins and regressions honestly.
- Critical path: yes - this round implements slice 5 regression grouping and `--fail-on-regression`, which is required before the harness can be treated as a repeatable local gate.
- Milestone progress: added missing-data subgroup reporting, regression-gate summaries, an expected raw-win allowlist, `--fail-on-regression` CLI behavior, formal design policy for the gate, and tests for both expected and unexpected raw wins.
- Deferred milestone work: slice 6 remains incomplete: documentation/examples explaining how to run the command and read the report without treating a single score as the whole product claim. Slice 6 was not done here because the regression policy needed review before documenting it as stable usage.

## Response To Review 6

Review-6's `Direction Verdict` was `CONTINUE - F-MAT and F-CAND are fully resolved and the score baseline is now clean and defensible; proceed to slice 5.`

I addressed the actionable feedback as follows:

1. Regression grouping: added `summary.missing_data` alongside the existing `summary.false_causality_traps`. Both subgroup summaries include average metric deltas, required-score delta, fixture count, Janus wins, raw wins, and ties.
2. Regression gates: added `summary.regression_gates`, including aggregate required delta, false-causality trap delta/raw wins, missing-data delta/raw wins, aggregate required-metric raw wins, observed raw winners, expected raw winners, unexpected raw winners, the expected allowlist, and failed gate names.
3. `--fail-on-regression`: implemented the CLI flag. It writes and prints the report first, then exits non-zero only if the gate fails.
4. Gate policy: implemented the review-6 recommendation at subgroup/aggregate level, not per scenario. The gate fails on negative aggregate required-score delta, trap subgroup regression or trap raw wins, aggregate required-metric raw wins, or non-allowlisted raw winners.
5. Expected regressions: documented and implemented the initial allowlist as `traffic-shift-hotspot` and `missing-data-gap`. They stay visible in the report and text output as raw wins; they do not silently disappear.
6. Missing-data diagnosis: the `missing-data-gap` raw win is now explicitly visible as a missing-data subgroup regression: required delta `-0.035`, raw wins `1`, driven by auditability (`-0.156`) and token efficiency (`-0.022`) while missing-data awareness itself ties at `0.0` delta.

## Implementation Summary

This round adds the first actionable regression policy without changing the scorer formulas.

The report now has two new summary maps:

- `missing_data`: subgroup score deltas and win counts for missing-data / under-determined scenarios.
- `regression_gates`: pass/fail policy details used by `--fail-on-regression`.

The current V1 gate passes when:

- aggregate required-score delta is non-negative within tolerance;
- false-causality trap fixtures have non-negative subgroup delta and no raw wins;
- no required metric regresses in aggregate;
- all raw-winning scenarios are on the explicit expected-regression allowlist.

The gate does not require Janus to win every fixture. Current expected raw wins remain visible:

- `traffic-shift-hotspot`
- `missing-data-gap`

The text report now includes:

```text
regression_gates: passed=true, expected_raw_wins=2, unexpected_raw_wins=0
```

## Review Focus

Please focus on these questions:

1. Is the `--fail-on-regression` policy aligned with review-6: aggregate/subgroup gate, not per-scenario gate?
2. Is hard-coding the initial expected raw-win allowlist in the eval harness acceptable for V1, given that the list is documented and emitted in `summary.regression_gates`?
3. Should `missing-data-gap` remain on the expected-regression allowlist despite being thematically important, or should slice 6 block until it is investigated more deeply?
4. Are the `regression_gates` JSON fields sufficient for CI and review diagnosis, or should the gate emit more per-scenario detail before the topic completes?
5. Should slice 6 proceed to docs/examples if this policy is accepted?

## Verification

Commands run successfully on baseline `c731177ccbdc8ac9a92565cd7dd1500d4aa0b268`:

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
- `cargo run --bin compare_evidence_access -- --all --fail-on-regression` exited `0` and recorded `repo_sha: c731177ccbdc8ac9a92565cd7dd1500d4aa0b268`.
- Aggregate required average: Janus `0.877`, raw `0.828`, delta `0.049`.
- Regression gates: `passed=true`, expected raw wins `2`, unexpected raw wins `0`.
- Scenario winners: Janus won 10, raw won 2 (`traffic-shift-hotspot`, `missing-data-gap`).
- Trap subgroup: required delta `0.109`, raw wins `0`.
- Missing-data subgroup: required delta `-0.035`, raw wins `1`.
- `git check-ignore` confirmed `target/eval/comparative-eval-v1.json` is ignored through `target/`.

The covered implementation was committed and pushed first as `c731177 Add comparative eval regression gates`; this review document's baseline SHA points to that pre-review-document tree and must remain frozen.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

**CONTINUE — slice 5 is complete, correct, and honest; the milestone success bar is now
empirically met. Proceed to slice 6 (docs), which is the only remaining work.** Implementation
round, so milestone progress before local defects. The regression policy follows the review-6
guidance exactly: it gates at subgroup/aggregate level (never per scenario), enforces on *new*
regressions, and keeps the two known raw wins visible through a transparent, report-emitted
allowlist. No blocking defects. Importantly, this round is where the central Janus bet measurably
pays off across the corpus.

Next action: **continue** to slice 6 (documentation/examples) — the last slice. After it lands with
no open findings, the topic should terminate and report completion, not spin further rounds.

### Milestone Progress (judged first) — the bet is now demonstrated

I verified the gate logic in code and reproduced the numbers on baseline `c731177`
(`cargo fmt --check`, `cargo clippy --all-targets --all-features` clean; 18 `comparative_eval` tests
+ full suite pass; `--all --fail-on-regression` exits `0`):

- **Janus improves 4 of 5 required metrics in aggregate and regresses none** (Janus−raw):
  auditability `+0.053`, false-causality risk `+0.112`, suspicious-entity accuracy `+0.042`,
  token efficiency `+0.038`, missing-data awareness `0.0` (tie). `raw_required_metric_wins = 0`.
  Report-only `timeline_quality` is `-0.015` and correctly excluded from the gate.
- **Nothing is masked.** Trap subgroup required delta `+0.109` with 0 raw wins; auditability is
  positive in aggregate; the missing-data subgroup regression (`-0.035`, 1 raw win) and the
  timeline dip are surfaced, not hidden. This satisfies the design's completion bar — "Janus
  improving at least one roadmap target metric without hiding regressions, and without masking
  false-causality-trap or auditability failures behind an aggregate score" — and then some.
- **The gate actually enforces.** `regression_gate_fails_for_unexpected_raw_winner` flips one
  scenario to a raw win and asserts `passed = false` with the scenario named and the failure
  message containing `unexpected_raw_winners`. So the green result is a real pass, not a
  happy-path tautology. The four fail conditions (aggregate delta, trap regression/raw-wins,
  per-required-metric aggregate regression, unexpected raw winners) match my review-6 recommendation.
- **O-ALIAS follow-through.** `resource_aliases_cover_expected_resource_entity_namespace` now
  spot-checks the alias mapping against each fixture's scored ground-truth entities — the review-4
  ask is closed.

### Answers to the round's Review Focus

1. **Yes — aligned with review-6.** Subgroup/aggregate gate, not per-scenario; the gate explicitly
   does not require Janus to win every fixture, and the unexpected-raw-win test confirms it.
2. **Acceptable for V1.** The `EXPECTED_RAW_WIN_SCENARIOS` constant is documented, emitted in
   `summary.regression_gates`, and the gate fails on any non-allowlisted regression, so it hides
   nothing. One low-severity follow-up (not blocking): make the allowlist configurable via a CLI
   flag or config file in a later iteration, so changing the gate *policy* is not a source edit —
   that better fits a "repeatable local gate." Fine as a constant for V1.
3. **Keep `missing-data-gap` on the allowlist.** Your diagnosis resolves the review-6 concern: the
   subgroup `missing_data_awareness` delta is `0.0` (a tie), so Janus is *not* worse at the
   thematically important uncertainty handling — the raw win is driven by auditability and token
   efficiency on that one fixture. Allowlisting it is correct; just make sure slice-6 docs state
   *why* (auditability/token gap, not an uncertainty failure) so a reader does not misread it as
   Janus failing at missing-data.
4. **Sufficient for V1.** `regression_gates` carries aggregate + per-subgroup deltas, raw metric
   wins, observed/expected/unexpected winners, `expected_raw_winners_not_observed`, and
   `failed_gates`. Per-scenario required deltas already live under `scenarios[].comparison`, so CI
   and review have what they need. No more is required before completion.
5. **Yes — proceed to slice 6.**

### Observations (non-blocking)

- **Allowlist rot:** `expected_raw_winners_not_observed` is emitted but does not warn or fail when
  an allowlisted fixture stops being a raw win (i.e. Janus improved and the entry is now stale).
  Emitting it is enough for V1; a future stricter mode could surface it more loudly so the allowlist
  is pruned. No action needed now.
- **Topic is near done:** every Definition-of-Done item is satisfied today; slice 6 is documentation
  only. The design does not list docs as a DoD criterion, but it is listed as slice 6, so finishing
  it cleanly closes the topic. Keep slice 6 to docs/examples — do not reopen scorer or gate behavior
  there.

### Summary

Slice 5 lands a well-designed, honest regression gate: subgroup/aggregate conditions, a transparent
emitted allowlist, real enforcement on new regressions (test-proven), and the review-4 O-ALIAS and
review-6 missing-data diagnoses both closed. The full run now demonstrates the milestone bet — Janus
improves 4/5 required metrics with none regressing, both traps clean, and all regressions visible.
Continue to slice 6 (docs); explain the allowlist rationale (especially missing-data-gap) there.
This round leaves no blocking defects, so review-8 should carry slice 6 and then, if no findings
remain, the topic should be reported complete per Round Termination rather than continuing to spin.
