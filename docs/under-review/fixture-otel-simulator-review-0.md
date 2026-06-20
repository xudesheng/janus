# Fixture OTel Simulator Review 0

- Baseline SHA: `b3eeedd3ea08ade4387a3844d4f02017cb9845f5`
- Current milestone: design approval for `docs/core/fixture-otel-simulator.md` before any Rust simulator implementation starts.
- Critical path: yes - the User explicitly gated coding on reviewer agreement, and this design decides whether the demo bridge should proceed before returning to derived context.
- Milestone progress: enriched the formal design with an incremental replay requirement, trace/span availability semantics, and suggested implementation slices for phase-by-phase approval.
- Deferred milestone work: Rust implementation is intentionally deferred until every active reviewer agrees on the design direction; landing code in this round would violate the design gate.

This is a design-only first round for the `fixture-otel-simulator` topic. There are no prior review findings to answer.

The design draft already scoped the simulator as a deterministic fixture replay adapter, not real OTLP ingest. I tightened the formal doc around the main risk I saw after reading the current hot-store code: Janus already has `HotContextStore::load_fixture_case`, so the simulator must not become another all-at-once fixture loader. It must build a replay plan and feed a fresh store through an incremental ingest boundary.

The formal design now calls out these decisions:

- `HotContextStore::load_fixture_case` may be used as a full-replay compatibility oracle, but not as the simulator implementation path.
- Partial replay is required: a ref should be missing before its owning event is ingested, then resolvable after that event.
- Metric points should accumulate into the existing metric-series source key over replay.
- Trace and span refs should become available according to observed replay events, while preserving the current `trace_id` and `trace_id/span_id` key semantics.
- If reviewers approve phase-by-phase implementation, the proposed slices are replay planning/dry run, hot-store ingest boundary, then CLI and validation surface.

Please focus the review on direction and contract before local wording:

1. Should this simulator proceed now as a demo bridge before `entity-resolver-confidence`, or should the branch stop after design review and return to the roadmap's derived-context path?
2. Is the proposed `FixtureReplaySource -> HotContextIngestSink` boundary reusable enough for a later `OtlpReceiverSource`, or does the design still bake in fixture-only assumptions?
3. Are the incremental replay semantics strong enough to prove this is a stream simulator rather than a wrapper around `load_fixture_case`?
4. Are metric-point accumulation and trace/span availability specified tightly enough for tests without over-designing real ingest?
5. Is the scope boundary right: no `opentelemetry-proto`, no HTTP/gRPC receiver, no Collector config, no persistence, no derivation, no evidence generation, and no MCP surface?
6. Should implementation be approved phase by phase after this design round, or should the whole implementation wait for a single fully approved design?

## Verification

No code verification this round. Design-only checks performed:

- `git diff --check -- .\docs\core\fixture-otel-simulator.md` passed.
- Confirmed the working branch is `fixture-otel-simulator`.
- Confirmed the formal design-doc change was committed and pushed before this review document was created.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
