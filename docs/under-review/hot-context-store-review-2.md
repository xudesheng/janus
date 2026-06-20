# Hot Context Store Review 2

- Baseline SHA: `e1bee5431b466a49adcaf225eb7fee64253e6c99`
- Current milestone: Milestone 4 local hot context store that loads fixture
  inputs and same-fixture expected artifacts into an in-memory recent-window
  substrate, dereferences Evidence IR source refs to concrete records, exposes
  time-window and entity selectors, and lets the fixture-backed
  `get_evidence_bundle` path exercise store-backed source lookup.
- Critical path: yes - this is the first approved implementation slice for the
  Milestone 4 store boundary.
- Milestone progress: implemented slice 1: shared reference helpers, hot-store
  record envelope, fixture loading, source-key lookup, alias-compatible scalar
  resolution, signal-aware source-ref resolution, and focused tests proving
  current fixture evidence refs dereference to concrete payloads.
- Deferred milestone work: time-window/entity selectors, store-aware
  `get_evidence_bundle` integration, optional inspection CLI, and final
  Milestone 4 DoD remain for later slices. Simulator, OTLP ingest, derivation,
  ranking, persistence, and MCP/API surfaces remain out of scope.

## Response To Review 1

Review 1's Direction Verdict approved the revised design and explicitly allowed
slice 1 implementation. This round implements only that approved slice.

### Shared Reference Helpers

Added `src/references.rs` as the shared source for reference categories,
`ReferenceIndex`, trace-alias lookup, metric/span key helpers, signal-category
mapping, and category display.

Refactored `src/fixture_validation.rs` to consume the shared module instead of
owning a private `ReferenceIndex`. Existing fixture-validation behavior is
preserved; the committed corpus still validates at 0 errors and 0 warnings.

The shared index now also indexes relationship ids when a future fixture gives
relationships explicit ids. Current fixtures are unaffected because their
relationships do not yet carry ids.

### Hot Store Slice 1

Added `src/hot_context_store.rs` with:

- `HotContextStore`;
- `StoredRecord`, `StoredRecordKind`, and `SourceKey`;
- `HotStoreError`;
- `SourceResolution`;
- `load_fixture_case(&FixtureCase)`;
- `insert_record`;
- `resolve_scalar_ref`;
- `resolve_source_ref`.

The store loads raw fixture input records for resources, traces, spans, metrics,
logs, changes, prior incidents, and telemetry gaps. It also loads same-fixture
expected artifacts with explicit ids: entities, relationship ids when present,
anomaly windows, log patterns, and evidence items.

Resolution behavior implemented in this slice:

- source refs return concrete `StoredRecord` payloads;
- `profile` and `external` refs return deterministic unsupported outcomes;
- missing refs, ambiguous scalar refs, and signal/category mismatches are
  distinct outcomes;
- primary identity is `(StoredRecordKind, SourceKey)`;
- same raw key across different kinds can be disambiguated by `SourceRef.signal`;
- same-kind duplicate primary keys fail as loader/insert errors.

The loader treats null optional derived time bounds as absent. This is needed
for existing "no anomaly" derived artifacts such as a healthy anomaly-window
placeholder with `"start": null` and `"end": null`; the record still loads with
`time_window: None` and remains source-resolvable.

### Tests

Added `tests/hot_context_store.rs` covering:

- every current fixture loads into `HotContextStore`;
- every current Evidence IR source ref resolves to a concrete stored record;
- span, metric, change, and derived log-pattern refs resolve to expected record
  kinds and payloads;
- signal/category mismatch is distinct from missing;
- `profile` and `external` are unsupported outcomes;
- duplicate same-kind primary keys fail;
- same raw key across different kinds disambiguates by signal and is ambiguous
  for scalar lookup.

## Reviewer Focus Requested

Please judge slice 1 implementation first, not the later slices.

Specific points needing reviewer attention:

1. Does the `references` module adequately prove the single-source-of-truth
   requirement, with fixture validation refactored to consume it?
2. Is the `HotContextStore` record envelope and `(StoredRecordKind, SourceKey)`
   primary identity implemented consistently with the approved design?
3. Are the source-resolution outcomes strong enough for Milestone 4:
   found, missing, ambiguous, signal mismatch, and unsupported?
4. Is treating null optional time bounds as `time_window: None` acceptable for
   derived records that are still valid source-ref targets?
5. Is it acceptable that fixture validation still has its historical
   warning-only negative test for signal mismatch, while the hot store exposes
   mismatches as hard, distinct resolution outcomes for store-aware use?
6. If this slice is accepted, should the next implementation slice be the
   approved slice 2: time-window/entity selectors plus store-aware source lookup
   checks in the fixture-backed `get_evidence_bundle` path?

## Verification

- Ran `cargo fmt -- --check`: passed.
- Ran `git diff --check`: passed.
- Ran `cargo check --target-dir target/codex-check`: passed.
- Ran `cargo test --target-dir target/codex-test`: passed.
- Ran `cargo clippy --target-dir target/codex-clippy --all-targets --all-features -- -D warnings`: passed.
- Ran `cargo run --target-dir target/codex-run --bin validate_fixtures`: passed
  with 0 errors and 0 warnings.

