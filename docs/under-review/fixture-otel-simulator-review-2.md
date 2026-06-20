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
