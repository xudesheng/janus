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

## Review (by Claude)

### Direction Verdict

**On critical path, milestone moved decisively, continue — slice 3 approved.** This is the
riskiest slice (store mutation, merge semantics, partial-replay availability) and it lands
correctly, with proof rather than assertion. I re-ran the full verification: `cargo test`
green (12 `fixture_simulator` + 15 `hot_context_store` tests), `cargo clippy --all-targets
--all-features` clean (exit 0), `validate_fixtures` 0/0 across 12 fixtures, and I confirmed
the F1 fix on the real same-second case — `retry-storm-amplification` now orders
`2026-06-05T14:45:01Z` *before* `14:45:01.330Z`/`.990Z`. All four review-2 findings (F1–F4)
are closed with tests, and review-1's Findings A–D are now *proven by tests*, not just
specified. Next action: **continue to slice 3**, after resolving the one forward decision in
Finding S3 below.

### Milestone progress (judged first)

- **Merge semantics are correct and proven.** `ingest_metric_point` creates the series on
  the first point and appends subsequent points only when non-point metadata matches
  exactly; conflicting metadata returns `DuplicatePrimaryKey`. The negative test
  (`metric_series_is_the_only_merge_eligible_ingest_key`, conflicting `unit` → error) is
  exactly the contract review-1 Finding C / review-2 note 1c asked for. The accumulated
  record converges to `{name, entity, unit, points:[…]}`, satisfying Finding B.
- **The duplicate-key invariant survives.** Non-metric ingest flows through the existing
  primary-key machinery; `non_metric_duplicate_ingest_keys_remain_errors` proves a repeated
  `Log` id still errors. Metric is the *only* merge path. Finding C holds end to end.
- **Partial-replay availability is proven precisely.** `partial_replay_..._change_ref` shows
  a ref Missing-before / Found-after its owning event;
  `partial_replay_distinguishes_trace_and_span_availability` shows the trace ref resolving
  while its span ref is still Missing until the span event — exactly review-1 Finding D and
  the design's trace/span rules.
- **Resolution-equivalence, not payload-equality, is the bar.** `full_replay_resolves_
  current_raw_evidence_source_refs` checks `Found` for raw refs across the whole corpus
  through the simulated ingest path, honoring Finding A. The trace record still carries
  embedded spans (same as the batch loader), so resolution stays equivalent.
- **F1 closed with a test that earned its keep.** The corpus-wide
  `timed_events_are_monotonic_by_parsed_fixture_time` uses an independent parser and caught
  a real latent `retry-storm` mis-order — precisely the regression my review-2 F1 note (b)
  was meant to prevent. The `fixture_timestamp_sort_key` `(base, nanos)` design is sound:
  `base` is always the fixed-width 19-char prefix, so its lexical order equals chronological
  order, and a whole-second timestamp sorts before the same-second fractional one.

### Headline forward finding (resolve before slice 3, not a slice-2 defect)

- **[S3 — design decision] "Full replay resolves *all* evidence source refs" collides with
  out-of-scope derivation.** Slice 2 correctly scopes its full-replay test to *raw* refs
  (Trace/Metric/Log/Change/PriorIncident/TelemetryGap). But the DoD says full replay
  resolves *all* current fixture evidence source refs, and gold bundles also carry *derived*
  refs (anomaly_window, log_pattern, evidence_item, entity, relationship). The batch loader
  resolves those only because `load_fixture_case` loads `input` **and** `expected`; the
  simulator replays `input` only, `HotIngestEvent` has no variants for derived kinds, and
  derivation is explicitly out of scope. So slice 3's "bundle source-ref validation after
  replay" cannot resolve derived refs from input replay alone. Decide before slice 3, and
  write it into `docs/core/fixture-otel-simulator.md`:
  - **(a)** the simulator also loads expected-side derived records into the store via a
    separate, non-`ingest` validation path (mirroring `load_expected`), purely so the gold
    bundle's derived refs resolve; or
  - **(b)** bundle validation is scoped to raw refs, and the DoD is amended to say "raw
    source refs" instead of "all evidence source refs."
  Both are defensible; (b) is smaller and keeps derivation cleanly out of this topic. What
  is not acceptable is shipping slice 3 against the current "all refs" DoD wording while the
  simulator structurally cannot produce derived records.

### Minor findings (fold into slice 3; none block)

- **[m1] Stored metric time windows still use raw-string min/max, not the normalized key.**
  The F1 normalization fixed *ordering* only; `merge_time_window` and the metric series
  `time_window` compare original timestamp strings. With consistent per-series formats this
  is exact and matches prior batch behavior; a mixed-format series could yield a slightly
  off lexical window bound. Low impact (windows are second-tolerant) — just note that
  normalization was deliberately not extended to stored windows.
- **[m2 — nit] Span `trace_id` round-trips through `source_key`.** `TryFrom` recovers
  `trace_id` by `split_once('/')` on the source key, then `ingest` rebuilds the key via
  `span_ref`. Correct for fixture ids, but it assumes a trace id never contains `/`. Fine to
  leave; worth a comment so a future real-OTLP id scheme does not silently break it.

### Answers to the round's questions

- **Q1 (`HotIngestEvent` reusable / metric payload too fixture-shaped?)** — Reusable enough;
  `MetricSeriesKey` is a clean abstraction and the fixture-shaping is the same source-owned
  key-derivation seam already accepted in review-1 Q2. No derived-kind variants is correct,
  since real OTLP will not emit derived records either.
- **Q2 (metric merge correct?)** — Yes: exact metadata equality excluding `points`,
  append-only accumulation, conflicting metadata errors. Proven.
- **Q3 (non-metric duplicate invariant strong enough?)** — Yes, proven by test.
- **Q4 (partial replay proves the right semantics?)** — Yes, both the owning-event and
  trace-before-span cases.
- **Q5 (raw-ref full-replay scope acceptable for slice 2?)** — Yes, with Finding S3 flagged
  for slice 3.
- **Q6 (proceed to slice 3?)** — Yes, approved — resolve S3 first (a one-paragraph design
  decision), since slice 3 *is* the bundle-validation slice S3 concerns.

### Framework / process check (passes)

- Baseline `45f629a` is pushed and is the parent of the review-3 commit `f0b7a7c`
  (pre-review-document tree); frozen. Correct.
- Implementation round: `## Verification` lists real commands; I reproduced them
  independently and they pass as claimed (including the 12/15 test counts and the
  retry-storm ordering claim).
- Locked rounds 0–2 are unchanged (`070c288..f0b7a7c` touches only the round-3 doc, the
  formal design doc, and source/test files). Editing the formal design doc is permitted (it
  carries no review section). Locking honored.
- Header fields complete; milestone names the artifact; deferred work correctly lists the
  slice-3 items.

### Bottom line

Excellent slice-2 round — the hardest slice, delivered with proofs for every prior finding
and a test that caught a real bug. **Continue to slice 3** (default CLI replay, compact
summary, bundle/source-ref validation), but first make the S3 decision and record it in the
design doc so slice 3's validation scope and the DoD wording agree.
