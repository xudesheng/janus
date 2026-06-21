# OTel Ingest Prototype Review 0

- Baseline SHA: `0fa0f2fe4e158aa52af55deb6b009d10692d2054`
- Current milestone: reviewer-approved design direction for the local OTLP JSON/file ingest prototype before any Rust implementation begins
- Critical path: yes - the User explicitly requires design agreement before coding, and this decision gates whether `otel-ingest-prototype` should proceed now or return to derived-context work
- Milestone progress: submits the initial `docs/core/otel-ingest-prototype.md` design for direction review and identifies the reviewer decisions needed before implementation
- Deferred milestone work: Rust implementation of the ingest adapter, CLI, fixture sample, and tests is intentionally deferred until every active reviewer agrees on the design direction

This is the first review round for `otel-ingest-prototype`. There are no prior
review findings to answer.

The design under review is `docs/core/otel-ingest-prototype.md`. It proposes a
narrow Milestone 9 preview that accepts OTLP JSON or Collector-exported JSON
from files or stdin, normalizes traces, metrics, logs, and resources into the
existing `HotIngestEvent` / `HotContextStore` boundary, and proves stable
source-ref generation and resolution. It deliberately avoids production
OTLP/gRPC, binary protobuf decoding, durable persistence, derived context,
ranking, MCP tools, and dashboard work.

The design keeps the implementation JSON/file-first unless reviewers explicitly
approve a small optional OTLP/HTTP JSON receiver. The central acceptance
criterion is not protocol completeness; it is whether OTel-shaped records can
enter Janus through the same hot-store ingest path as fixture simulation while
preserving auditable source refs.

Reviewers should focus on these direction questions:

1. Should `otel-ingest-prototype` proceed now as a demo-enabling Milestone 9
   preview, or should the project stop this branch and return immediately to
   the stricter derived-context roadmap topic, `entity-resolver-confidence`?
2. Is JSON/file-first the right first ingest boundary, or is OTLP/HTTP JSON
   required in the first implementation slice for the demo to be credible?
3. Are the proposed source-key rules stable and auditable enough for a
   prototype without pretending to solve full entity resolution?
4. Does the design reuse the existing `HotIngestEvent` and `HotContextStore`
   boundary strongly enough, or does it risk creating a parallel ingest path?
5. Are the exclusions strong enough to keep this topic from absorbing
   production ingest, persistence, derivation, ranking, MCP, or dashboard work?
6. If reviewers approve implementation, should it proceed phase by phase using
   the proposed slices, or should the full design be finalized before any code
   lands?

The requested reviewer output is a `Direction Verdict` that explicitly says
whether implementation may begin after review, must wait for another
design-only round, or should be redirected away from this topic.

## Verification

No code verification this round. Design-only review preparation included
reading:

- `docs/review-framework.md`
- `docs/core/what_and_why.md`
- `docs/core/roadmap.md`
- `docs/core/hot-context-store.md`
- `docs/core/fixture-otel-simulator.md`
- `docs/core/otel-ingest-prototype.md`

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

**On critical path:** No, not to the strict roadmap's nearest derived-context
deliverable. The nearest concrete roadmap artifact off `hot-context-store` /
`fixture-otel-simulator` is `entity-resolver-confidence` (Milestone 5A).
`otel-ingest-prototype` is a Milestone 9 topic and is the **second consecutive
demo-bridge detour** ahead of derived context, after `fixture-otel-simulator`.
The design is honest about this and asks reviewers to decide — good.

**Verdict: AGREE to proceed now, with conditions and a hard stop.** I approve
starting the JSON/file-first implementation (slices 1–3). I do **not** approve the
optional OTLP/HTTP receiver (slice 4) in this topic; it needs a separate explicit
approval and should stay design-only for now. After this topic completes, the
project should return to `entity-resolver-confidence`, **not** continue down the
M9 ingest chain (HTTP receiver, persistence, `change-event-ingest`). A third
consecutive demo detour would trip the framework's repeated-deferral guard.

**Why agree despite being off the strict path:**

1. `fixture-otel-simulator`'s entire justification was that it built a reusable
   ingest boundary "so a future OTLP receiver can reuse it." That promise is
   currently **unvalidated** — nothing has exercised `HotIngestEvent` /
   `HotContextStore::ingest` with a non-fixture source. This topic closes that
   loop cheaply and is the cleanest test of whether the boundary actually
   generalizes. That is real risk reduction, not just demo polish.
2. The scope is genuinely small, JSON-first, with strong exclusions, and it
   reuses the existing boundary rather than forking a parallel ingest path (I
   verified `HotIngestEvent`, `HotContextStore::ingest`, `IngestOutcome`,
   `MetricSeriesKey` all exist and are `pub` in `src/hot_context_store.rs`).
3. The User selected this topic and asked for it to be reviewed, which is the
   explicit-approval signal the framework requires for off-critical-path work.

**Next action: continue** (begin implementation), provided the must-resolve
findings below are settled in the design doc or in round 1 before merge.

### Boundary-reuse findings (verified against `src/hot_context_store.rs`)

These are the load-bearing ones for Review Focus #4 (reuse vs parallel path).

