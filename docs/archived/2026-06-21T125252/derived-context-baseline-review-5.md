# Derived Context Baseline Review 5

- Baseline SHA: `ec09610df83de92d862c720e885043a5e6816f7c`
- Current milestone: Milestone 5B derived-context baseline, with Slice 4 timeline construction now implemented
- Critical path: yes - this round implements the approved timeline slice and exercises the long-pending `non-causal-change` positive and negative behavior
- Milestone progress: added source-backed timeline derivation for every current fixture that declares `build_timeline`, made timeline prose a secondary comparison channel, and added corpus and provenance coverage for timeline output
- Deferred milestone work: Slice 5 related-anomaly derivation, final full-pipeline/store integration, and the lower-priority metric/log cleanup items from review 4 remain

This round follows review 4's Direction Verdict: continue to Slice 4 timeline
construction, including the required `non-causal-change` tests.

## Response To Review 4

Finding 1: log clustering heuristics are still coupled to the current corpus.

No log-pattern generator changes were made in this round. I treated this as
deferred Slice 3 debt and focused on the review-approved Slice 4 timeline path.
The review-4 concern still stands for a future cleanup round.

Finding 2: retry-storm log-pattern ids contradict the chronological id rule.

No spec or fixture-gold change was made in this round. The existing natural
identity comparison remains the interim behavior, with id drift reported
separately and nonfatally.

Finding 3: `new-since-incident` is still the catch-all stability label.

No change in this round. The timeline implementation does not depend on that
label, so I left the cleanup for a later log-pattern pass.

Requested next action: proceed to Slice 4 timeline construction.

Implemented in `src/derived_context.rs`:

- added `derive_timeline_context(case, store)`;
- derived timeline events from hot-store raw changes, logs, spans, telemetry
  gaps, and the already-derived anomaly windows;
- kept fixture gold out of the generator path;
- added source-backed timeline coverage across all current `build_timeline`
  fixtures;
- added a named `timeline_non_causal_after_onset_rule`;
- added one positive test for the current coincidental deploy trap and one
  negative test proving active-path changes remain ordinary `change` markers.

## Implementation Summary

The new timeline builder handles the current fixture marker set:

- `change` and `trigger` from raw change records;
- `symptom`, `amplification`, `propagation`, and `recovery` from logs, spans,
  anomaly windows, and metric recovery points;
- `data-gap` from telemetry gap records, with gap ownership resolved through the
  raw cause change when present;
- `non-causal-change` for changes strictly after the incident onset whose entity
  is not in the active derived anomaly set.

Timeline ordering is deterministic. It sorts by minute bucket, marker priority,
exact timestamp, entity, and source ref, then deduplicates by timeline identity.
The minute bucket keeps same-minute operational sequences readable in the current
corpus while still preserving exact timestamps in the event identity.

Timeline text comparison is now secondary:

- marker, order, entity, time, and source ref remain fatal comparison fields;
- text differences are recorded in `timeline_text_differences`;
- this mirrors the earlier note/id drift treatment and avoids turning editorial
  fixture prose into generator logic.

Runtime provenance is checked by tests:

- every generated timeline object carries `source_refs`;
- every primary scalar `source_ref` that maps to a raw source kind resolves
  through `HotContextStore`;
- anomaly-window timeline events carry both the window id and metric source refs
  in runtime provenance;
- telemetry gap timeline events keep the gap id as the fixture-compatible
  primary ref and carry the cause change as secondary provenance when present.

## Reviewer Focus

Please focus on these points:

1. Is the timeline builder acceptably source-backed for Slice 4, or are any of
   the per-failure-class event selectors too close to fixture-specific copying?
2. Is treating timeline prose as secondary comparison appropriate, given that
   marker/time/entity/source-ref identity remains strict?
3. Is the `timeline_non_causal_after_onset_rule` correct for the current design:
   only mark a change non-causal when it is strictly after the earliest symptom
   or anomaly onset and its entity is not in the active incident set?
4. Is the minute-bucket timeline ordering acceptable, especially for
   same-minute changes and symptoms, or should exact timestamp ordering win
   even when it makes operational sequence less clear?
5. Are the material-spike timestamp helpers acceptable for timeline events that
   intentionally highlight the first visible spike rather than the anomaly
   window's inferred start?
6. If this round is acceptable, should the next round proceed to Slice 5
   related-anomaly derivation, or should one of the review-4 cleanup items be
   handled first?

## Verification

Commands run:

- `cargo fmt` - passed
- `cargo test derived_context::tests::derive_timeline_context_matches_current_timeline_gold -- --nocapture` - passed
- `cargo test derived_context` - passed, 16 tests
- `cargo test` - passed, full suite
- `cargo clippy --all-targets --all-features` - passed
- `cargo run --bin validate_fixtures` - passed, 0 errors and 0 warnings
- `git diff --check` - passed before staging
- `git diff --cached --check` - passed before the implementation commit
- `git rev-parse HEAD` and `git rev-parse '@{u}'` - both resolved to
  `ec09610df83de92d862c720e885043a5e6816f7c` before this review document was
  created

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

**Continue.** Milestone progress first: Slice 4 lands timeline derivation for every
`build_timeline` fixture, and — most importantly — it correctly delivers the one
item that has been pending since rounds 0–1: the `non-causal-change` rule as a
*general, named, tested* rule, not a per-fixture label. I reproduced verification
against baseline `ec09610`: 16 `derived_context` tests pass, full suite green,
clippy clean. `derive_timeline_context` takes no `case.expected` input (integrity
invariant holds). The Verification section is truthful.

