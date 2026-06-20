# Fixture Validation Harness Review 0

- Baseline SHA: `437b47873994142eada9e84cbbe201ab3df2b8fe`
- Current milestone: A committed Milestone 3 fixture validation harness that validates the registered corpus with one command, fails structural fixture defects, exposes stable selectors, and reports coverage.
- Critical path: yes - the harness is the executable acceptance gate for fixture-backed Janus work before hot-store and derivation implementation.
- Milestone progress: Design-only round submitted; `docs/process/fixture-validation-harness.md` now includes an explicit design review gate, implementation phases, and sharper review-focus questions.
- Deferred milestone work: Rust implementation, CLI, tests, and fixture defect fixes are deferred because the User explicitly required reviewer agreement on the design before coding.

This is the first review round for the `fixture-validation-harness` topic, so
there are no prior review findings to answer.

The covered design is `docs/process/fixture-validation-harness.md`. It keeps the
Milestone 3 boundary from `docs/core/roadmap.md`: make the incident corpus
executable acceptance data before building storage, derivation, or real
retrieval.

The proposed harness contract is:

- load `fixtures/registry.json` and each registered fixture's `scenario.json`,
  `input.json`, and `expected.json`;
- validate registry vocabulary, paths, manifest/file agreement, declared
  capabilities, failure classes, difficulty, and id consistency;
- reuse the existing `EvidenceBundle` Rust validation for
  `expected.evidence_bundle`;
- build a same-fixture reference index from input artifacts and expected derived
  artifacts;
- fail dangling source references and reject external refs unless a future
  fixture model gives them an in-fixture target;
- report initial `signal`/ref mismatches as warnings when the ref resolves,
  especially for current `signal: "log"` references to derived log patterns;
- enforce structural witnesses for declared capabilities;
- enforce structural false-causality and missing-data guards without claiming to
  calculate true causality;
- print deterministic coverage by failure class, capability, difficulty, and
  false-causality-trap status;
- expose selectors by capability, failure class, and difficulty for later tests
  and eval runs;
- provide a no-argument validation command, currently proposed as
  `cargo run --bin validate_fixtures`.

Reviewers should focus on these decisions:

1. Direction: should this whole Milestone 3 design be approved before coding, or
   should implementation be approved phase by phase?
2. Reference closure: are the supported reference forms complete enough, and is
   failing `external` refs the right default?
3. Compatibility: should `signal`/ref mismatches start as warnings, or should
   the design require immediate fixture cleanup and hard errors?
4. Capability declarations: should missing structural witnesses be hard errors
   for every declared capability?
5. False causality and missing data: are the proposed checks structural enough
   to be implementable now, while still preventing confident unsupported
   conclusions later?
6. Implementation shape: is it acceptable to keep nested OTel-shaped input as
   `serde_json::Value` with targeted extractors for this milestone?

## Verification

- `git diff --check`: passed for the formal design-doc update before this review
  document was created.
- No code verification this round; this is a design-only review submission and
  no Rust implementation was started.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

On critical path: **yes**. Milestone 3 in `docs/core/roadmap.md` makes the incident
corpus executable acceptance data before any store/derivation work, and this design
is the executable gate for that. This is the correct next topic per the roadmap's
contract-first sequencing.

Moves the milestone: **yes**. This is a design-only round; its milestone progress is
securing reviewer agreement on the harness contract, which it does well. The design
is grounded in `fixtures.md`, the roadmap, and the existing Milestone 1 code, and its
factual claims check out against the actual corpus (verified below).

**I agree with the design direction and with the proposed defaults for all six review
questions.** The architecture (corpus model + staged, all-issues-collected validation
+ reference-index closure + structural witnesses + coverage report) is the right shape
and does not need rework.

Next action: **continue, with one design-doc reconciliation required before coding,
plus two clarifications.** Because round 0 leaves actionable feedback, the implementor
should fold these into the formal design doc and respond in a `review-1.md` (it may
remain design-only). I am not blocking on architecture — only on closing the gap in F1
so the design's own purpose holds.

### Verification I ran

- Baseline `437b47873994142eada9e84cbbe201ab3df2b8fe` is the pre-review-document tree
  (the design-doc commit), pushed to `origin/fixture-validation-harness` and fetchable;
  the review-0 commit only adds the review doc and is correctly not the baseline.
- Existing narrow loader (`load_bundle_by_scenario_id`,
  `load_bundle_from_expected_path`) and the `validate_scenario_id` path-safety guard
  exist in `src/fixtures.rs` — the "keep narrow loader, share path safety" plan is
  accurate.
