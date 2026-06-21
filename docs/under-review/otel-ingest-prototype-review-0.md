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
