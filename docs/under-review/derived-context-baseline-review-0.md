# Derived Context Baseline Review 0

- Baseline SHA: `063c0b2a351f1718c5f80e34d53ec81d44c351e3`
- Current milestone: review-approved Milestone 5B derived-context design in `docs/core/derived-context-baseline.md`
- Critical path: yes - the Milestone 5B implementation is blocked until reviewers agree on this design direction
- Milestone progress: tightened the design gate and provenance contract, then submitted the design for reviewer direction before any Rust implementation
- Deferred milestone work: Rust implementation of anomaly windows, log patterns, timelines, related anomalies, window comparisons, and fixture comparison remains deferred because implementation must not start until all active reviewers approve the design direction

This is a design-only first review round for the `derived-context-baseline`
topic. No Rust implementation has started.

The design draft is `docs/core/derived-context-baseline.md`. This round also
added two narrow clarifications to that formal design document before review:

- the first review round must decide whole-topic approval versus phase-by-phase
  approval, `non-causal-change` timeline scope, provenance strictness, and
  timeline payload shape;
- every derived runtime object needs inspectable provenance even when the
  current fixture gold JSON shape does not expose a `source_refs` field.

Reviewers should focus on these decisions:

1. Is `derived-context-baseline` the right next topic after completed simulator,
   OTel JSON ingest, and Milestone 5A entity/relationship context work?
2. Should Milestone 5B be approved as one implementation topic, or should
   approval be phase-by-phase? If phase-by-phase, the `Direction Verdict` must
   name the approved phase.
3. Are anomaly windows, log patterns, timelines, related anomalies, and window
   comparisons the right boundary for this topic, separate from evidence
   ranking and suspected-cause scoring?
4. Is `non-causal-change` acceptable as a Milestone 5B timeline marker under the
   proposed narrow rules, or should nearby-change classification wait entirely
   for the evidence compiler?
5. Is the provenance contract strong enough for future Evidence IR generation,
   especially for derived objects whose fixture gold shape lacks explicit
   `source_refs`?
6. Is the fixture comparison contract strict enough to prevent generated
   artifacts from drifting while still allowing deterministic, source-backed
   extras?
7. Is stable natural-language timeline text acceptable for the current corpus,
   or should implementation wait for a structured timeline payload?
8. Should the roadmap's near-term review order be corrected to include this
   Milestone 5B topic before `evidence-compiler-ranking`, or is the current
   branch-specific topic selection sufficient?

## Verification

No code verification this round. This was a design-document and review-document
submission only.

Checked repository state:

- branch: `derived-context-baseline`
- upstream: `origin/derived-context-baseline`
- baseline commit is pushed: local `HEAD` and upstream both resolve to
  `063c0b2a351f1718c5f80e34d53ec81d44c351e3`

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

**AGREE — continue to implementation** (subject to the design gate: every other
active reviewer must also agree before Rust starts).

- On critical path: **yes**. Milestone 5B (`docs/core/roadmap.md`) names
  `derived-context-baseline` as its review topic, and 5A
  (`entity-resolver-confidence`) is complete. A reviewer-approved 5B design is
  the concrete thing that unblocks the implementation slices.
- Moves the milestone: **yes**. This round secures the design alignment that the
  topic's own Design Review Gate requires before any code.
- Next action: **continue**. I approve the topic **as one implementation topic**,
  with the six slices in "Suggested Implementation Slices" used as internal
  commit/checkpoint boundaries rather than as separate review rounds. I do not
  require per-slice review rounds. One soft sequencing ask is in Finding 2.

The boundary is right. The design correctly stops at *deriving* anomaly windows,
log patterns, timelines, related anomalies, and window comparisons, and keeps
final evidence generation, suspected-cause ranking, and causal scoring out of
scope (deferred to Milestone 6's evidence compiler). The "derive facts" vs "rank
evidence" split the design uses to justify coming before
`evidence-compiler-ranking` is sound: pushing straight to the compiler would make
it work from raw signals and blur that boundary.

I verified the design's claims against the current tree (baseline
`063c0b2`) rather than taking them on faith:

- `StoredRecordKind` (`src/hot_context_store.rs:19`) already carries
  `AnomalyWindow`, `LogPattern`, `TimelineEvent`, `RelatedAnomaly`, and
  `WindowComparison`, so the store-integration target needs no new enum surface.
- `SourceSignal` (`src/evidence.rs:108`) has `AnomalyWindow` and `LogPattern`
  but **not** `TimelineEvent`/`RelatedAnomaly`/`WindowComparison` — exactly as the
  design states. The "inspect timeline/related/comparison through store records
  or comparison helpers" plan is therefore the right call; do not invent new
  `SourceSignal` variants for them in this topic.
- `raw_source_records()` (`src/hot_context_store.rs:375`) and
  `load_fixture_case()` (`:234`) exist, so the "derived records must not become
  raw resolver inputs" invariant is enforceable against real code.
