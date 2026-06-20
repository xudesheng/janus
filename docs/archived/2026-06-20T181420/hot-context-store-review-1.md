# Hot Context Store Review 1

- Baseline SHA: `e06bd99d8c6266474e0ed2d777a962b23e29957a`
- Current milestone: Milestone 4 local hot context store that loads fixture
  inputs and same-fixture expected artifacts into an in-memory recent-window
  substrate, dereferences Evidence IR source refs to concrete records, exposes
  time-window and entity selectors, and lets the fixture-backed
  `get_evidence_bundle` path exercise store-backed source lookup.
- Critical path: yes - this round resolves the design-contract items that
  review 0 marked as required before coding.
- Milestone progress: updated `docs/core/hot-context-store.md` to address the
  review 0 must-resolve findings around shared reference-index semantics,
  hard-failure mismatch policy, primary-key namespace, unsupported signal
  handling, and the store-aware query path.
- Deferred milestone work: Rust implementation remains intentionally deferred
  until reviewers agree that the revised design is ready. No store code,
  selectors, query integration, or CLI work was started in this round.

## Response To Review 0

Review 0's Direction Verdict was "continue" and approved the overall direction,
but required three design-contract refinements before coding. This round folds
those points into the formal design doc.

### Finding 1: Reuse The Existing Reference Index

Addressed in `docs/core/hot-context-store.md`.

The design now states that Milestone 4 should reuse, promote, or extend the
Milestone 3 `fixture_validation::ReferenceIndex` key and alias scheme instead
of re-deriving source keys in a parallel implementation. The intended split is:

- Milestone 3 uses the shared key scheme to project `ref -> categories` and
  prove fixture closure.
- Milestone 4 uses the same key scheme to map refs to concrete stored-record
  handles.

This keeps "closure" and "retrieval" aligned on one source of truth.

### Finding 2: Make Mismatch A Hard Outcome

Addressed in `docs/core/hot-context-store.md`.

The design no longer keeps a general warning path for signal/category
mismatches. For the committed corpus, a mismatch is a distinct resolution
outcome and a hard failure in store-aware validation and `get_evidence_bundle`
source lookup. A future warning escape hatch would require an explicit,
test-covered fixture witness or flag.

I also ran `cargo run --bin validate_fixtures`; it reported 0 errors and
0 warnings, which supports the hard-failure policy for the current corpus.

### Finding 3: Define Primary-Key Namespace

Addressed in `docs/core/hot-context-store.md`.

The design now defines primary store identity as `(StoredRecordKind, SourceKey)`,
not the raw key string alone. Duplicate behavior is now explicit:

- same kind + same primary key + different record is a loader error;
- same raw key across different kinds is allowed in the index;
- `SourceRef.signal` disambiguates signal-aware lookups;
- scalar ref lookups report ambiguity when the raw key maps to multiple
  categories and no signal can choose one.

This separates duplicate-key loader errors from ambiguous-resolution outcomes.

### Minor Feedback

Also addressed:

- `profile` refs now have a deterministic unsupported outcome until a profile
  source record kind exists, matching the existing unsupported `external` path.
- The fixture-loading section now names the `fixture_validation` module as the
  source of `FixtureCorpus` and `FixtureCase`.
- The query-integration section now states that store-aware source lookup needs
  the registry-backed `FixtureCorpus`/`FixtureCase` path, not only the narrow
  `expected.json` bundle loader.

## Reviewer Focus Requested

Please judge whether the review 0 must-resolve findings are now fully addressed
in the formal design and whether implementation may start after this round.

Specific points needing reviewer attention:

1. Is the shared `ReferenceIndex` direction precise enough, or should the design
   name a stricter module/API shape before coding starts?
2. Is the hard-failure policy for signal/category mismatches acceptable for the
   clean committed corpus?
3. Is `(StoredRecordKind, SourceKey)` the right primary identity for the store,
   with scalar ambiguity only when signal-free lookup cannot choose a category?
4. Are `external` and `profile` both sufficiently specified as deterministic
   unsupported outcomes?
5. If the revised design is approved, should the next round implement only
   slice 1: shared reference helpers, store envelope, fixture loading, aliases,
   and source-reference resolution?

If any reviewer still sees a design defect, the next round should remain
design-only. If reviewers agree, the next round can start slice 1 implementation
without touching simulator, OTLP ingest, derivation, ranking, persistence, or
MCP/API surfaces.

## Verification

- Read `docs/core/hot-context-store.md` and
  `docs/under-review/hot-context-store-review-0.md`.
- Checked the current code references named by review 0:
  `src/fixture_validation.rs`, `src/evidence.rs`, and `src/query.rs`.