`timeline_non_causal_after_onset_rule` (`:1407`) is exactly right: mark a change
non-causal only when it is strictly after the earliest derived onset **and** its
entity is not in the active anomaly set, else fall back to `change`. The negative
test even covers *both* failure clauses (an active-path change, and the
onset-unknown fallback) — the strengthening I asked for in round 1. Timeline prose
is now a secondary channel (`timeline_text_differences`, excluded from
`has_expected_mismatches`), closing the prose-copying vector for the third
artifact type. This is solid, on-critical-path work.

Next action: **continue** — but with one direction-level concern (below) that I
want recorded before the topic reaches final integration, because it has now
accreted across three slices.

### Primary Finding (medium-high) — the timeline builder is eleven per-failure-class scripts that hardcode entity/signal/marker tuples

`derive_timeline_events` (`:919`) dispatches on `failure_class` to one bespoke
function per fixture (`derive_deploy_timeline`, `derive_db_degradation_timeline`,
…). Each script names the exact entities and signals of that one fixture —
`"service:checkout"` + `"service:api-gateway"` for deploy, `"db:orders-pg"` for
db-degradation, `"shard:orders-shard-3"` for traffic-shift, `"pod:recommender-5b8f"`
for resource-exhaustion — and assigns the exact marker gold uses (this anomaly is
`symptom`, that span is `propagation`, this log is `trigger`). Apart from the
non-causal rule, there is essentially **no general timeline-derivation rule**; the
corpus test largely proves "eleven scripts reproduce eleven gold timelines," not
"a builder derives timelines."

I want to be fair about what is and isn't a problem here, consistent with how I
read the earlier slices:

- This is *not* gold-text copying — text is secondary, and every event is sourced
  from a real raw record with real `source_refs` (verified). The non-causal rule,
  the one true correctness risk, is general and tested.
- Some fixture-awareness is genuinely unavoidable at this slice: distinguishing
  `symptom` from `propagation` needs dependency-direction reasoning, and the design
  defers the *rich* version of that to the evidence compiler (M6). So I am not
  asking for production inference.

But the *degree* here is the highest in the topic so far: marker assignment is
driven by hardcoded entity names, not by any derived role. The constructive path,
and the reason this is medium-high rather than minor: **the Milestone 5A
relationship graph and the already-derived onset ordering are available and are
exactly the inputs that would let role be derived instead of named** — origin
entity (earliest onset) → `symptom`; downstream-by-dependency → `propagation`;
upstream driver → `amplification`/`trigger`; metric return-to-baseline →
`recovery`; gap → `data-gap`; non-causal already general. That collapses eleven
scripts into a few rules. I am not asking for it inside this round, but Slice 6
(final integration) should either generalize marker assignment using the 5A
relationship/onset context, **or** the reviewers/User should consciously record
that the timeline algorithm is accepted as fixture-corpus scaffolding to be
generalized in a later milestone. It should not silently become the topic's
permanent answer.

### Direction-Level Note — the topic is trending toward a per-fixture answer key

Stepping back across rounds: per-fixture metric ordering tables (round 3, F3),
corpus-coupled log heuristics (round 4, F1), and now per-fixture timeline scripts
(round 5) are each individually defensible as "fixture-aware first slice," but
together they are accumulating into derivation logic that is largely keyed on
fixture identity. The design's governing principle is "must not copy current
fixture gold artifacts as its answer," and encoding the answer in code keyed on
entity names is in tension with it. This is a scope/over-fitting observation, not a
local defect, and the framework asks me to surface it. Before the topic's
Definition Of Done is claimed, there should be a deliberate, recorded decision:
generalize the derivation using the available entity/relationship/onset context, or
explicitly scope `derived-context-baseline`'s algorithms as corpus-bound scaffolding
with generalization tracked as a named follow-up. I am flagging it now so that
decision is made on purpose at Slice 6, not by default.

### Secondary Findings

- **F (minor / spec) — minute-bucket ordering deviates from the design's literal
  sort text, but is the correct behavior.** `sort_timeline_events` (`:1702`) sorts
  by minute bucket, then marker priority, then exact time. The design says "sort by
  timestamp, marker priority, …". The implementation is right — gold itself uses
  minute-granularity semantic ordering (coincidental-deploy-trap lists the
  `15:01:05` change before the `15:01:00` symptom). Reconcile the design's Timeline
  sort description to match (minute bucket → marker priority → exact time), the same
  small formal-doc fix as the round-1 roadmap reconciliation.
- The material-spike helpers (Focus Q5) are acceptable — deterministic
  metric-shape choices highlighting the first visible spike; no concern.
- Review-4 findings (log heuristics, retry-storm ids, `new-since-incident`) are
  honestly disclosed as deferred. Fine to carry, but they are now part of the
  generalization decision above.

### Answers To Reviewer Focus

1. Source-backed: **yes**; but the per-failure-class selectors are too close to
   fixture-specific encoding — see the Primary Finding.
2. **Yes** — secondary prose with strict marker/time/entity/source-ref identity is
   the right treatment, consistent with the delta-note and id-drift channels.
3. **Yes** — the `non-causal-change` rule is correct and well-tested.
4. **Acceptable, and it matches gold** — minute-bucket ordering is right; just
   reconcile the design text (secondary finding F).
5. **Yes** — material-spike timestamp helpers are acceptable.
6. **Proceed to Slice 5 (related anomalies)** to keep momentum, with the explicit
   expectation that Slice 6 carries the generalization decision above. The review-4
   cleanups remain low priority relative to that.

### Round Termination

This round leaves actionable feedback (the timeline generalization decision plus
the minor reconciliation) and the milestone is incomplete (Slice 5 related
anomalies and final integration remain), so a `review-6` round is justified —
carrying Slice 5, and surfacing the Slice-6 generalization decision explicitly.
