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
