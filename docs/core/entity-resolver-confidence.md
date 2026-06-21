# Entity Resolver Confidence Design

Status: design for the `entity-resolver-confidence` topic.

This document defines the Milestone 5A entity and relationship context slice. It
is grounded in [`what_and_why.md`](what_and_why.md),
[`roadmap.md`](roadmap.md), [`hot-context-store.md`](hot-context-store.md),
[`fixture-otel-simulator.md`](fixture-otel-simulator.md), and
[`otel-ingest-prototype.md`](otel-ingest-prototype.md). If this document
conflicts with `what_and_why.md`, the canonical design doc wins and this
document should be corrected.

## Why This Topic Is Next

The project has now landed the two demo-enabling ingest topics:

- `fixture-otel-simulator`, which replays fixture-owned telemetry through the
  hot-store ingest boundary;
- `otel-ingest-prototype`, which ingests local OTLP JSON through the same
  `HotIngestEvent` and `HotContextStore` path.

That means the simulator and OTel ingestion paths have not been forgotten. They
now provide enough input surface for the next evidence-substrate problem: Janus
still needs to turn raw telemetry records into stable operational entities and
relationships that an agent can reason about.

The strict roadmap topic after the hot store is `entity-resolver-confidence`.
This is the right next topic now. Continuing deeper into HTTP receivers,
persistence, or `change-event-ingest` before entity context would expand the
input surface without improving Janus's ability to explain what the input means.

## Purpose

This topic should derive the first agent-useful operational context:

```text
HotContextStore records
      -> entity resolver
      -> relationship builder
      -> derived entity and relationship records
      -> fixture-gold comparison
      -> later anomaly, timeline, and evidence compiler inputs
```

The output should make these facts explicit:

- which services, routes, instances, pods, databases, queues, external APIs,
  tenants, and other operational objects are present;
- which records support each entity mapping;
- which relationships are observed between entities;
- how confident Janus is in each mapping or relationship;
- what alternatives or unresolved states remain.

This topic is not about producing final evidence bundles or root-cause
rankings. It creates the derived context those later steps need.

## Design Review Gate

No Rust implementation should start for this topic until every active reviewer
has agreed on the design direction in their `Direction Verdict`.

Reviewers should explicitly decide whether the identity rules and confidence
model below are strong enough for Milestone 5A, especially for the
`ambiguous-entity-resolution` fixture. If reviewers find that the comparison
contract is too loose, the implementation should not start until that contract
is tightened.

## Scope

In scope:

- a small entity model compatible with fixture `expected.json` entities;
- a resolver that reads recent source records from `HotContextStore`;
- deterministic entity ids for the current fixture corpus;
- confidence, alternatives, unresolved states, missing attributes, and
  discriminators;
- relationship derivation for dependency and runtime relationships;
- relationship confidence and source evidence;
- insertion or exposure of derived entity and relationship records through the
  hot-store derived-record boundary;
- comparison against fixture gold `entities` and `relationships`;
- tests that prove ambiguous identities are not silently merged.

Out of scope:

- anomaly detection;
- log clustering;
- timeline generation;
- evidence item generation or ranking;
- causal classification;
- MCP or external API surfaces;
- persistence beyond the current local hot store;
- OTLP/HTTP, OTLP/gRPC, or additional ingest protocols;
- `change-event-ingest` as an external API;
- dashboard or UI features.

## Inputs

The resolver should consume records already present in `HotContextStore`, not
fixture files directly. This keeps the topic useful for all current input paths:

```text
fixture loader          -> HotContextStore -> entity resolver
fixture simulator       -> HotContextStore -> entity resolver
OTLP JSON ingest sample -> HotContextStore -> entity resolver
```

If the current store API does not expose enough read access, add a narrow
read-only iterator or query method. Do not make the resolver reach into private
store internals or parse fixture files as its primary path.

The first implementation should use these source record kinds:

- resources;
- spans and traces;
- metric series;
- logs;
- changes;
- telemetry gaps when they name affected entities.

Expected derived artifacts remain fixture gold outputs. The resolver should
compare to them, not hand-load them as the derived answer.

## Output Model

Use a small Rust model that serializes close to the fixture shape:

```rust
struct ResolvedEntity {
    id: String,
    kind: EntityKind,
    from: Vec<String>,
    confidence: f64,
    discriminators: BTreeMap<String, serde_json::Value>,
    alternatives: Vec<EntityAlternative>,
    unresolved: bool,
    missing_attributes: Vec<String>,
    estimated_share: Option<f64>,
}

struct ResolvedRelationship {
    src: String,
    relationship_type: RelationshipType,
    dst: String,
    confidence: f64,
    evidence: Vec<String>,
    attributes: BTreeMap<String, serde_json::Value>,
}
```

Exact Rust names are flexible. The required contract is:

- entity ids are stable strings;
- entity kind is explicit;
- confidence is numeric and bounded from `0.0` to `1.0`;
- source support is preserved through `from` and relationship `evidence`;
- alternatives are visible when multiple identities are plausible;
- missing attributes are visible when Janus cannot safely resolve an identity;
- unresolved entities are first-class records, not dropped records.

Store integration may use existing `StoredRecordKind::Entity` and
`StoredRecordKind::Relationship`. If the implementation keeps the resolver
output outside the store at first, it must still provide a path to source-ref
resolution and fixture comparison.

## Entity Identity Rules

Entity ids should follow the fixture convention:

```text
{kind}:{name}
{kind}:{name}@{variant}
```

Minimum supported kinds:

- `service`;
- `route`;
- `instance`;
- `pod`;
- `db`;
- `queue`;
- `cache`;
- `external-api`;
- `tenant`;
- `infra`;
- `deployment`;
- `host`;
- `container`;
- `shard`.

The first slice does not need perfect semantic-convention coverage. It does need
deterministic rules for the current fixtures.

### Service Identity

Resource `service.name` maps to `service:<name>` when it is the only meaningful
identity dimension.

If the same `service.name` has strong discriminators that change the
investigation unit, the resolver should produce variant identities:

- `rollout=canary` maps to `service:<name>@canary`;
- sibling non-canary instances of the same service may map to
  `service:<name>@stable` when fixture evidence clearly models a stable fleet;
- explicit version or deployment attributes may be used as discriminators, but
  should not create noisy variants when there is no ambiguity.

The `ambiguous-entity-resolution` fixture is the required test case. The
resolver must keep `service:payments@canary`,
`service:payments@stable`, and `service:payments@unresolved` separate. A
single `service:payments` aggregate must not replace them.

### Instance And Runtime Identity

Resource or span `service.instance.id` maps to `instance:<id>`. Pod, host,
container, and deployment attributes should produce corresponding runtime
entities when present:

- `k8s.pod.name` -> `pod:<name>`;
- `host.name` -> `host:<name>`;
- container names or ids -> `container:<name-or-id>`;
- deployment-like names -> `deployment:<name>`.

Runtime entities can be relationship endpoints even when a fixture does not list
them as top-level expected entities. The comparison harness should distinguish
missing required gold entities from acceptable extra derived entities.

### Route Identity

HTTP route attributes should map to:

```text
route:<service>/<method> <route>
```

When method is missing, use the best stable route key available and lower the
confidence. Do not derive route identities from raw high-cardinality URLs unless
the fixture already treats the value as a route.

### Dependency Identity

Database, queue, cache, and external API identities should come from resource or
span attributes when present:

- database names or systems -> `db:<name>`;
- queue names -> `queue:<name>`;
- cache or redis-like resources -> `cache:<name>` or `infra:<name>` when the
  fixture already uses `infra`;
- external service or provider names -> `external-api:<name>`.

When a span only has an operation name such as `stripe.charge`, use a
conservative external-api identity only if the fixture or attributes clearly
name the dependency. Otherwise record the relationship as unresolved or low
confidence.

### Tenant, Shard, And Other Logical Entities

Metrics, logs, spans, or changes may carry already-normalized fixture entity ids
such as `tenant:acme` or `shard:orders-shard-3`. The resolver may accept those
ids as source hints, but should still attach provenance and confidence rather
than treating all strings as equally certain.

## Confidence Model

Confidence should be simple, deterministic, and explainable. It does not need a
learned model.

