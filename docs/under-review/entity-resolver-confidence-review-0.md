# Entity Resolver Confidence Review 0

- Baseline SHA: `f17b918bf0f6813e8205865ed57da542ef6dcbcd`
- Current milestone: reviewer-approved Milestone 5A design direction for source-backed entity and relationship context before any Rust implementation begins
- Critical path: yes - entity and relationship context is the next Derived Context V1 slice, and the User explicitly requires design agreement before coding
- Milestone progress: submits `docs/core/entity-resolver-confidence.md` for direction review after clarifying raw-source-only derivation, phase approval, and deterministic relationship store keys
- Deferred milestone work: all Rust implementation of the resolver, relationship builder, comparison helper, store insertion path, CLI, and tests is intentionally deferred until every active reviewer agrees on the design direction or explicitly approves a named phase

This is the first review round for `entity-resolver-confidence`. There are no
prior review findings to answer.

The design under review is `docs/core/entity-resolver-confidence.md`. It defines
Milestone 5A: derive operational entities and relationships from records already
in `HotContextStore`, attach confidence and provenance, make ambiguity visible,
and compare the derived context against fixture gold `entities` and
`relationships`.

This round is design-only. No Rust implementation should start until every
active reviewer has agreed on the design direction in their `Direction Verdict`.
If reviewers want implementation to proceed phase by phase, the verdict should
name the approved phase explicitly; otherwise the default is whole-design
approval before coding.

The current design deliberately keeps this topic out of anomaly detection, log
clustering, timeline generation, Evidence IR generation or ranking, MCP,
persistence, new ingest protocols, and dashboard/UI work. Those remain later
milestones or separate topics.

Reviewers should focus on these direction questions:

1. Is returning to `entity-resolver-confidence` now correct after the hot store,
   fixture simulator, and OTLP JSON ingest topics, or is another prerequisite
   still missing?
2. Are the entity identity rules deterministic enough for the current fixture
   corpus without hard-coding fixture answers, especially for
   `ambiguous-entity-resolution`?
3. Does the confidence model clearly separate entity/relationship mapping
   confidence from causal confidence, so this topic does not become root-cause
   ranking?
4. Are alternatives, unresolved entities, missing attributes, and estimated
   unresolved share concrete enough to prevent silent false causality from
   merged identities?
5. Is the raw-source-only boundary strong enough? In particular, does the design
   adequately prevent the resolver from copying gold `Entity` and
   `Relationship` records that `HotContextStore::load_fixture_case` may load as
   expected derived reference targets?
6. Are relationship derivation, relationship confidence, evidence refs, and the
   deterministic store key for relationship triples sufficient for Milestone 5A?
7. Is the fixture comparison contract strict enough for required gold entities
   and relationships while still allowing deterministic, source-backed extra
   runtime entities?
8. If implementation is approved, should it proceed as the proposed slices
   (data model/store read boundary, entity resolver, relationship builder,
   fixture comparison/tests, optional CLI), or should reviewers require the full
   design to be finalized and implemented in one coding round?

The requested reviewer output is a `Direction Verdict` that explicitly says
whether implementation may begin after review, must wait for another
design-only round, may begin only for a named phase, or should be redirected.

## Verification

No code verification this round. Design-only review preparation included
reading:

- `docs/review-framework.md`
- `AGENTS.md`
- `docs/core/what_and_why.md`
- `docs/core/roadmap.md`
- `docs/core/hot-context-store.md`
- `docs/core/fixture-otel-simulator.md`
- `docs/core/otel-ingest-prototype.md`
- `docs/core/entity-resolver-confidence.md`
- `docs/process/fixtures.md`
- `src/hot_context_store.rs`
- `src/fixture_validation.rs`
- `fixtures/scenarios/ambiguous-entity-resolution/input.json`
- `fixtures/scenarios/ambiguous-entity-resolution/expected.json`

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
