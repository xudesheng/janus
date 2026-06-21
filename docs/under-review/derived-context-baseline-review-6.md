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

## Review (by Claude)

### Direction Verdict

**Continue.** Milestone progress first: Slice 5 lands related-anomaly derivation
for every `find_related_anomalies` fixture, and it is the **best-generalized slice
in the topic so far** — exactly the direction I pushed for in review 5. I
reproduced verification against baseline `9d622b6`: 17 `derived_context` tests
pass, full suite green, clippy clean. `derive_related_context` runs through the
real Milestone 5A path (`resolve_entities` + `resolve_relationships`) plus raw
prior incidents, with no `case.expected` input (integrity invariant holds). The
Verification section is truthful, and the two formal-doc edits match their claims
(timeline sort text reconciled; Slice 6 now formally requires the
timeline-generalization decision).

What makes this slice notably better than the timeline slice:
`derive_downstream_dependency_related` (`:1527`) walks the actual relationship
graph — `src == seed.entity` with `ReadsFrom`/`DependsOn`, relating anomaly windows
on the `dst` entity — and *derives* its note from the edge
(`related_dependency_note`, `:1632`) instead of looking up gold prose by entity
name. The retry path keys off the `Retries` edge, lag is computed, and recurring
incidents match by a named signature. This is relationship-driven derivation, not
per-fixture scripting. It also matters for the Slice 6 decision below: it
**demonstrates that role/marker derivation from the 5A relationship + onset
context is feasible**, which strengthens the case for generalizing the timeline
rather than freezing it as scaffolding.

Next action: **continue to Slice 6 (final integration)**, the last slice, which
carries three things: store integration, the carried timeline-generalization
decision, and the relationship-ref resolution question raised below.

### Findings

No new significant code defects — this slice is clean. Three items for Slice 6,
none blocking:

**1 (open question for Slice 6, answers your Focus Q3) — relationship refs do not
resolve as store scalar refs.** You disclosed this honestly: expected relationship
records have no ids, so the fixture loader's scalar-ref index skips them, and
related-anomaly `source_refs` carry relationship keys that do not resolve through
the hot store. The design's Store Integration and Provenance Contract want derived
refs to resolve "through existing reference rules." So **yes** — Slice 6 should
insert derived relationship records (or assign them resolvable keys) before
related-anomaly records, so relationship-backed provenance resolves like anomaly
and log-pattern refs already do. This is the right place to fix it.

**2 (acceptable interim, record it) — seed selection is failure-class-keyed.**
`related_anomaly_seed` (`:1486`) branches on `failure_class` with fixture-specific
signal/entity expectations. This is the one part of the slice that will not
generalize, but it is **defensible as interim**: `find_related_anomalies` has no
caller-provided seed/query surface in this topic (API surfaces are out of scope),
and the design models the seed as an *input* ("select a seed anomaly by id"). When
the agent surface lands (M7), the seed must become caller-driven; record that as a
known boundary so it is not mistaken for derivation logic.

**3 (acceptable) — `RECURRING_SIGNATURE_EXACT_MATCH_SIMILARITY` and the signature
match.** The named constant and the content-keyed `recurring_signature_matches`
(entity + `db.locks.waiting` + VACUUM trigger) are corpus-coupled but deterministic
and named — the same bar the baseline blend met in review 4. Acceptable for the
first corpus slice. The hardcoded retry/recurring notes are fine because
related-anomaly notes are correctly routed to the nonfatal
`related_anomaly_note_differences` channel (verified).

### Answers To Reviewer Focus

1. **Sufficiently relationship-driven — yes**, for the core derivation; this is the
   strongest generalization in the topic. The remaining coupling is seed selection
   (item 2), not the relation logic.
2. **Yes** — failure-class seed selection is acceptable until a caller-provided
   `find_related_anomalies` seed/query surface exists (M7). Record it as interim.
3. **Yes** — make relationship refs resolvable in Slice 6 by inserting derived
   relationship records before related-anomaly records (item 1).
4. **Yes** — the named recurring-signature similarity constant is acceptable.
5. **My recommendation: generalize the timeline at Slice 6, do not freeze it as
   scaffolding.** This slice proves relationship/onset-driven derivation works, so
   "scaffolding forever" is now the weaker option. Concretely: derive
   symptom/propagation/amplification from onset ordering + dependency direction
   (the inputs Slice 5 already uses), keeping per-fixture *event selection* only
   where genuinely unavoidable. If full generalization proves too large for one
   round, the formal doc already requires the decision to be explicit — but make it
   a tracked follow-up, not a silent permanent state.
6. **No review-4 item should block Slice 6** — they stay lower priority than final
   integration. Keep the retry-storm id and `new-since-incident` items on a tracked
   post-milestone list so they are not lost.

### Round Termination

This round leaves actionable feedback (the Slice-6 relationship-ref resolution and
the carried timeline-generalization decision) and the milestone is incomplete
(final integration remains), so a `review-7` round is justified. Slice 6 is the
last slice: it should reach the Definition Of Done — full-corpus comparison, store
integration with resolvable derived refs, and a recorded resolution of the timeline
generalization decision. If Slice 6 closes those with no open findings, the topic
completes and no further round should be opened.