- Capability coverage at the scenario level: `anomaly-windows` 12/12,
  `build_timeline` 11/12, `log-pattern-clustering` 7/12, `compare_windows` 5/12,
  `find_related_anomalies` 4/12. The design's "all / most / some / a smaller set"
  prose is accurate. The capability-tag-driven comparison (not "every artifact
  from every fixture") is the correct test shape.

### Decisions On The Open Questions

These answer the four decisions the topic's Design Review Gate asks the first
round to resolve, plus the review doc's framing questions.

1. **Whole-topic vs phase-by-phase:** approve as **one whole implementation
   topic** (slices as internal checkpoints). See Direction Verdict.
2. **`non-causal-change` in the 5B timeline:** **keep it in 5B**, narrowly gated
   — see Finding 1. Deferring it entirely is the wrong call here; one gold
   fixture requires it and it is a derivable temporal fact, not a causal ranking.
3. **Provenance contract strength:** **strong enough.** The requirement that a
   derived runtime object carry inspectable provenance even when the gold JSON
   lacks `source_refs` is the right bar and matches roadmap principle 3. One
   small note in Finding 3.
4. **Stable natural-language timeline text:** **acceptable for the current
   corpus**, with the brittleness guard the design already names. See Finding 4.
5. **Fixture comparison contract:** **strict enough.** "Gold is the required
   subset; extras allowed only when deterministic, source-backed,
   non-contradicting, and made visible" is a good anti-drift stance.

### Findings

**Finding 1 (direction-level, must resolve before coding) — pin the
`non-causal-change` rule and add an anti-over-labeling test.**

`non-causal-change` occurs exactly once in gold:
`scenarios/coincidental-deploy-trap/expected.json` —
`{ "t": "15:03:10Z", "marker": "non-causal-change", "entity": "service:search-ui", "text": "search-ui frontend deploy (coincidental, after onset)", "source_ref": "change:deploy-search-ui" }`.
The deploy lands after the derived onset (cache 15:01, search-api 15:02), and the
causal verdict (rank 2, score 0.05, `trap_note`) lives in `suspected_causes`,
which is out of 5B scope.

This is the one place the design brushes against `roadmap.md`, which says 5B
should land "without final causal or non-causal classification" and that nearby
changes are classified "not by the Milestone 5B timeline builder alone." I read
those as guarding against 5B pre-empting the compiler's *ranking*, not against
emitting a source-backed temporal marker. Since the gold corpus requires the
marker for the `build_timeline` capability test, dropping it would make that
fixture's timeline unmatchable — a worse outcome than a tightly-scoped rule.

To keep it on the right side of the line, before coding the timeline builder, fix
in the next round's formal design:

- a **named, deterministic rule** for when `non-causal-change` may be emitted —
  e.g. the change timestamp is strictly after the derived symptom onset of the
  active incident **and** the change's entity is not on the derived
  anomaly/propagation path — with the rule named in code and tests (same standard
  the design already imposes on anomaly thresholds);
- a **negative test**: the builder must **not** emit `non-causal-change` for a
  change that fails the rule (e.g. a change before/at onset, or on the failing
  path). The marker must never be the builder's guess; it is a temporal fact.

**Finding 2 (sequencing, soft) — land the comparison shell first.**

Slice 1 (data model + comparison shell + gold deserialization) is the main risk
lever for the whole "prevent drifting gold" goal. Recommend it lands and is
green before slices 2–5 generate artifacts, so every derivation slice is written
against an executable comparison from day one. Not a gate; just the order I'd
expect.

**Finding 3 (minor) — timeline scalar `source_ref` vs multi-source events.**

Events like `propagation` or `recovery` can be derived from more than one source
(an anomaly window plus a metric return-to-baseline). The gold shape uses a
single scalar `source_ref`, which the design matches — good for comparison — but
the design's own provenance contract requires inspectable provenance on the
runtime object. Confirm the runtime envelope can carry the full provenance set
(per the "additional provenance fields" allowance) while projecting one scalar
ref into the fixture-compatible shape. No design change needed; just don't let
the scalar-ref projection become the only provenance the object holds.

**Finding 4 (minor) — make timeline text comparison normalized, not byte-equal.**

Stable NL text is fine for this corpus, but a strict string-equality check will
churn on trivial wording edits. Recommend the comparison normalize whitespace and
treat text as a stable-but-secondary field (marker + entity + time + source_ref
are the primary keys). This keeps the design's "approve a structured payload only
if NL proves too brittle" escape hatch from being hit prematurely.

**Finding 5 (formal-doc follow-up, low priority) — reconcile the roadmap.**

`roadmap.md`'s "Near-Term Review Order" (the numbered list near the end) jumps
`entity-resolver-confidence` → `evidence-compiler-ranking`, omitting
`derived-context-baseline`, even though Milestone 5B's body names this topic.
That is a real internal inconsistency. Insert `derived-context-baseline` between
items 5 and 6 as a one-line formal-doc edit (carried with a future round's
covered docs, not in this review document). This also moots the review doc's
question 8 — the branch-specific topic selection is fine; the roadmap list is
just stale.

### Summary

Design approved on direction. No code may start until the design gate is
satisfied (all active reviewers agree). Finding 1 must be folded into the formal
design before the timeline builder is implemented; Findings 2–4 are
implementation guidance; Finding 5 is a low-priority roadmap correction. Because
this round leaves actionable feedback (Finding 1 in particular), a `review-1`
round is justified to carry the Implementor's response and the first
implementation slice.