Suggested bands:

- `0.95` to `1.0`: direct resource identity with strong discriminators;
- `0.85` to `0.94`: direct telemetry fields with one minor missing attribute;
- `0.60` to `0.84`: inferred from span, metric, log, or change hints;
- `0.30` to `0.59`: unresolved or ambiguous identity with plausible
  alternatives;
- below `0.30`: too weak to promote unless needed as explicit missing data.

Confidence must not be reused as causal confidence. It answers "how sure are we
that this record maps to this entity or relationship?", not "is this the root
cause?".

The resolver should preserve supporting reasons in fields that reviewers and
future code can inspect:

- `discriminators`;
- `alternatives`;
- `missing_attributes`;
- relationship `evidence`;
- optional `attributes.reason` values when useful.

## Ambiguity And Missing Data

Ambiguity is a first-class output, not an implementation failure.

When records share a weak key such as `service.name` but differ by
`service.version`, `service.instance.id`, rollout, pod, or deployment, the
resolver should:

1. create separate high-confidence identities for the distinguishable groups;
2. attach alternatives linking sibling identities;
3. create an unresolved bucket when records lack the discriminating attributes;
4. populate `missing_attributes`;
5. estimate the unresolved share when there is enough local evidence.

The resolver must not silently merge canary and stable traffic into one service
identity. That merge is a false-causality risk because it can hide a failing
variant behind a healthy aggregate.

## Relationship Derivation

Minimum relationship types:

- `calls`;
- `reads-from`;
- `writes-to`;
- `depends-on`;
- `runs-on`;
- `deployed-as`;
- `emits`;
- `retries`;
- `fans-out-to`;
- `shares-resource-with`.

The first implementation should prioritize:

- `calls` from trace parent-child service boundaries;
- `reads-from` and `writes-to` from client spans or database attributes;
- `deployed-as` from service-to-instance or service-to-pod mappings;
- `runs-on` from service-to-host, pod, or container attributes when present;
- `depends-on` when the dependency kind is clear but the more specific edge is
  not.

Relationship evidence should use existing scalar ref conventions accepted by
the fixture validator, for example:

```text
trace:t-0001
trace:t-0001/s-3
http.server.error_rate@service:checkout
log-1
change:deploy-checkout-v2
```

The exact scalar formatting can follow current helper APIs, but every evidence
ref must resolve through the same fixture/hot-store reference rules used
elsewhere.

Relationship confidence should combine:

- endpoint entity confidence;
- evidence strength, such as direct trace parent-child observation;
- relationship specificity;
- missing endpoint attributes;
- ambiguity of the source records.

Do not infer ownership or causal direction from time correlation alone in this
topic. Ownership and root-cause ranking belong later.

## Fixture Comparison Contract

The topic should add a comparison helper, for example:

```rust
compare_entity_context(case, derived) -> EntityContextComparison
```

The comparison should report:

- missing expected entities;
- missing expected relationships;
- kind mismatches;
- confidence below expected tolerance;
- missing discriminators;
- missing alternatives;
- missing unresolved or missing-attribute markers;
- unresolved extra entities and extra relationships.

For the first implementation, gold fixture records are the required subset. The
resolver may produce extra lower-level entities, such as instances, as long as
they are deterministic, source-backed, and do not contradict the gold output.

Recommended tolerances:

- exact id and kind match for expected entities;
- exact `src`, `type`, and `dst` match for expected relationships;
- confidence may differ by a small tolerance, such as `0.05`;
- alternatives and missing attributes should match by id/name, not array order;
- relationship evidence should include at least one resolvable supporting ref
  when the gold relationship has evidence.

The ambiguous fixture should have stricter checks:

- `service:payments@canary` exists;
- `service:payments@stable` exists;
- `service:payments@unresolved` exists;
- canary and stable list each other as alternatives;
- unresolved lists missing `service.version` and `service.instance.id`;
- aggregate `service:payments` does not replace the three required identities.

## Store And Query Integration

Derived entity and relationship records should be available to the hot store's
source-ref resolver as `entity` and `relationship` categories. This matters
because later evidence items will cite entities and dependency edges.

