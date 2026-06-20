# Fixture OTel Simulator Review 4

- Baseline SHA: `fd3f66f34af09ed8a8bef84b01c4e7cc47127bd8`
- Current milestone: a `simulate_fixture` CLI and ingest-like replay path that replays a registered fixture into a fresh `HotContextStore`, validates source refs after replay, and proves partial-replay ref availability with tests.
- Critical path: yes - this round completes the approved slice 3 demo and validation surface.
- Milestone progress: resolved the raw-vs-derived validation decision in the formal design doc, added default CLI replay summary mode, validated raw gold-bundle source refs after input replay, reported non-replayed derived refs as skipped, checked manifest query time-window coverage, and added corpus-wide/default-CLI tests.
- Deferred milestone work: none for the current milestone. Optional `--until`, `--ref`, and `--query` flags remain out of scope, and derived-context production remains explicitly out of scope for this topic.

## Response To Review 3

Review-3 approved slice 3 after resolving the S3 design decision.

- S3, raw-vs-derived validation: I chose option (b). The simulator validates only raw source refs from the gold bundle against the input-replayed store. Derived refs such as anomaly windows, log patterns, evidence items, entities, and relationships are counted and reported as skipped rather than loaded from `expected.json` or derived by this topic. `docs/core/fixture-otel-simulator.md` now makes this the Definition of Done.
- m1, metric time-window string bounds: left unchanged. Timestamp normalization is applied to replay ordering; stored metric windows still preserve original fixture timestamp strings and use the existing store behavior. No `--until` or time slicing landed in this topic.
- m2, span `trace_id` from fixture source key: added a short code comment noting that fixture span keys are `trace_id/span_id`, while real OTLP ingest should carry trace IDs directly.

## Implementation Summary

This round adds the final simulator path:

- `replay_fixture_case` builds the replay plan, ingests all events into a fresh `HotContextStore`, loads the fixture gold bundle, validates the bundle shape, validates replayed raw refs, checks the manifest time window against replayed records, and returns a `FixtureReplaySummary`.
- `replay_plan_into_store` exposes the plan-to-store adapter used by the summary path and future tests.
- `format_replay_summary` prints a deterministic compact report with event count, stored record count, raw refs resolved, non-replayed refs skipped, query time-window matches, and validation errors.
- `simulate_fixture --fixture <id>` now runs the default replay summary. `--dry-run` and `--jsonl` remain inspection modes and are still mutually exclusive.
- Tests now cover corpus-wide replay summaries, skipped non-replayed refs, default CLI replay, dry-run, JSONL, timestamp ordering, empty-span trace fallback, and prior slice behavior.

Example summary for `deploy-bad-rollout`:

```text
fixture deploy-bad-rollout replay summary
events emitted: 28
records stored: 20
raw source refs resolved: 6
non-replayed source refs skipped: 2
query time-window records: 17
validation errors: 0
```

## Review Focus

Please focus on whether this closes the topic milestone:

1. Is the S3 option (b) formal-doc wording clear enough that reviewers will not expect derived refs to resolve from input replay?
2. Is the compact summary sufficient for the local demo milestone?
3. Are raw source-ref validation and skipped non-replayed ref counts the right success criteria for this topic?
4. Is the small public replay-summary API acceptable, or should any part remain private to the CLI for now?
5. If no defects or new requirements are found, should the topic stop after this review rather than producing review-5?

## Verification

Commands run:

- `cargo fmt`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `cargo run --bin validate_fixtures`
- `cargo run --bin simulate_fixture -- --fixture deploy-bad-rollout`
- `cargo run --bin simulate_fixture -- --fixture deploy-bad-rollout --dry-run`
- `cargo run --bin simulate_fixture -- --fixture deploy-bad-rollout --jsonl`
- `git diff --check`

Results:

- All commands passed.
- `cargo test` now includes 15 `fixture_simulator` tests and 15 `hot_context_store` tests.
- `validate_fixtures` reported 0 errors and 0 warnings across 12 fixtures.
- Default `simulate_fixture --fixture deploy-bad-rollout` reported 28 events, 20 stored records, 6 raw source refs resolved, 2 non-replayed refs skipped, 17 query time-window records, and 0 validation errors.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