- `SourceSignal` (`src/evidence.rs`) includes `LogPattern`, `TelemetryGap`, and
  `External`. This grounds Q3 (clean form is `signal: "log_pattern"`) and the
  fail-`external` rule.
- The `signal: "log"` + `ref: "lp-1"` (a log-pattern id) case is real:
  `fixtures/scenarios/retry-storm-amplification/expected.json`. The
  warning-then-tighten approach is justified by an actual fixture, not a hypothetical.
- No `external` source refs exist in the corpus today, so failing them now is
  zero-churn.
- Registry: 12 fixtures, all present on disk, `proposed` empty.

### Answers to the six review questions

1. **Whole-design approval vs phase-by-phase.** Approve the whole design now; land
   implementation in the four phases the doc already lists. The phases share one corpus
   model and one issue model, so agreeing the contract up front is cheaper than
   re-litigating it per phase. The doc's own caveat (Phase 1 alone is not a complete
   Milestone 3) is correct and should be kept.
2. **Reference closure / failing `external`.** Agreed. The supported forms (Evidence IR
   `{signal, ref}`, scalar ids, `trace:<id>[/<span>]`, relationship-evidence
   shorthands) cover every reference-bearing field I see in the corpus. Failing
   `external` by default is right and currently zero-churn.
3. **`signal`/ref mismatch as warning vs hard error.** Start as a warning — but see F3:
   a "tighten later" warning with no committed conversion trigger tends to become
   permanent. Bind the tightening to a concrete condition.
4. **Capability witnesses as hard errors.** Agreed they should be hard errors — an
   undeclared-but-claimed capability silently misleads downstream eval/tests. See F2:
   define "witness" as present **and** non-empty.
5. **False-causality / missing-data checks.** Agreed they are structural enough. They
   assert the *presence* of counter-evidence / missing-data channels and ordering
   constraints without computing causality, which is the right line for this milestone.
6. **Nested input as `serde_json::Value` + targeted extractors.** Agreed for this
   milestone. Typed structs for registry/manifest, `Value` + extractors for nested
   OTel-shaped input is the pragmatic boundary; a full input model is a Milestone 4
   concern.

### Findings

**F1 (must reconcile before coding) — derived-artifact vocabularies are unguarded, and
the corpus already contradicts `fixtures.md`.** The design validates canonical
vocabulary for registry/manifest (capabilities, failure classes, difficulty) but is
silent on derived-artifact enums: `timeline[*].marker`, `relationships[*].type`,
`anomaly_windows[*].signal`. Concretely, `fixtures/scenarios/missing-data-gap/expected.json`
uses `marker: "data-gap"`, which is **not** in the `fixtures.md` marker enum (`change,
symptom, propagation, recovery, trigger, amplification, non-causal-change`). This is
exactly the "silent drift away from the contract" the harness exists to prevent
(design Purpose Q3). Decide one of:
- add a lightweight vocabulary check for these derived enums and reconcile the
  discrepancy (either add `data-gap` to the `fixtures.md` marker enum or change the
  fixture), or
- explicitly scope derived-artifact enum validation out of Milestone 3 in the design
  doc, with a one-line reason and a forward pointer.
  Either is acceptable; leaving it unstated is not, because the corpus is already
  drifting and a green harness would imply it is clean.

**F2 (clarify) — define "witness" as present and non-empty.** Stage 6 says validation
fails if a capability "lacks its witness," but several witnesses are arrays
(`expected.entities`, `expected.relationships`, `expected.timeline`, …). State
explicitly that the witness key must be present **and** non-empty (and for
`token-budget-retrieval`, that `evidence_bundle.budget` exists), so an empty `[]` does
not pass as a witness.

**F3 (clarify) — give the Q3 warning a conversion trigger.** A warning that is meant to
become an error "after current fixtures are clean" needs a concrete trigger or it
ossifies. Recommend: the design commit that once the corpus has zero
`signal:"log"`→log-pattern (and similar) mismatches, the mismatch becomes a hard error
in the same or the next round — and that the coverage/issue report counts these
warnings so the "clean" condition is observable. As a forcing function, consider
fixing `retry-storm-amplification` to `signal: "log_pattern"` within Milestone 3 so the
warning path is exercised by a test but the corpus ships clean.

**Non-blocking observations.**
- Stage 5's reference-bearing field list correctly excludes `suspected_causes[*].entity`
  (the corpus uses pseudo-entities like `under-determined` / `under-determined` in
  `missing-data-gap`), so those are not treated as refs. Good — keep it that way.
- `expected.json` and `input.json` both already use `_note`, so the `_`-prefixed
  helper-key allowance in the data model is load-bearing, not theoretical. The
  manifest `inputs`/`expected` equality check must compare against non-helper keys only,
  as the design states.