If this topic adds store insertion APIs for derived records, keep them narrow:

```rust
store.insert_record(StoredRecord {
    kind: StoredRecordKind::Entity,
    key: SourceKey::new(entity.id.clone()),
    entities: vec![entity.id.clone()],
    payload: serde_json::to_value(entity)?,
    time_window: None,
})
```

Relationship records should include both endpoints in their store `entities`
field so entity selectors can find them.

This topic may add a query or inspection helper for tests, but it should not
change `get_evidence_bundle` from fixture-backed output into generated evidence.

## CLI

A small CLI is useful but not mandatory if tests cover the full behavior. If
added, prefer:

```bash
cargo run --bin derive_entities -- --fixture ambiguous-entity-resolution
cargo run --bin derive_entities -- --fixture deploy-bad-rollout --json
```

Minimum useful output:

- derived entity count;
- derived relationship count;
- missing expected entity count;
- missing expected relationship count;
- unresolved entity count;
- lowest confidence mappings.

The CLI should be an inspection tool, not a new product surface.

## Suggested Implementation Slices After Design Approval

No slice should start until reviewers agree on the design direction, or
explicitly approve that slice in their `Direction Verdict`.

Recommended slices:

1. Data model and store read boundary: define entity and relationship output
   types, expose the narrow store records needed by the resolver, and add
   serialization tests.
2. Entity resolver: derive resource, service, route, instance, runtime,
   dependency, tenant, shard, and unresolved entities with confidence and
   alternatives.
3. Relationship builder: derive trace and runtime relationships with source
   evidence and confidence.
4. Fixture comparison and tests: compare derived output against all fixture gold
   entities and relationships, with stricter assertions for
   `ambiguous-entity-resolution`.
5. Optional CLI/reporting: add a small inspection command only after tests pin
   the contract.

## Tests

Add tests that prove:

- every fixture with `entity-resolution` can derive its required gold entities;
- every fixture with `relationship-building` can derive its required gold
  relationships or report a reviewed unsupported case;
- expected entity ids, kinds, confidence bands, discriminators, alternatives,
  unresolved markers, and missing attributes are compared;
- the ambiguous payments fixture does not collapse canary, stable, and
  unresolved telemetry into `service:payments`;
- relationship evidence refs resolve;
- entity and relationship records can be inserted into or exposed through the
  hot-store derived-record boundary;
- OTLP JSON sample records can participate in the same resolver path at least as
  a smoke test;
- no existing fixture validation or source-ref validation regresses.

Existing verification should continue to pass:

```bash
cargo fmt
cargo test
cargo clippy --all-targets --all-features
cargo run --bin validate_fixtures
```

## Definition Of Done

This topic is complete when:

- `HotContextStore` records can be resolved into source-backed operational
  entities;
- entity output includes confidence, discriminators, alternatives, unresolved
  states, and missing attributes where applicable;
- relationship output includes endpoints, type, confidence, and evidence refs;
- all current fixture gold `entities` and `relationships` are compared by an
  automated test or reviewed report;
- `ambiguous-entity-resolution` proves that canary, stable, and unresolved
  payments identities remain separate;
- derived entity and relationship records are usable by the hot-store reference
  boundary or have an explicitly reviewed path to become usable;
- no anomaly, log-pattern, timeline, evidence-ranking, MCP, persistence, or new
  ingest protocol work is introduced;
- `cargo fmt`, `cargo test`, `cargo clippy --all-targets --all-features`, and
  `cargo run --bin validate_fixtures` pass.

## Review Focus

Reviewers should focus on:

1. Whether returning to `entity-resolver-confidence` is now correct after the
   simulator and OTLP JSON ingest topics.
2. Whether the entity identity rules are deterministic without being too
   fixture-specific.
3. Whether confidence, alternatives, unresolved states, and missing attributes
   are concrete enough to prevent silent false causality.
4. Whether the fixture comparison contract is strict enough while allowing
   useful extra low-level entities.
5. Whether relationship evidence and confidence are source-backed instead of
   inferred from weak correlation.
6. Whether the topic stays limited to Milestone 5A and does not absorb anomaly,
   timeline, evidence compiler, MCP, persistence, or more ingest work.