Note: plain `cargo test` against the default `target/debug` directory initially
hit Windows `LNK1104` executable-lock errors after a prior test run. The full
suite passed using the alternate target directory above.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

On critical path: yes. Milestone progress: strong — accept slice 1. Next action:
continue to slice 2.

This is the first implementation round, and it delivers exactly the approved
slice-1 scope with real, working code — not scaffolding. I independently verified
the baseline (`e1bee54` is the parent of the review-2 commit, so the reviewed tree
is the pushed slice-1 code), re-ran the full suite in an alternate target dir, and
confirmed: `cargo test` all green, `cargo clippy --all-targets --all-features
-D warnings` exits 0, and the committed corpus still validates at 0 errors /
0 warnings. The milestone moved materially: Janus can now load every registered
fixture into a `HotContextStore` and dereference every current Evidence IR source
ref to a concrete payload, which is the M3→M4 "closure → retrieval" step this topic
exists to produce.

Critically, the review-1 acceptance condition is met *as evidence, not assertion*:
`src/fixture_validation.rs` no longer contains any private copy of `ReferenceIndex`,
`categories_for_signal`, `span_ref`, `metric_series_ref`, `resolve_ref_map`, or the
`trace:` alias strip — all are gone and imported from `crate::references`
(`src/fixture_validation.rs:4`), and the validator's behavior is unchanged (0/0,
green fixture-validation tests). That demonstrates one source of truth rather than a
second key derivation that merely happens to agree. The relationship-indexing gap I
flagged in review 1 was also closed (`src/references.rs:93`).

### What I verified in the code

- **Primary identity** is `(StoredRecordKind, SourceKey)` (`hot_context_store.rs:53`,
  `:589-599`); same-kind+key+different-record is a hard `DuplicatePrimaryKey`, while
  same raw key across kinds coexists in the `index` and is disambiguated by
  `SourceRef.signal`. Matches the design exactly.
- **Resolution outcomes** are complete and correctly ordered: unsupported
  (empty signal categories) is checked *before* missing, so
  `profile`/`external` dominate a nonexistent ref (`:186-220`). Found / Missing /
  Ambiguous / SignalMismatch / Unsupported are all distinct and test-covered.
- **Alias resolution is shared**: the store's lookup goes through the same
  `resolve_ref_map` the validator uses (`:613`), so the `trace:` route can't drift
  between the two surfaces.
- **Tests prove behavior, not shape**: the corpus-wide
  "every source ref resolves to a concrete, non-null payload" test
  (`tests/hot_context_store.rs:31-58`) is the right strong check, and the
  signal-mismatch / scalar-ambiguity / duplicate-key cases assert the precise
  outcome variant.

### Answers to the requested reviewer questions

1. **Does `references` prove single-source-of-truth?** Yes — the duplicated logic is
   fully removed from `fixture_validation` and the validator consumes the shared
   module with unchanged output. Confirmed by grep + green validator tests + 0/0.
2. **Envelope and `(StoredRecordKind, SourceKey)` consistent with design?** Yes.
3. **Resolution outcomes strong enough for M4?** Yes — found/missing/ambiguous/
   signal-mismatch/unsupported are all first-class and tested.
4. **Null optional time bounds → `time_window: None`?** Acceptable. A both-null
   derived window loading as `None` while staying source-resolvable is the right
   call for "no anomaly" placeholders. See the one robustness caveat below about the
   *partial* (one-sided) case.
5. **OK that fixture-validation keeps its warning-only mismatch test while the store
   is hard?** Yes — they are different acceptance surfaces. The M3 validator warning
   is a corpus-authoring lint; the M4 store mismatch is the retrieval contract. They
   stay consistent precisely because the corpus is clean, so no warning actually
   fires. No conflict.
6. **Next slice = slice 2 (selectors + store-aware `get_evidence_bundle`)?** Yes —
   approved as the next round.

### Non-blocking notes (address in a later slice or when a fixture forces it)

These do not block slice 1; the current corpus exercises neither, and tests/clippy
are green. Flagging so they are conscious decisions, not surprises.

- **One-sided time windows are a hard load error.** `time_window_from_fields`
  (`hot_context_store.rs:885-909`) errors with `InvalidShape` when exactly one of
  start/end is present (both-absent → `None`, both-present → window). An open-ended
  window — e.g. an ongoing telemetry gap or anomaly with a `start` but no `end` yet —
  would fail the *entire* fixture load, not just that record. No current fixture hits
  this, but telemetry gaps are a plausible source. Decide explicitly for slice 2/3
  whether a one-sided window should be a load error or degrade to a one-bounded /
  `None` window, and add a fixture/test witness for whichever you choose.
- **Timestamp ordering is lexicographic.** `trace_time_window` and
  `metric_time_window` derive bounds by `String` sort (`:936-937`, `:973`). That is
  correct only while all fixture timestamps share one RFC3339 UTC `Z` format; mixed
  offsets or precisions would misorder. Fine today given the corpus convention — just
  worth a comment or a normalization step if real ingest timestamps ever enter this
  path.

Net: slice 1 accepted, no required changes. Proceed to slice 2 under the approved
scope and the Milestone 4 Definition Of Done.
