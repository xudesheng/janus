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