- Ran `git diff --check`: passed.
- Ran `cargo run --bin validate_fixtures`: passed with 0 errors and 0 warnings.
- No Rust implementation or code tests were run beyond fixture validation; this
  was a design-only review response.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

On critical path: yes. Moves the milestone: yes. Next action: continue —
**start slice 1 implementation.**

I AGREE with the revised design. All three review-0 must-resolve findings are now
folded into `docs/core/hot-context-store.md` accurately and completely (I verified
the doc diff `1d71357..e06bd99`, not just the response summary), the minors are
handled, and the corpus is still clean (0 errors / 0 warnings — code is unchanged
since review 0, only docs moved). The design gate is satisfied from my side: no
remaining design defect, and implementation may begin under the proposed
phase-by-phase slicing. As before, the design's own gate requires *every* active
reviewer to agree — if the User is a second reviewer, defer the start of coding
until they also sign off.

### Confirmation that review-0 findings are resolved

- **Finding 1 (reuse `ReferenceIndex`):** Resolved. "Source Keys" now requires
  promoting/extending `fixture_validation::ReferenceIndex` into a shared
  helper/module as one source of truth, with M3 projecting `ref -> categories` and
  M4 mapping the same keys to concrete record handles ("value richness, not key
  semantics"). Fixture Loading also points at the shared helpers. This is exactly
  the contract I asked for.
- **Finding 2 (hard mismatch):** Resolved. The general warning path is gone;
  mismatch is a distinct resolution outcome and a hard failure in store-aware
  validation and `get_evidence_bundle`, with any future warning escape hatch gated
  behind a test-covered fixture witness/flag. Review Focus #4 updated to match.
- **Finding 3 (key namespace):** Resolved, and resolved the way I'd have chosen:
  primary identity is `(StoredRecordKind, SourceKey)`; same-kind+key+different-record
  is a loader error, same raw key across kinds is allowed, `SourceRef.signal`
  disambiguates, and scalar lookup reports ambiguity only when no signal can choose.
  The loader-error list, Fixture Loading rules, and Tests list were all updated
  consistently — no dangling "duplicate source keys" language left over.
- **Minors:** `profile` now mirrors `external` as a deterministic unsupported
  outcome (resolution is total over the signal enum); `fixture_validation` is named
  as the module for `FixtureCorpus`/`FixtureCase`; the query path correctly notes it
  needs the registry-backed corpus, not just the `expected.json` bundle loader.

### Answers to the requested reviewer questions

1. **Shared `ReferenceIndex` precise enough?** Yes — naming the module/API shape is
   an implementation decision for slice 1, not a design-doc prerequisite. See the
   acceptance suggestion below for how to *prove* the single-source-of-truth claim.
2. **Hard-failure policy acceptable for the clean corpus?** Yes.
3. **`(StoredRecordKind, SourceKey)` the right identity, scalar ambiguity only when
   signal-free lookup can't choose?** Yes — correct and testable.
4. **`external` and `profile` sufficiently specified?** Yes; both are deterministic
   unsupported outcomes and resolution is now total over `SourceSignal`.
5. **Next round = slice 1 only?** Approved. Slice 1 (shared reference helpers, store
   envelope, fixture loading, aliases, source-reference resolution) is the right
   first cut; keep simulator, OTLP, derivation, ranking, persistence, and MCP/API
   out, and preserve the full Definition Of Done.

### Non-blocking notes for slice 1 (implementation-time, not design blockers)

- **Make the refactor *prove* finding #1, don't just assert it.** When you promote
  `ReferenceIndex` into the shared helper, refactor the existing validator to consume
  that helper and treat "validator output unchanged" as an acceptance check — i.e.
  `cargo run --bin validate_fixtures` still reports 0 errors / 0 warnings and the
  `fixture_validation` tests stay green after the extraction. That demonstrates one
  source of truth rather than a second key derivation that merely happens to agree
  today.
- **Relationship-signal refs are not in the current index.** `ReferenceIndex`
  indexes resources, traces, spans, metrics, logs, changes, prior incidents,
  telemetry gaps, entities, anomaly windows, log patterns, and evidence items, but
  `add_expected_refs` never adds a `RefCategory::Relationship` entry
  (`src/fixture_validation.rs:1377-1397`). The design's resolver list says "entity
  ids and relationship refs **when present**," and the clean corpus implies no
  evidence bundle currently carries a relationship `source_ref` (an unresolved one
  would already be a hard error). That's fine for slice 1, but if/when a fixture
  introduces a relationship source ref, the shared index will need relationship
  indexing added — flagging so it isn't a surprise mid-slice.

Net: design approved, no further design round needed from me. Proceed to slice 1
under the stated scope and DoD (pending any other active reviewer's agreement).
