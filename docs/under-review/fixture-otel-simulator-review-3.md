# Fixture OTel Simulator Review 3

- Baseline SHA: `45f629aa8da2eecf7a3be70b21fc66bf6c8b73e8`
- Current milestone: a `simulate_fixture` CLI and ingest-like replay path that replays a registered fixture into a fresh `HotContextStore`, validates source refs after replay, and proves partial-replay ref availability with tests.
- Critical path: yes - this round implements the approved slice 2 ingest boundary that the simulator needs before default replay and source-ref validation can be wired into the CLI.
- Milestone progress: added `HotIngestEvent`, `HotContextStore::ingest`, metric-point accumulation into metric-series records, duplicate-key negative coverage, partial replay source-ref tests, raw full-replay source-ref tests, timestamp normalization for replay ordering, and review-2 design clarifications.
- Deferred milestone work: default full CLI replay, compact summary output, bundle source-ref validation after replay, derived artifact validation, and final CLI smoke coverage remain for slice 3 because this round first had to prove the store mutation boundary and partial replay semantics.

## Response To Review 2

Review-2 approved slice 2 and raised four non-blocking findings. This round addresses them as follows:

- F1, mixed timestamp lexical ordering: replay planning now computes a normalized UTC timestamp sort key for fixture `Z` timestamps with optional fractional seconds while preserving the original timestamp string on emitted events. The new corpus-wide parsed monotonicity test caught the existing `retry-storm-amplification` same-second mixed-format ordering bug, and the fix now orders `2026-06-05T14:45:01Z` before later fractional timestamps. Broader timestamp formats and `--until` remain out of scope.
- F2, empty-span trace timing: `docs/core/fixture-otel-simulator.md` now formalizes the trace-level `start` fallback for absent or empty span start data, and the planner honors `spans: []` plus trace-level `start`. A focused test covers this case.
- F3, missing timed signal fields: metric points, spans, logs, changes, and telemetry gaps now require their event timestamp fields during replay planning. Prior incidents remain optional and can still preload as warm-context memory.
- F4, conflicting render flags: `simulate_fixture --dry-run --jsonl` now errors instead of silently choosing JSONL.

## Implementation Summary

Slice 2 adds a small ingest boundary around `HotContextStore`:

- `HotIngestEvent` represents resources, traces, spans, metric points, logs, changes, prior incidents, and telemetry gaps.
- `MetricSeriesKey` identifies the merge target for metric points with the same source-key convention Evidence IR already uses.
- `HotContextStore::ingest` inserts non-metric records through the existing primary-key/index machinery.
- Metric-point ingest creates the metric series on the first point and appends later points only when the non-point metadata matches exactly; conflicting metric metadata returns a duplicate-primary-key error.
- `TryFrom<&SimulationEvent> for HotIngestEvent` connects the replay planner to the store boundary without exposing store internals to the simulator.
- Partial replay tests prove a change ref is missing before its event and found after it, and that a trace ref can resolve before a span ref from the same trace.
- Full replay tests currently assert raw source refs resolve through the simulated ingest path. Derived refs such as anomaly windows and log patterns are intentionally left for the slice-3 validation path.

The formal design doc was updated to describe normalized UTC timestamp ordering, required timestamps for timed signals, empty-span trace fallback, metric-point payload shape, mutually exclusive render modes, and the raw-vs-derived split across implementation slices.

## Review Focus

Please focus on slice-2 correctness and whether the next round can proceed to slice 3:

1. Is `HotIngestEvent` the right reusable boundary for the future OTLP receiver, or is the metric-point payload too fixture-shaped?
2. Are metric merge semantics correct: exact metadata equality excluding `points`, append-only point accumulation, and duplicate errors for conflicting metadata?
3. Does non-metric ingest preserve the existing duplicate-primary-key invariant strongly enough?
4. Are the partial replay tests proving the right semantics for source refs becoming available over simulated time?
5. Is the raw source-ref full replay scope acceptable for slice 2, with derived artifact validation deferred to slice 3?
6. May the next implementation round proceed to default full CLI replay, compact summary output, and bundle/source-ref validation after replay?

## Verification

Commands run:

- `cargo fmt`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `cargo run --bin validate_fixtures`
- `cargo run --bin simulate_fixture -- --fixture retry-storm-amplification --dry-run`
- `cargo run --bin simulate_fixture -- --fixture deploy-bad-rollout --jsonl`
- `git diff --check`

Results:

- All commands passed.
- `cargo test` now includes 12 `fixture_simulator` tests and 15 `hot_context_store` tests.
- `validate_fixtures` reported 0 errors and 0 warnings across 12 fixtures.
- The retry-storm dry run now orders the whole-second `2026-06-05T14:45:01Z` log before the later `14:45:01.330Z` and `14:45:01.990Z` spans.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
