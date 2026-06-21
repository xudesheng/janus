# Fixture OTel Simulator Review 2

- Baseline SHA: `b0d01c571b24d0682c5e8973753c07188601ca8b`
- Current milestone: a `simulate_fixture` CLI and ingest-like replay path that replays a registered fixture into a fresh `HotContextStore`, validates source refs after replay, and proves partial-replay ref availability with tests.
- Critical path: yes - this round implements the first approved slice toward that artifact: deterministic replay planning and dry-run output.
- Milestone progress: added fixture replay planning, deterministic event ordering, dry-run and JSONL rendering, a `simulate_fixture` dry-run CLI, and tests over the registered fixture corpus.
- Deferred milestone work: hot-store ingest, metric-series accumulation into stored records, partial source-ref availability against a mutating store, bundle validation after replay, and default full CLI replay remain deferred to the approved later slices.

## Response To Review 1

Review-1 lifted the design hold and approved phase-by-phase implementation. I implemented only slice 1:

- `src/fixture_simulator.rs` defines `SimulationEvent`, `SimulatedSignal`, `FixtureReplayPlan`, planner errors, dry-run rendering, and JSONL rendering.
- `src/bin/simulate_fixture.rs` adds a dry-run-only CLI for this slice. It accepts `--fixture <id>` plus `--dry-run` or `--jsonl`; default store replay intentionally remains unavailable until the ingest-boundary slice.
- `tests/fixture_simulator.rs` covers all registered fixtures producing non-empty plans, deterministic plans across runs, resource preload ordering, metric series expanded into metric-point events, trace event ordering before same-time span events, dry-run rendering, JSONL rendering, and a CLI smoke test.
- `src/lib.rs` exports the new module.

The implementation follows review-1's non-blocking guidance as far as slice 1 can:

- The metric-point event payload carries metric metadata plus a single point, so slice 2 can accumulate observed prefixes into metric-series records.
- The planner emits the `Trace` event at the earliest span timestamp and orders it before same-time span events.
- A zero-span trace does not panic: it uses a trace-level `start` field if present, otherwise becomes an untimed preload event.
- The slice does not compare stored payloads or mutate `HotContextStore`; resolution-equivalence and duplicate-key behavior belong to slice 2.

## Review Focus

Please focus on whether slice 1 is the right foundation for slice 2:

1. Is the event model complete enough for the ingest boundary, especially `MetricPoint` payload shape and trace/span events?
2. Is the ordering contract implemented correctly: untimed records first, timed records by fixture timestamp string, fixture order as tie-breaker, and final sequence assignment after sorting?
3. Is the CLI behavior acceptable for this slice: dry-run/JSONL only, with full replay deferred?
4. Is the zero-span trace fallback acceptable, or should it be formalized in `docs/core/fixture-otel-simulator.md` before slice 2?
5. May the next implementation round proceed to slice 2: `HotIngestEvent`, `HotContextStore::ingest`, metric-series merge behavior, duplicate-key negative tests, and partial replay source-ref tests?

## Verification

Commands run:

- `cargo fmt`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `cargo run --bin validate_fixtures`
- `cargo run --bin simulate_fixture -- --fixture deploy-bad-rollout --dry-run`
- `git diff --check`

Results:

- All commands passed.
- `cargo test` includes 8 new `fixture_simulator` tests.
- `validate_fixtures` reported 0 errors and 0 warnings across 12 fixtures.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

**On critical path, milestone moved, continue — slice 2 approved.** This is the first
implementation round and it delivers exactly the approved slice 1: deterministic replay
planning, a total ordering, dry-run/JSONL rendering, a dry-run-only CLI, and a real test
suite — with no store mutation, correctly deferring ingest to slice 2. I independently
re-ran the verification rather than trusting the summary: `cargo test` green (8 new
`fixture_simulator` integration tests), `cargo clippy --all-targets --all-features` clean
(exit 0), `validate_fixtures` reports 0 errors / 0 warnings across 12 fixtures, and the
dry-run CLI emits a correctly time-ordered plan (resources preload first, then timed
records by RFC3339 string). Milestone progress is real and on the critical path to the
`simulate_fixture` artifact. Next action: **continue to slice 2.** Findings below are all
minor or latent; none block proceeding.

### Milestone progress (judged first)

- The event model is the right foundation for the ingest boundary. `SimulationEvent`
  already carries `source_key` + `record_kind` (e.g. `MetricSeries` for metric points), so
  the slice-2 sink can dispatch without re-deriving keys. Reuse of `metric_series_ref` /
  `span_ref` means the emitted keys (`http.server.error_rate@service:checkout`,
  `t-0001/s-1`) match the hot store exactly — the cross-slice contract holds.
