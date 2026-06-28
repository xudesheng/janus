# Comparative Eval V1 Usage

This document explains how to run and read the Comparative Eval V1 harness. The
design contract lives in [`docs/core/comparative-eval-v1.md`](../core/comparative-eval-v1.md);
this file is the day-to-day process guide.

Comparative Eval V1 compares two access paths over the same fixture scenario,
time window, item budget, and token budget:

- Janus access: compiled Evidence IR material.
- Raw access: a deterministic raw telemetry context pack.

The harness is an evidence-substrate comparison. It is not an RCA agent, an LLM
judge, a production benchmark, or a claim that one aggregate number proves
product quality.

## Quick Run

Run the whole fixture corpus and enforce the V1 regression gate:

```bash
cargo run --bin compare_evidence_access -- --all --fail-on-regression
```

The command prints a text summary and writes JSON to:

```text
target/eval/comparative-eval-v1.json
```

`target/` is ignored by git; generated eval reports should not be committed
unless a later review explicitly adds a stable snapshot.

To print JSON to stdout as well as writing the output file:

```bash
cargo run --bin compare_evidence_access -- --all --format json
```

To write JSON somewhere else:

```bash
cargo run --bin compare_evidence_access -- --all --output target/eval/my-run.json
```

## Selecting Fixtures

Use `--all` for the full corpus, or use selector flags. Do not combine `--all`
with selectors.

Examples:

```bash
cargo run --bin compare_evidence_access -- --fixture coincidental-deploy-trap
cargo run --bin compare_evidence_access -- --failure-class missing-data
cargo run --bin compare_evidence_access -- --difficulty hard
cargo run --bin compare_evidence_access -- --trap true
cargo run --bin compare_evidence_access -- --capability false-causality-guard
```

The default budget is:

```text
max_items = 6
max_tokens = 1200
```

Override it only when you are intentionally testing budget sensitivity:

```bash
cargo run --bin compare_evidence_access -- --all --max-items 8 --max-tokens 1600
```

## Reading The Text Summary

A full run currently has this shape:

```text
comparative eval v1
schema: comparative-eval/v1
repo_sha: <current commit>
fixtures: 12 (registry schema fixtures/v1)
budget: max_items=6, max_tokens=1200
metrics: 5 required, 1 report_only
required_avg: janus=0.877, raw=0.828, delta=0.049
regression_gates: passed=true, expected_raw_wins=2, unexpected_raw_wins=0
scenarios:
- deploy-bad-rollout v1 (..., winner=janus)
```

Read the fields as follows:

- `schema` is the report schema version.
- `repo_sha` is the source tree used for the run.
- `fixtures` records how many registered scenarios were selected and the
  fixture registry schema.
- `budget` is the shared budget applied to both access paths.
- `metrics` separates required metrics from report-only metrics.
- `required_avg` is the average of required metrics only.
- `regression_gates` is the CI/review-oriented pass/fail policy.
- Each scenario line shows measured tokens, required-average scores, and the
  per-scenario winner.

Do not read `required_avg` as the whole Janus product claim. It is a compact
summary over the current synthetic fixture corpus. The report must also be read
through the required metric deltas, false-causality trap subgroup, missing-data
subgroup, raw-winning scenarios, and source-reference/auditability behavior.

## Metrics

Required V1 metrics:

- suspicious-entity accuracy;
- false-causality risk;
- missing-data awareness;
- auditability;
- token efficiency.

Report-only V1 metric:

- timeline quality.

Timeline quality remains visible in the report, but it is excluded from
`required_avg` and from the V1 regression gate. A timeline regression is still a
useful diagnostic; it just does not block this milestone's required-metric gate.

## JSON Report

The JSON report has these top-level sections:

- `schema_version`
- `repo_sha`
- `fixture_registry`
- `budget`
- `metrics`
- `summary`
- `scenarios`

The most useful summary fields are:

- `summary.janus`, `summary.raw`, and `summary.delta`: aggregate averages and
  win counts.
- `summary.false_causality_traps`: subgroup deltas for trap fixtures, so false
  causality cannot be hidden by an aggregate score.
- `summary.missing_data`: subgroup deltas for missing-data / under-determined
  scenarios.
- `summary.regression_gates`: the pass/fail policy inputs and result.

Each `scenarios[]` entry records the scenario id, fixture schema/version,
failure class, difficulty, trap flag, both access-path submissions, scores, and
comparison deltas. Per-scenario raw wins are expected to remain visible here
even when the aggregate gate passes.

## Regression Gate

Without `--fail-on-regression`, the command exits non-zero for harness errors:
invalid arguments, fixture loading failures, schema/runtime failures, or report
build failures. A raw-winning scenario is reported but does not by itself fail
the command.

With `--fail-on-regression`, the command writes and prints the report first,
then exits non-zero if any V1 gate fails:

- aggregate required-score delta is negative beyond tolerance;
- false-causality trap fixtures regress as a subgroup or any trap fixture is a
  raw win;
- raw access wins any required metric in aggregate;
- a non-allowlisted fixture becomes a raw winner.

The initial expected raw-win allowlist is:

- `traffic-shift-hotspot`
- `missing-data-gap`

These are not hidden exemptions. They appear in:

- `summary.regression_gates.observed_raw_winners`
- `summary.regression_gates.expected_raw_winners`
- `summary.regression_gates.expected_raw_win_allowlist`

Any raw winner outside that list appears in
`summary.regression_gates.unexpected_raw_winners` and fails
`--fail-on-regression`.

`expected_raw_winners_not_observed` is emitted when a selected allowlisted
fixture no longer has a raw win. That is useful pressure to prune the allowlist
in a later review, but V1 does not fail on stale allowlist entries.

## Current Known Raw Wins

The current all-fixture run has two expected raw wins:

- `traffic-shift-hotspot`
- `missing-data-gap`

`missing-data-gap` needs careful interpretation. The missing-data subgroup
currently ties on `missing_data_awareness` itself (`0.0` delta), so the raw win
is not evidence that Janus is worse at surfacing uncertainty. The subgroup
regression is driven by auditability and token-efficiency deltas on that one
fixture. Keep that distinction visible when changing the scorer, baseline, or
allowlist.

## Change Checklist

When changing fixtures, scoring, access adapters, or regression policy:

```bash
cargo fmt --check
cargo test comparative_eval
cargo test
cargo clippy --all-targets --all-features
cargo run --bin validate_fixtures
cargo run --bin compare_evidence_access -- --all --fail-on-regression
```

Then inspect:

- aggregate required metric deltas;
- `summary.false_causality_traps`;
- `summary.missing_data`;
- `summary.regression_gates.unexpected_raw_winners`;
- `summary.regression_gates.expected_raw_winners_not_observed`;
- per-scenario `comparison` details for every raw winner.

Do not expand the expected raw-win allowlist as a convenience. A new expected
raw win should have a documented reason and should stay visible in the report.
