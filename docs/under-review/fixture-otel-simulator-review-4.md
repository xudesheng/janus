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

## Review (by Claude)

### Direction Verdict

**Milestone complete — stop here; do not submit review-5.** This round delivers slice 3 and
closes the topic's Definition of Done. I re-verified every DoD gate independently: `cargo
fmt --check` clean, `cargo test` green (9 + 12 + 15), `cargo clippy --all-targets
--all-features` clean (exit 0), `validate_fixtures` 0/0 across 12 fixtures, and the default
`simulate_fixture --fixture deploy-bad-rollout` prints exactly the claimed summary
(28 events / 20 records / 6 raw resolved / 2 skipped / 17 window / 0 errors). The S3 decision
I asked for in review-3 is resolved cleanly as option (b). I found **no defects and no new
requirements**; the two notes below are optional, non-blocking polish that do **not** by
themselves warrant another round. Next action for the Implementor: **report completion to
the User and stop** (this answers Q5: yes).

### Milestone progress (judged first) — DoD met

I checked the slice-3 work against the topic's Definition of Done, item by item:

- **Replay through the ingest adapter** — `replay_plan_into_store` ingests every planned
  event into a fresh `HotContextStore` via the slice-2 boundary. ✓
- **Metric points land under expected series refs** — unchanged from slice 2, still proven. ✓
- **Full replay resolves raw refs, derived reported as skipped (S3 / option b)** —
  `validate_raw_source_refs` resolves raw signals and counts non-raw as skipped; I confirmed
  the only non-raw signals in the entire fixture corpus are `anomaly_window` and
  `log_pattern` (both genuinely derived), so the skip path is exactly the derived refs. The
  validator hard-errors on any *raw* ref that fails to resolve, which is the right success
  criterion. ✓
- **Partial-replay availability** — the slice-2 tests proving Missing-before/Found-after and
  trace-before-span remain in place. ✓
- **Deterministic local CLI summary** — `format_replay_summary` is a fixed-field, ordered
  report; default mode wired correctly, with `--dry-run`/`--jsonl` preserved as mutually
  exclusive inspection modes. ✓
- **Scope exclusions held** — no OTLP protobuf, network receiver, persistence, derivation,
  ranking, or MCP surface introduced. The design doc DoD wording was tightened to "raw
  source refs … reports derived refs as skipped," so the spec and the code now agree. ✓
- **Toolchain gates** — fmt / test / clippy / validate_fixtures all pass. ✓

The error taxonomy (`FixtureReplayError` with `source()` chaining and `describe_resolution`
diagnostics) and the empty-match guard on the manifest time-window check are good defensive
touches beyond the minimum.

### Minor notes (optional polish — NOT blocking, do NOT require a review-5)

- **[n1] `is_raw_replay_source_signal` buckets `Profile` and `External` with the derived
  signals.** The raw set is Trace/Metric/Log/Change/PriorIncident/TelemetryGap; everything
  else (including `Profile` and `External`) is counted as "skipped." No current fixture
  references either, so this is harmless today and the corpus-wide numbers are correct. But
  `Profile` is semantically a *raw* telemetry signal the simulator simply doesn't model, not
  a derived one — if a future fixture adds a profile ref it would be silently counted as a
  "skipped derived" ref instead of flagged as an unmodeled raw signal. Optional: distinguish
  "unmodeled raw" from "derived/out-of-scope," or note `Profile`/`External` explicitly. This
  is a natural thing to carry into the future real-OTLP topic rather than reopen this one.
- **[n2] `FixtureReplaySummary.validation_errors` is structurally always `0`.** Any
  validation failure returns `Err(FixtureReplayError)` and never produces a summary, so a
  returned summary's `validation errors:` line is always `0`. The field communicates nothing
  today. Optional: drop it, or change the validator to aggregate non-fatal findings into it.
  Cosmetic; a one-line cleanup commit would suffice if the Implementor wants it — it does not
  need a review round.

### Answers to the round's questions

- **Q1 (S3 option (b) wording clear?)** — Yes. The design doc now says the simulator replays
  `input.json` only and reports derived refs as skipped, and the DoD matches. A reviewer will
  not expect derived refs to resolve from input replay.
- **Q2 (compact summary sufficient for the demo milestone?)** — Yes; deterministic and
  legible.
- **Q3 (raw validation + skipped counts the right criteria?)** — Yes, with the n1 caveat for
  future non-`derived` non-raw signals.
- **Q4 (public replay-summary API acceptable?)** — Yes. `replay_fixture_case`,
  `replay_plan_into_store`, `format_replay_summary`, and the summary/error types are a
  reasonable module surface; nothing needs to be hidden.
- **Q5 (stop after this review rather than produce review-5?)** — **Yes, stop.** Milestone
  complete, no defects, no new requirements. Under the framework's Round Termination rule
  this is exactly the "report completion and wait" case, not a new round.

### Framework / process check (passes)

- Baseline `fd3f66f` is pushed and is the parent of the review-4 commit `5d2edf9`
  (pre-review-document tree); frozen. Correct.
- Implementation round: `## Verification` lists real commands; I reproduced them
  independently and they pass as claimed, including the exact default-CLI summary.
- Locked rounds 0–3 are unchanged (`f89cda1..5d2edf9` touches only the round-4 doc, the
  formal design doc, and source/test files). Editing the formal design doc is permitted (no
  review section there). Locking honored.
- Header fields complete; `Deferred milestone work: none` is accurate now that the DoD is
  met.

### Bottom line

The `fixture-otel-simulator` topic is **done**: a deterministic fixture→`HotContextStore`
replay path with raw source-ref validation, derived refs reported as skipped, and a local
CLI demo — built across four well-disciplined rounds with each prior finding closed and
proven. **Stop here; report completion to the User. No review-5.** n1/n2 are optional polish;
n1 belongs to the future real-OTLP topic, and n2 is a trivial cleanup if wanted — neither
requires another review round.
