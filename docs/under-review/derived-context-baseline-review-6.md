# Derived Context Baseline Review 6

- Baseline SHA: `9d622b6e23f388333a9312bb5237a94156006c96`
- Current milestone: Milestone 5B derived-context baseline, with Slice 5 related-anomaly derivation now implemented
- Critical path: yes - this round implements the approved related-anomaly slice and reconciles the formal timeline ordering text called out in review 5
- Milestone progress: added `derive_related_context`, source-backed related-anomaly derivation for every current `find_related_anomalies` fixture, corpus/provenance coverage, and a design-doc note that Slice 6 must make the timeline-generalization decision explicit
- Deferred milestone work: final full-pipeline/store integration remains; review-4 log cleanup items remain deferred; timeline marker generalization is intentionally carried to Slice 6 per review 5

This round follows review 5's Direction Verdict: continue to Slice 5 related
anomalies, while recording the Slice 6 timeline-generalization decision.

## Response To Review 5

Primary finding: timeline construction is still too close to per-failure-class
scripts.

Not fixed in code this round. Review 5 explicitly directed the next action to
Slice 5 and asked that the generalization decision be carried into Slice 6. I
updated `docs/core/derived-context-baseline.md` so Slice 6 must either
generalize timeline marker assignment through entity/relationship/onset context
or explicitly record the current timeline algorithm as fixture-corpus
scaffolding to be generalized after this milestone.

Direction-level note: the topic is trending toward corpus-bound derivation.

Acknowledged and carried forward. The new related-anomaly slice is relationship
driven where the current corpus supports it, but seed selection still uses
fixture failure class because this milestone has no caller-provided
`find_related_anomalies` seed API yet. That tradeoff is called out below for
reviewer attention.

Secondary finding: minute-bucket timeline ordering deviated from the literal
design text.

Fixed in `docs/core/derived-context-baseline.md`: timeline sorting now states
minute bucket, marker priority, exact timestamp, entity id, and source ref.

Review-4 cleanup items remain deferred:

- log clustering corpus coupling;
- retry-storm log-pattern id/spec cleanup;
- `new-since-incident` catch-all stability label.

## Implementation Summary

Added `derive_related_context(case, store)` in `src/derived_context.rs`.

The generator:

- derives anomaly windows from canonical metric series;
- derives entities and relationships through the existing Milestone 5A
  `resolve_entities` and `resolve_relationships` path;
- canonicalizes raw prior-incident records from `HotContextStore`;
- runs only for fixtures declaring `find_related_anomalies`;
- does not read expected related-anomaly gold as generator input.

Implemented related-anomaly rules:

- downstream dependency: for service anomalies with `reads-from` or
  `depends-on` edges, relate windows on the downstream dependency and compute
  absolute lag in seconds;
- retry storm: use a `retries` relationship to relate caller retry-rate
  amplification and victim request-rate load amplification to the victim error
  window;
- recurring incident: match prior incidents by primary entity, metric signature,
  and VACUUM trigger pattern, with the similarity constant named as
  `RECURRING_SIGNATURE_EXACT_MATCH_SIMILARITY`.

Runtime provenance:

- `DerivedRelatedAnomalies` carries aggregate `source_refs`;
- each `RelatedAnomaly` carries seed anomaly refs, related window refs, metric
  refs, relationship key/evidence refs, or prior-incident refs as applicable;
- tests verify non-empty runtime provenance and scalar ref resolution for refs
  the current hot-store fixture loader can resolve.

One boundary remains visible: expected fixture relationship records do not have
ids and are skipped by the fixture loader's scalar-ref index, so relationship
keys are carried as internal provenance but not asserted as fixture-store scalar
refs in this slice. Slice 6 store integration should decide whether
relationship-backed related anomalies require explicitly inserted derived
relationship records before related-anomaly records.

## Reviewer Focus

Please focus on these points:

1. Are the related-anomaly rules sufficiently relationship-driven for Slice 5,
   or are they still too close to fixture-class scripting?
2. Is failure-class-based seed selection acceptable until Janus has a real
   caller-provided `find_related_anomalies` seed/query surface?
3. Should relationship keys in related-anomaly `source_refs` become resolvable
   by inserting derived relationship records before related-anomaly records in
   Slice 6?
4. Is the named recurring-signature similarity constant acceptable for this
   first corpus slice?
5. For Slice 6, should the implementor generalize timeline marker assignment
   now, or explicitly scope the current timeline builder as corpus scaffolding
   and finish store integration first?
6. Should any review-4 cleanup item block Slice 6, or continue to remain lower
   priority than final integration?

## Verification

Commands run:

- `cargo fmt` - passed
- `cargo test derived_context::tests::derive_related_context_matches_current_related_gold -- --nocapture` - passed
- `cargo test derived_context` - passed, 17 tests
- `cargo test` - passed, full suite
- `cargo clippy --all-targets --all-features` - passed
- `cargo run --bin validate_fixtures` - passed, 0 errors and 0 warnings
- `git diff --check` - passed before staging
- `git diff --cached --check` - passed before the implementation commit
- `git rev-parse HEAD` and `git rev-parse '@{u}'` - both resolved to
  `9d622b6e23f388333a9312bb5237a94156006c96` before this review document was
  created

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
