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
