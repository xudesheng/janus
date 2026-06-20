# Hot Context Store Review 0

- Baseline SHA: `1d71357e10782eb52c3c96c767321145a2b14d85`
- Current milestone: Milestone 4 local hot context store that loads fixture
  inputs and same-fixture expected artifacts into an in-memory recent-window
  substrate, dereferences Evidence IR source refs to concrete records, exposes
  time-window and entity selectors, and lets the fixture-backed
  `get_evidence_bundle` path exercise store-backed source lookup.
- Critical path: yes - this store boundary is the next required substrate
  before simulator, OTLP ingest, real retrieval, or evidence compilation work.
- Milestone progress: clarified the formal design in
  `docs/core/hot-context-store.md` and submitted this design-review round with
  focused reviewer questions.
- Deferred milestone work: Rust implementation is intentionally deferred until
  every active reviewer agrees on the design direction. Live ingest,
  simulator work, derivation, ranking, durable persistence, and MCP/API
  surfaces remain out of scope for this milestone.

This is the first design-review submission for `hot-context-store`; there are no
prior review findings to address.

## Design Summary

The proposed Milestone 4 implementation is a small local store, not a retrieval
engine. It should load the validated fixture corpus into a `HotContextStore`
boundary, preserve fixture-shaped payloads as source material, assign stable
source keys and aliases, resolve source refs to concrete records, and expose
basic time-window and entity selectors.

The design deliberately keeps evidence generation unchanged. The existing
fixture-backed `get_evidence_bundle` path may still return the gold bundle, but
it should load the same fixture into the hot store and verify that returned
evidence source refs resolve to inspectable records. This moves Janus from
"gold JSON exists" to "gold evidence is attached to a recent-window context
substrate" without pulling Milestone 6 ranking or derivation forward.

The design doc was enriched in the baseline tree to make these review surfaces
explicit:

- recommended post-approval implementation slices;
- primary source keys versus resolver aliases;
- signal/category mismatch reporting;
- loader errors versus source-resolution outcomes;
- the rule that store-aware `get_evidence_bundle` must not rewrite, filter, or
  synthesize the returned gold bundle in this milestone;
- expanded review focus for scope, mismatch policy, selector semantics, and
  implementation slicing.

## Reviewer Focus Requested

Please start with the direction verdict: should this design proceed now for
Milestone 4, and is it on the critical path toward the roadmap's local hot
context store deliverable?

Specific points needing reviewer attention:

1. Store boundary: is the `StoredRecord` envelope plus JSON payload approach
   strong enough for later simulator and OTLP adapters while staying small?
2. Source refs: does the design clearly advance beyond Milestone 3 closure
   checks by requiring concrete record dereferencing?
3. Mismatch policy: should signal/category mismatches be hard failures
   immediately, or should the implementation keep a narrow compatibility
   warning path while current fixtures are cleaned up?
4. Scope control: is it correct to exclude live OTLP ingest, simulator work,
   derivation, ranking, durable persistence, and MCP/API surfaces from this
   topic?
5. Implementation slicing: after design approval, should this land in the
   proposed slices, or should Milestone 4 be implemented and reviewed as one
   coding round?
6. Selector and error semantics: are the proposed selector behavior,
   deterministic ordering, duplicate-key rules, alias-conflict rules, and
   structured failure cases precise enough to test?
7. Query integration: is it acceptable for store-aware `get_evidence_bundle` to
   return the same gold bundle unchanged while exercising source lookup and
   basic store selection checks?

If any active reviewer rejects the direction or asks for a redirect, no Rust
implementation should start. If reviewers approve only a phase-by-phase path,
the next round should implement only the approved phase and preserve the
Milestone 4 Definition Of Done.

## Verification

- Read `AGENTS.md`, `docs/review-framework.md`,
  `docs/core/what_and_why.md`, `docs/core/roadmap.md`,
  `docs/core/evidence-ir-schema.md`,
  `docs/core/get-evidence-bundle-contract.md`,
  `docs/process/fixture-validation-harness.md`,
  `docs/process/fixtures.md`, and
  `docs/core/hot-context-store.md`.
- Ran `git diff --check` after the formal design-doc enrichment: passed.
- No code verification this round; this is a design-only review submission.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
