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

## Review (by Claude (Opus 4.8))

### Direction Verdict

On critical path: **yes**. `entity-resolver-confidence` is Milestone 5A in
`docs/core/roadmap.md` ("Milestone 5A: Entity And Relationship Context"), the
strict next topic after the hot store, fixture simulator, and OTLP JSON ingest.
Returning here now is correct: those topics built the input surface, and this
topic is the first one that turns that input into agent-usable meaning rather
than widening ingest. Direction question 1 in the doc is answered yes.

Moves the milestone: **yes**. This design-only round secures the contract the
coding rounds need.

Verdict: I **AGREE with the design direction**, and I **approve a phased start
at Phase 1** (data model + store read boundary), with Phases 2 and 3 cleared to
proceed once Phase 1 lands. I do **not** give whole-design sign-off yet: the
design's own Design Review Gate says implementation must not start until the
comparison contract is tightened if reviewers find it too loose, and I find the
comparison contract (Phase 4) too loose in three concrete ways below. These are
cheap to fix and do not require another design-only round — fold them into the
formal design doc or the Phase-4 comparison contract before Phase 4 lands. Next
action: **continue**.

Note for the User: per the topic's gate, "every active reviewer must agree"
before coding. This verdict is one reviewer's. If other reviewers are active,
coding waits on their verdicts too.

I verified the design against ground truth rather than prose alone:
`HotContextStore` (`src/hot_context_store.rs`) exposes the read access the design
assumes (`records()`, `select(SourceQuery)` filtering by `kinds`/`entities`/
`time_window`, plus `StoredRecord.kind`); `StoredRecordKind` has `Entity` and
`Relationship` variants; `load_fixture_case` -> `load_expected` really does
insert gold `Entity`/`Relationship` records into the store (the leakage risk the
design flags is real); the roadmap confirms Milestone 5A; and the
`ambiguous-entity-resolution` `expected.json` matches the design's required
identities exactly (canary/stable/unresolved, mutual alternatives,
`missing_attributes`, `estimated_share: 0.18`, three `deployed-as` edges to
instance entities not present in gold `entities`).

### Required tightenings before Phase 4 (comparison contract)

These are the "comparison contract too loose" findings the gate asks reviewers
to judge. Highest-impact first.

1. **Pin the deterministic variant-token rule; the required ambiguous-fixture
   ids are not yet derivable.** Gold requires the literal ids
   `service:payments@stable` and `service:payments@unresolved`, but the
   discriminators are `service.version: 4.3.2` (stable) and missing version/
   instance (unresolved). A reasonable deterministic rule — "variant token =
   `service.version`" — would emit `service:payments@4.3.2`, which fails the
   exact-id match the contract requires. The design names the vocabulary
   `{canary, stable, unresolved}` but only says identities "may map to
   `@stable`". For the one required test case this must be pinned, e.g.: same
   `service.name` records partition by `(rollout, service.version,
   service.instance.id presence)`; `rollout=canary` -> `@canary`; non-canary
   records with a concrete version -> `@stable`; records missing **both**
   `service.version` and `service.instance.id` -> `@unresolved`. Also pin that
   the two stable resources (`res:payments-stable-a/-b`) **merge into one**
   `@stable` identity with two `deployed-as` edges, not two service identities.
   Without this the required fixture's exact-id assertions are not deterministic.

2. **Pin the required-fixture set; "or report a reviewed unsupported case" is an
   implementor-discretion escape hatch.** All 12 fixtures under
   `fixtures/scenarios/` have non-empty `entities` **and** `relationships` in
   `expected.json`, but the design's "first implementation priority" covers only
   a subset of relationship types (`calls`/`reads-from`/`writes-to`/
   `deployed-as`/`runs-on`/`depends-on`) while gold uses others
   (`retries`, `fans-out-to`, `shares-resource-with`, ...). The Definition of
   Done's "automated test **or** reviewed report" plus "reviewed unsupported
   case" lets a fixture pass via prose at the implementor's discretion. Tighten
   to: an explicit, committed list of fixtures whose gold `entities`/
   `relationships` **must** match by automated test for round 1 (at minimum
   `ambiguous-entity-resolution`, strict); any fixture deferred to "unsupported"
   must be named in that list with a reason, not omitted silently.

3. **Reconcile confidence bands with the `0.05` comparison tolerance.** Gold
   confidences are `0.96`/`0.95`/`0.4` with tolerance `0.05`, so derived values
   must land in `[0.91,1.0]`/`[0.90,1.0]`/`[0.35,0.45]`. But the unresolved
   **band** is `0.30–0.59`, wider than the `±0.05` tolerance window — a
   deterministic value that is "in band" (e.g. `0.55`) still fails the contract.
   State that the deterministic rule must hit within tolerance of the gold point
   value, not merely land in the band, for compared entities (or relax the
   per-entity tolerance and say so).

### Recommended (not blocking)

4. **Make raw-source isolation structural, not disciplinary.** The only public
   constructors are `new()`, `load_fixture_case()`, `insert_record`, `ingest`;
   there is no public raw-input-only construction path today, so the resolver
   relies on kind-filtering discipline to avoid copying the gold `Entity`/
   `Relationship` records `load_fixture_case` inserts. Strengthen review
   question 5 by adding a structural guarantee: either a raw-only read/
   construction path, or a test asserting the resolver never consumes
   `StoredRecordKind::Entity`/`Relationship`. This makes leakage impossible
   rather than merely avoided.

5. **Specify discriminator value comparison for scalar-vs-array.** Gold
   `@stable` has `service.instance.id` as an **array** (`["payments-7a",
   "payments-7b"]`) while `@canary` has it as a scalar. The model
   (`serde_json::Value`) accommodates both, but the comparison contract's
   "missing discriminators" check should state that discriminator values are
   compared set-wise/by-value (order-independent), consistent with the existing
   "match by id/name, not array order" rule for alternatives.

6. **Relationship store key grammar.** `relationship:{src}|{type}|{dst}` is a
   fine opaque `SourceKey` and does not collide with the evidence-ref grammar
   (`:`/`@`/`/`) because relationship **evidence** stays a separate ref list —
   confirmed against `src/references.rs`. No change needed; just keep the store
   key out of the scalar-ref resolver path, since `src`/`dst` themselves contain
   `:`/`@`.

### Answers to the doc's direction questions (brief)

- Q1 correct-to-return: **yes** (roadmap-confirmed).
- Q2 deterministic identity rules: **almost** — blocked on finding 1 (variant
  token) for the required fixture.
- Q3 mapping- vs causal-confidence separation: **yes**, the doc is explicit and
  correct ("Confidence must not be reused as causal confidence").
- Q4 ambiguity made first-class: **yes** — alternatives, unresolved bucket,
  `missing_attributes`, `estimated_share` all map to gold; this is the
  strongest part of the design.
- Q5 raw-source boundary: **adequate but disciplinary** — see finding 4.
- Q6 relationship derivation/store key: **yes** — see finding 6.
- Q7 comparison strictness: **too loose** — see findings 1–3.
- Q8 phased vs one round: **phased**, starting Phase 1; do not start Phase 4
  until findings 1–3 are folded into the contract.
