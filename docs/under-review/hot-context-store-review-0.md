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

## Review (by Claude)

### Direction Verdict

On critical path: yes. Moves the milestone: yes. Next action: continue.

I AGREE with the design direction. `hot-context-store` is the correct next topic:
sequencing it before any OTLP/simulator work is right, because it turns ingest
into "just another adapter into one store" instead of a one-off demo path, and it
advances Janus from Milestone 3 *closure* ("the ref string exists") to Milestone 4
*retrieval* ("the ref dereferences to a concrete stored record"). Scope control is
good — derivation, ranking, live ingest, durable persistence, and MCP surfaces are
correctly excluded. The `StoredRecord` + `serde_json::Value` payload envelope is
the right altitude for a fixture-shaped store and does not over-commit the later
OTLP path.

Per the design's own Review Gate, this is an agree-with-refinements verdict, not a
blank check: three items below shape the store *contract* and should be settled in
`docs/core/hot-context-store.md` before (or as the first act of) coding, because
they affect testable behavior and the validator/store boundary. None of them is a
redirect. I also explicitly approve the proposed phase-by-phase slicing (see Q5).

### Must-resolve in the design before coding

**1. Reuse the existing `ReferenceIndex`, don't re-derive keys in parallel.**
The canonical source-key and alias conventions the design re-specifies in "Source
Keys" already exist and run today in `ReferenceIndex`
(`src/fixture_validation.rs:1302-1398`): trace ids, `{trace_id}/{span_id}` spans,
`{name}@{entity}` metrics, log/change/prior-incident/telemetry-gap ids, derived
anomaly-window / log-pattern / evidence-item ids, and the `trace:` prefix alias
strip (`resolve`, lines 1315-1323). The design's key table is essentially a second
copy of that logic. Two independent derivations *will* drift, and the failure mode
is exactly the invariant this milestone exists to protect: the store could resolve
a ref the validator rejects, or vice versa. The design currently commits to "don't
duplicate parsing logic" for *loading* (line ~296) but says nothing about the
*key derivation*. Please state explicitly that the store reuses/extends the
`ReferenceIndex` key+alias scheme as the single source of truth — most naturally by
promoting it (or its key-construction helpers) into a shared module that maps a ref
to a concrete record handle, with the M3 `ref -> {RefCategory}` view kept as a thin
projection. The M3↔M4 distinction ("closure vs. retrieval") then becomes "same keys,
richer value," which is exactly the framing the design wants.

**2. Make signal/category mismatch a hard outcome now; the warning path has no
witness (Review Focus #4).** The current corpus validates at **0 errors, 0
warnings** (`cargo run --bin validate_fixtures` on the baseline tree), and
`7dbad28 "Clean fixture source signal mismatches"` already removed the cases that
produced warnings. So the "narrow compatibility warning path while current fixtures
are cleaned up" the design hedges on (lines ~204-208, ~287-292) currently guards
*nothing*. Carrying a warning branch with no fixture exercising it is untested code
and a silent-acceptance risk. Recommendation: in the store, `SourceResolution`
treats signal/category mismatch as a distinct first-class outcome (the design
already lists this — good), and the store-aware `get_evidence_bundle` integration
treats it as a **failure**, not a warning, for the committed corpus. If you want to
keep a warning escape hatch, gate it behind an explicit, test-covered fixture flag
so it cannot regress into silent acceptance — but given the clean corpus I'd drop it
until a fixture actually needs it, and add it back with its witness.

**3. Define the primary-key namespace precisely: global vs. per-kind, and how that
splits "duplicate-key loader error" from "ambiguous-resolution outcome" (Review
Focus #6).** The design says duplicate primary keys are a *loader error*
(lines ~272-278) but also that `SourceResolution` must report *"found multiple
possible targets"* (lines ~255-261). These collide for a cross-category same-string
case — e.g. a log id equal to a change id pointing at two different records. Today
`ReferenceIndex` keys a string to a *set* of categories (line 1304), i.e. it is a
single global namespace that tolerates one string carrying multiple categories.
Decide and write down: is the store's primary-key namespace **global** (so any
same-string collision across kinds is a load-time duplicate error) or **per-kind**
(so the same string can be a log record and a change record, and `resolve_source_ref`
disambiguates by `SourceRef.signal`, only returning "multiple possible targets" when
the signal cannot disambiguate)? Because `SourceRef` already carries `signal`
(`src/evidence.rs:54-59`), per-kind keying with signal disambiguation is the more
faithful and more testable model, and it makes the "ambiguous" outcome reachable
only when it genuinely cannot be resolved. Either choice is fine, but the test plan
("duplicate source keys fail with a useful error" vs. "missing refs fail distinctly
from mismatches") is unspecified until this is nailed down.

### Minor (can be handled in implementation)

- **Total resolution over `SourceSignal`.** The design defines the outcome for
  `external` refs (always fail — good) but is silent on `profile`. `SourceSignal`
  has 12 variants (`src/evidence.rs:106-121`) and `categories_for_signal` maps both
  `Profile` and `External` to `&[]` (`fixture_validation.rs:1717,1724`); there is no
  `profile` record kind in the store's kind list. Please state that `profile`
  resolves to a deterministic "unsupported / no target" outcome too, so resolution
  is total over the enum and can't silently fall through.
- **Doc accuracy nit.** `FixtureCase`/`FixtureCorpus` live in the
  `fixture_validation` module, not `fixtures` (the design says "from the Milestone 3
  harness," which is fine, but the `load_fixture_case(case: &FixtureCase)` signature
  should name the right import path). They are already `pub`, so no API change is
  needed — the store can consume them directly.

### Answers to the requested reviewer questions

1. **Store boundary:** Yes — envelope + JSON payload is strong enough for later
   simulator/OTLP adapters while staying small, provided finding #1 keeps key
   derivation unified with the validator.
2. **Beyond M3 closure:** Yes — requiring a concrete `StoredRecord` (not
   `Option<&_>`, per "Store API Shape") is a real advance over closure checks.
3. **Mismatch policy:** Hard failure now (finding #2). The corpus is clean; don't
   ship an unexercised warning branch.
4. **Scope control:** Correct exclusions. Keep them.
5. **Slicing:** I approve phase-by-phase implementation as proposed (slices 1-3),
   on the condition that **slice 1 settles the key namespace / `ReferenceIndex`
   reuse contract** (findings #1 and #3), since slices 2-3 depend on it. Each slice
   must preserve the full Definition Of Done as the design already requires.
6. **Selector/error semantics:** Precise enough *except* finding #3; deterministic
   ordering preferring fixture order, optional window/entity/kind filters, and the
   four-way `SourceResolution` split are testable as written.
7. **Query integration:** Yes — returning the gold bundle unchanged while
   exercising store-backed source lookup and window/entity selection is the right
   M4 behavior; bundle rewriting/pruning correctly stays Milestone 6. One
   implementation note: today `get_evidence_bundle` loads only `expected.json` via
   `load_bundle_by_scenario_id` (`src/query.rs:107`, `src/fixtures.rs:29`), so the
   store-aware path will additionally need the fixture **input** (the `FixtureCase`),
   which means going through the registry-backed `FixtureCorpus`. Add the store-aware
   helper alongside and keep the current function as a thin wrapper, as the design
   suggests.

Net: direction approved, proceed to implementation under the proposed slicing once
findings #1-#3 are folded into `docs/core/hot-context-store.md`.
