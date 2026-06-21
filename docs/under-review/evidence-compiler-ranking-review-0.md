# Evidence Compiler Ranking Review 0

- Baseline SHA: `0555a20b0d74affb05cc12f76e207ad75c3da16b`
- Current milestone: reviewer-approved Evidence Compiler V1 design in `docs/core/evidence-compiler-ranking.md`, enabling Milestone 6 implementation to start without crossing into MCP, persistence, production ingest, or dashboard scope
- Critical path: yes - this review is the required design gate before any Rust implementation for `evidence-compiler-ranking`
- Milestone progress: submits the Milestone 6 compiler/ranking design for direction review and identifies the design decisions that must be settled before coding
- Deferred milestone work: all Rust implementation is deferred because the topic requires all active reviewers to agree on the design direction first

This is the first design-only review round for `evidence-compiler-ranking`.
There are no prior findings to answer.

The draft under review is `docs/core/evidence-compiler-ranking.md`. I read it
against the canonical Janus vision, roadmap, Evidence IR contract,
`get_evidence_bundle` contract, hot context store, entity resolver, derived
context baseline, fixture simulator, OTLP ingest prototype, and fixture process
docs. I am not starting Rust implementation in this round.

The design direction I am submitting is:

- keep Milestone 6 focused on a source-backed Evidence Compiler V1;
- replace or explicitly transition away from the fixture-gold
  `get_evidence_bundle` return path;
- generate Evidence IR candidates from raw source records plus derived context,
  never from `expected.evidence_bundle`, `expected.suspected_causes`, or
  `expected.next_checks`;
- rank suspected causes and generate next checks as internal or store-visible
  outputs now, before MCP exposes them later;
- separate evidence strength from causal suspicion throughout the model;
- treat token budget as semantic whole-item evidence selection, not `LIMIT N`;
- make false-causality traps and missing-data uncertainty first-class
  acceptance criteria.

Please focus review on these decisions:

1. Whether the whole design is approved before coding, or whether implementation
   should proceed phase by phase. If phase by phase, please name the approved
   phase explicitly in the Direction Verdict.
2. Whether `get_evidence_bundle` should switch to compiler-generated bundles in
   this topic while keeping the current public request and response contract
   stable, or whether a temporary compiler-backed path should coexist with the
   fixture-gold stub.
3. Whether `suspected_causes` and `next_checks` belong in this milestone as
   internal/store outputs before the later MCP/agent surface exposes them.
4. Whether the draft is strict enough that fixture expected artifacts are
   comparison oracles only and cannot leak into runtime compilation.
5. Whether causal classification of nearby changes belongs in the evidence
   compiler, with Milestone 5B timelines remaining non-final context.
6. Whether the deterministic first token estimator and whole-item budget
   selection are acceptable for Milestone 6.
7. Whether the false-causality guard is concrete enough: low-ranked innocent
   suspects, explicit counter-evidence, missing-data uncertainty, and no
   confident root-cause prose.
8. Whether the scope exclusions are tight enough to prevent MCP, persistence,
   broader ingest, warm memory, dashboard, or mitigation work from entering this
   topic.

Reviewers should start their section with a Direction Verdict. A verdict that
allows implementation should say either that the whole design is approved or
which implementation slice is approved. Until all active reviewers agree in
their Direction Verdicts, implementation remains blocked by design.

## Verification

No code verification this design-only round.

Commands and checks performed:

- read `docs/review-framework.md`;
- read `docs/core/what_and_why.md`;
- read `docs/core/evidence-compiler-ranking.md`;
- read the linked formal docs needed to check the design boundary;
- confirmed the active branch is `evidence-compiler-ranking`;
- confirmed there were no local changes before creating this review document.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
