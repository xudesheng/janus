# Roadmap Review 0

- Baseline SHA: `01aba4228d8ab9e10a500de2e10354f311d379a1`
- Current milestone: committed `docs/core/roadmap.md` as the formal roadmap for moving Janus from design and fixtures toward a testable MVP.
- Critical path: yes - the roadmap sets the implementation order that future review topics will follow, starting with the Evidence IR contract.
- Milestone progress: added a milestone-based roadmap that sequences Evidence IR, `get_evidence_bundle`, fixture validation, hot context storage, derived context, evidence compilation, agent surface, comparative eval, ingest, warm/cold memory, and hardening.
- Deferred milestone work: none for this roadmap milestone. Implementation work begins in the next topic, expected to be `evidence-ir-schema`.

This first roadmap review covers `docs/core/roadmap.md`. The document is meant
to be a planning artifact, not an implementation spec or release schedule. It
should help keep Janus focused on the evidence-substrate thesis from
`docs/core/what_and_why.md` while giving implementation rounds concrete
milestones.

The roadmap currently makes these direction choices:

- Evidence IR and schema validation come before storage, ingest, and derivation.
- A narrow fixture loader appears in Milestone 1 because Evidence IR tests need
  fixture gold bundles before the full validation harness exists.
- `get_evidence_bundle` is the first investigation primitive, with fixture-backed
  gold output before real retrieval.
- Full fixture validation is separated into Milestone 3.
- Derived context is split into Milestone 5A for entities and relationships, and
  Milestone 5B for anomalies, log patterns, timelines, `find_related_anomalies`,
  and `compare_windows`.
- `previous_incident` evidence is fixture-sourced until warm/cold memory exists.
- The cold layer is treated as durable understanding plus backlinks, folded into
  the warm-memory and compaction milestone rather than full raw retention.

## Reviewer Focus

Please pay closest attention to direction and milestone boundaries, not wording
polish.

Key questions:

1. Is the roadmap faithful to `what_and_why.md`, especially the boundary that
   Janus is an evidence substrate, not OpenTelemetry replacement, APM UI, or RCA
   agent?
2. Is the sequence right: Evidence IR -> `get_evidence_bundle` -> fixture
   validation -> hot store -> derived context -> evidence compiler -> agent
   surface -> comparative eval?
3. Are Milestones 1 through 4 small enough to land as reviewable implementation
   rounds, or is any dependency still hidden?
4. Is the Milestone 5 split into entity/relationship first and
   anomaly/pattern/timeline second the right cut?
5. Are `find_related_anomalies` and `compare_windows` now given enough of a home
   to prevent fixture-roadmap drift?
6. Is the `previous_incident` boundary clear enough, given that fixture-provided
   prior incidents exist before real warm/cold memory?
7. Does Milestone 10 capture the cold-layer idea from `what_and_why.md` without
   implying long-term full raw telemetry retention?
8. Is anything missing from the roadmap that is required to prove the central
   bet: fewer, more accurate, more auditable evidence under the same agent,
   incident, time, and token budget?

## Verification

No code verification this round. This is a documentation-only roadmap review.

Commands run while preparing the covered roadmap and this review request:

- `Get-Content -Raw -Encoding utf8 docs/core/what_and_why.md`
- `Get-Content -Raw -Encoding utf8 docs/core/evidence-spine.md`
- `Get-Content -Raw -Encoding utf8 docs/process/fixtures.md`
- `Get-Content -Raw -Encoding utf8 docs/core/roadmap.md`
- `rg "Review \\(by Claude\\)|Milestone 2\\.|find_related_anomalies|compare_windows|previous_incident|cold-layer|Milestone 5A|Milestone 5B" docs/core/roadmap.md`
- `git status --short --branch`

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
