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

## Review (by Claude)

### Direction Verdict

**Conditional agree — do NOT start coding yet.** I agree with the design's *content
and scope*: it is well-bounded, technically grounded, and the incremental-replay
framing is the right architectural insight. But I cannot give an unconditional
"proceed" because the one decision that actually gates this topic — *should the
simulator be built now at all?* — is a roadmap/User call that this round has not yet
resolved. Next action: **redirect to the User for an explicit go/no-go on the demo
bridge**, then continue to implementation if approved.

Why this is the blocker, not a detail:

- The roadmap's Near-Term Review Order (`roadmap.md`) puts `entity-resolver-confidence`
  (Milestone 5) as the next topic after `hot-context-store`. Real ingest is Milestone 9
  (`otel-ingest-prototype`). `fixture-otel-simulator` is **not on the roadmap's stated
  critical path** — the design doc honestly says so itself ("The strict roadmap topic
  after the hot context store is `entity-resolver-confidence`. That is still the right
  next derived-context topic.").
- `review-framework.md` Milestone-and-Direction gate: off-critical-path work needs
  **explicit User approval** or it stays out of the review loop until the current
  milestone is unblocked. The round's `Critical path: yes` justification conflates two
  different things: "the User gated *coding* on reviewer agreement" is not the same as
  "the User approved spending this branch on a demo bridge ahead of derived context."
  The first is a process rule; the second is the direction decision still owed.
- The justification for jumping the queue is "demo pressure," but no concrete demo
  obligation is cited — no audience, date, or deliverable. Without that, this is useful
  tooling pulled forward ahead of critical-path Milestone-5 work, which is exactly the
  scope-creep/off-path case the framework asks reviewers to flag first.

So my answer to the implementor's **Q1** (proceed now vs. return to derived context):
*proceed only if the User confirms the demo bridge is wanted now.* If the User says
"return to the roadmap," this should stop as a design-only branch per the design's own
Review Gate. I am not rejecting the design — I am declining to let it skip the gate the
framework requires. To the implementor's **Q6** (phase-by-phase vs. one approved
design): if approved, phase-by-phase is fine and the proposed three slices are sensibly
ordered; slice 2 (ingest boundary + partial-replay tests) is where the real risk lives.

If the User approves proceeding, treat the design as **accepted with the tightenings
below** — these are specification gaps to close in the implementation round (or a short
design amendment), not reasons to re-gate.

### Direction answers to Q2–Q5

- **Q2 (boundary reusable for OTLP?)** — Plausible at the `HotContextIngestSink`
  interface, but the design overstates how solved this is. The part most likely to need
  rework for real OTLP is **source-key derivation**: the sink leans on fixture id
  conventions (`t-0001`, `t-0001/s-3`, `name@entity`) while real OTLP carries hex
  trace/span ids and OTLP metric identity. The event/sink *shape* can carry over; the
  *key normalization* is fixture-shaped. Add one sentence to the "Relationship To Real
  OTLP Ingest" section acknowledging key derivation as the seam a future
  `OtlpReceiverSource` will have to re-implement, so it is not assumed already reusable.
- **Q3 (strong enough to be a stream simulator?)** — Yes. The required behavior "ref
  missing before its owning event, resolvable after" plus metric-point accumulation is
  genuinely incompatible with `load_fixture_case`, which inserts one whole metric-series
  record per `(kind,key)` and treats a second insert as `DuplicatePrimaryKey`. The
  partial-replay tests prove the distinction. This is the strongest part of the design.
- **Q4 (metric/trace/span specified tightly enough?)** — Not quite; see Findings A, B, D.
- **Q5 (scope boundary right?)** — Yes, unreservedly. The out-of-scope list (no
  `opentelemetry-proto`, no HTTP/gRPC, no Collector, no persistence, no derivation, no
  evidence generation, no MCP) is exactly right and keeps this topic small. Keep it.

### Findings (close these in the implementation round if approved)

- **[A — substantive] Full-replay equivalence is two different bars and the doc uses
  both.** The doc calls `load_fixture_case` a "full-replay compatibility oracle" (implies
  the *stored records* match batch load) while the Definition Of Done only requires "full
  replay resolves all current fixture evidence source refs" (resolution equivalence, which
  is weaker). These diverge concretely: the batch loader stores a Trace record whose
  payload is the **whole trace object with `spans` embedded** (`load_traces`,
  `hot_context_store.rs:417`), but the incremental model emits `Trace(Value)` and separate
  `Span { trace_id, payload }`. A record-equality oracle would fail on the trace payload
  even when every ref resolves. State which equivalence you require. Resolution-equivalence
  is the right (lower-risk) bar; if so, stop calling `load_fixture_case` an oracle for
  record content and call it a ref-resolution oracle.

- **[B — substantive] Metric-series accumulation payload shape is unspecified.** Batch
  load stores one record per `name@entity` with the raw fixture metric object (incl.
  `unit` and the full `points[]`). Incremental ingest accumulates points, so after full
  replay the merged record must converge to that same shape for ref resolution (and any
  consumer) to stay unchanged. Pin the merge result: kept fields (`name`, `entity`,
  `unit`), `points[]` ordering (append in replay order = fixture order), and that no
  points are dropped. Otherwise slice-2 tests are guessing at the target record.

- **[C — substantive/contract] `ingest` weakens the duplicate-key invariant; name the
  exact exception.** The hot store's established guarantee is "duplicate primary keys with
  different payloads are errors." Metric-point accumulation deliberately breaks that for
  `MetricSeries`. The design must state precisely that **`MetricSeries` is the only
  merge-eligible `(kind, key)`**, and that all other kinds (Resource, Trace, Span, Log,
  Change, PriorIncident, TelemetryGap) keep hard `DuplicatePrimaryKey` semantics. Without
  that sentence, the ingest sink risks silently eroding the duplicate-detection guarantee
  the hot-store round just established.

- **[D — minor] "A trace ref resolves only after the trace has observed data" is
  ambiguous.** Define whether emitting the `Trace` event itself makes `t-0001` resolvable,
  or whether resolution requires ≥1 ingested span. A partial-replay test depends on this
  exact boundary. Pick one and state it (recommend: the Trace event makes the ref
  resolvable, since the batch loader resolves the trace ref independent of span presence).

- **[E — framework, minor] Round milestone vs. topic milestone.** `Current milestone:
  design approval` is a process gate, which is acceptable for a design-only round (the
  framework lets a design round's milestone be the alignment it secures). But name the
  **topic-level** milestone artifact explicitly — the DoD'd `simulate_fixture` CLI that
  replays a registered fixture into a fresh `HotContextStore` with passing partial-replay
  tests — so later rounds measure critical path against the artifact, not against
  "approval."

### Framework / process check (passes)

- Baseline SHA `b3eeedd` is the pushed parent of the review-doc commit `1b7fe88`
  (pre-review-document tree), and is frozen. Correct.
- Header fields (milestone, critical path, milestone progress, deferred) all present;
  design-only round correctly states "No code verification this round."
- Topic is issue-scoped and unique on a worktree branch; file path and `n=0` are correct.
- No code landed, honoring the design gate. Good.

### Bottom line

Strong, disciplined design — accept the *content* with Findings A–E closed. But the gate
this round must clear is the direction decision, and that one belongs to the User: **get
an explicit go/no-go on building the demo bridge ahead of `entity-resolver-confidence`
before any Rust lands.** I withhold unconditional agreement until then.