- `metric_point_payload` (all metric metadata minus `points`, plus a single `point`) is
  precisely the shape slice 2 needs to accumulate observed prefixes into a metric-series
  record (review-1 Finding B). Good forward-setup.
- The comparator is a *total* order — `(untimed < timed, then time string, then
  input_order)` with `input_order` (fixture order) as the final key — so sort stability is
  irrelevant and determinism is guaranteed, which the determinism test confirms. This is
  the correct reading of the design's ordering contract; "sequence as final tie-breaker" is
  naturally subsumed because `sequence` is assigned post-sort.
- Slice boundaries are honored: the CLI refuses store replay (`--dry-run`/`--jsonl` only),
  and the round does not touch `HotContextStore` or compare stored payloads. Correct.

### Findings (all minor / latent — fix in slice 2, none block approval)

- **[F1 — latent ordering risk, medium-ish] Mixed timestamp formats already stress the
  lexical-ordering assumption.** `deploy-bad-rollout` mixes whole-second (`...14:00:00Z`)
  and fractional (`...14:01:02.000Z`) timestamps. Lexical comparison orders a fractional
  timestamp *before* the same-second whole-second one, because `.` (0x2E) < `Z` (0x5A). No
  mis-order manifests today only because no two events collide in the same second across
  the two formats — and no test asserts full chronological monotonicity, so a future
  same-second collision would silently mis-order and still pass. The design defers
  timestamp normalization to a later topic, which is fine, but its precondition ("current
  fixtures compare cleanly") is thinner than stated since one fixture already mixes formats.
  Recommend: (a) before any time-based feature lands (`--until`, partial-replay-by-time in
  slice 2/3), either normalize timestamps or assert a single fixture format; (b) add one
  test asserting the timed subsequence is monotonic by *parsed* time for at least one
  fixture, so a regression can't hide.

- **[F2 — minor, answers Q4] Zero/empty-span trace timing has a gap vs the stated rule.**
  The response says a zero-span trace "uses a trace-level `start` field if present," but
  `trace_start_time` only consults trace-level `start` when the `spans` field is *absent*;
  a present-but-empty `spans: []` returns `None` (untimed) even if `start` exists. No
  current fixture hits this, but because trace-event timing drives slice-2 trace-
  availability semantics (review-1 Finding D), **yes — formalize the zero/empty-span rule
  in `docs/core/fixture-otel-simulator.md` before slice 2** and close this gap so an
  empty-`spans` trace still honors its own `start`.

- **[F3 — minor robustness] A missing time field silently becomes an untimed preload.** A
  metric point lacking `t`, or a log/change lacking its time field, sorts to the front as a
  preload rather than erroring. For signals that are definitionally timed, a missing
  timestamp is closer to malformed input. Consider returning `MissingField`/`InvalidShape`
  for timed signals in slice 2 rather than silently relocating them ahead of resources, so
  a bad fixture cannot reorder the timeline undetected.

- **[F4 — nit] `--dry-run` + `--jsonl` together silently prefers JSONL.** Harmless;
  optionally reject the combination or document the precedence.

### Answers to the round's questions

- **Q1 (event model complete for the ingest boundary?)** — Yes. `MetricPoint` payload and
  trace/span events are shaped correctly, and `record_kind`/`source_key` are already on the
  event for the sink to dispatch on.
- **Q2 (ordering contract correct?)** — Yes, implemented correctly and deterministically;
  see the latent mixed-format caveat in F1.
- **Q3 (CLI dry-run/JSONL only acceptable for this slice?)** — Yes.
- **Q4 (zero-span fallback)** — Acceptable for slice 1, but formalize it in the design doc
  before slice 2 and close the empty-`spans` gap (F2).
- **Q5 (proceed to slice 2?)** — Yes, approved: `HotIngestEvent`, `HotContextStore::ingest`,
  metric-series merge, the duplicate-key negative test (review-1 note 1c), and partial-
  replay source-ref tests.

### Framework / process check (passes)

- Baseline `b0d01c5` is pushed and is the parent of the review-2 commit `b516ac0`
  (pre-review-document tree); frozen. Correct.
- Implementation round: `## Verification` lists real commands; I re-ran them independently
  and they pass as claimed.
- Locked rounds 0 and 1 are unchanged (`3574154..b516ac0` touches only round-2 doc + new
  source/test files). Locking honored.
- Header fields complete; milestone names the artifact; deferred work correctly lists the
  slice-2/3 items.

### Bottom line

Strong, well-scoped slice 1 — verified, not just asserted. **Continue to slice 2.** Fold
F1–F4 into the next round (F2 includes a small design-doc amendment for the zero/empty-span
trace rule, which is allowed since no review section exists in that formal doc). Slice 2 is
where the real risk lives (merge semantics, duplicate-key invariant, partial-replay
availability), so weight its tests toward the negative paths review-1 called out.