**M1 — `HotStoreError` is fixture-shaped; OTLP records have no fixture id
(must-resolve).** `HotStoreError::MissingField` and `InvalidShape` require
non-optional `fixture_id: String` and `file_path: PathBuf` (lines 109–121). The
design says malformed OTLP records "fail with structured errors and record
counts," but an OTLP record from an arbitrary file / stdin / HTTP body has no
`fixture_id`. The adapter must not fake one. Decide explicitly: either (a) relax
those context fields to `Option` / add an OTLP-origin error variant, or (b) wrap
store errors in a separate adapter-level error type carrying file path + JSON
path + envelope path instead of fixture identity. Name the choice in the design;
don't leave it to discover mid-slice.

**M2 — Metric-series `<entity>` derivation for OTLP is unspecified
(must-resolve).** `MetricSeriesKey` is `name@entity` with private fields, and
source-key stability is the topic's central acceptance criterion. The design
gives the metric key as `<metric.name>@<entity>` but never says how `<entity>`
is computed from an OTLP metric point (resource `service.name` →
`service:<name>`? data-point attributes? both?), nor what happens when no
service entity is resolvable (low-quality hint, or rejected record?). Two
implementors will pick two derivations and silently break key stability. Pin the
exact rule and the no-entity fallback.

**M3 — Resource key family diverges from the store's `resource id`
convention (clarify).** The store/fixture convention keys resources on
`resources[*].id`; OTLP has no such id, so the proposed
`resource:<service.name>@<service.instance.id>` is a new key family. That is
inherent to OTLP and acceptable, but the design should state that (a) a store
populated from OTLP will hold resource keys in a different shape than a
fixture-loaded store, (b) this is fine because stores are per-source / per-run
and never mixed, and (c) nothing in the resolver assumes the fixture
`resource id` shape. Confirm (c) holds.

### Design-quality findings

**Q1 — Generated log ids: count them, don't just narrate them.** Logs without
`janus.log.id` get a key from `traceId, spanId, timestamp, sequence` that is only
deterministic per-file, not Collector-stable. Since stable source refs are *the*
acceptance criterion, make the ingest summary expose a **counted field** (number
of logs that used a generated, non-stable id), not only the prose caveat the
design currently promises. The test list should pin "same input file → identical
generated log id."

**Q2 — Provenance field placement.** The design's "optional fields on the
payload, don't widen the enum" guidance is correct. Recommend injecting
provenance under a `_`-prefixed key (e.g. `_janus.provenance` with
`source = "otlp-json"`): it avoids colliding with real OTLP fields and aligns
with the hot store's existing rule of not indexing `_` helper annotations as
records. State the exact key in the design.

**Q3 — Make the test data genuinely OTLP-shaped, not a fixture re-skin
(schema-surface check).** OTLP JSON ingest is an external input-contract surface
(framework "Schema And Agent Surface Checks"). The Test Data section is good but
doesn't pin the OTLP envelope key names. Require the sample to use the real
nested OTLP JSON shape — `resourceSpans/scopeSpans/spans`,
`resourceMetrics/scopeMetrics/metrics{...dataPoints}`,
`resourceLogs/scopeLogs/logRecords`, lowerCamelCase throughout, hex `traceId` /
`spanId` — so the parser is proven against true OTLP/HTTP JSON, not a convenient
flat subset. Add an explicit test that parses a real `ExportTraceServiceRequest`-
shaped envelope.

**Q4 — Pin the summary contract for `--allow-partial` vs metric merges
(minor).** The summary distinguishes inserted vs updated (metric merges), and the
error policy makes rejected records exit nonzero unless `--allow-partial`. State
explicitly that metric-point *merges* (`IngestOutcome::Updated`) are normal
success, never count as "rejected," and never trip the nonzero exit. This keeps
the deterministic `--json-summary` contract unambiguous for tests.

### Answers to the design's direction questions

1. **Proceed now or return to derived context?** Proceed now, once, then return
   to `entity-resolver-confidence`. See verdict.
2. **JSON/file-first vs HTTP immediately?** JSON/file-first is correct. HTTP is
   not required for demo credibility and is not approved this topic.
3. **Source keys stable/auditable enough?** Yes for traces/spans/metrics once M2
   and Q1 are settled; the log-id and metric-entity derivations are the only
   stability gaps and both are addressable as above.
4. **Reuses `HotIngestEvent`/`HotContextStore` vs parallel path?** Reuse is real
   and verified. The only true coupling gap is M1 (error type); resolve it inside
   the existing boundary rather than by building a parallel ingest/error stack.
5. **Exclusions strong enough?** Yes. The out-of-scope list (gRPC, protobuf,
   persistence, derivation, ranking, MCP, dashboards, change-event ingest) is
   appropriately strict. My one addition: keep slice 4 (HTTP) behind a separate
   approval so the topic can't quietly grow a service lifecycle.

### Summary

Direction: agree, proceed with slices 1–3, HTTP deferred, return to derived
context afterward. Must-resolve before merge: **M1** (error type), **M2** (metric
entity derivation). Strongly recommended: **M3, Q1, Q2, Q3, Q4**. None of these
require another design-only round — they can land as decisions recorded in the
design doc plus round 1's implementation. Next action: **continue**.
