# Hot Context Store Review 3

- Baseline SHA: `4ed66fa2be0c4df04f46360e52897c4d12de1472`
- Current milestone: Milestone 4 local hot context store that loads fixture
  inputs and same-fixture expected artifacts into an in-memory recent-window
  substrate, dereferences Evidence IR source refs to concrete records, exposes
  stable time-window and entity selectors, and lets the fixture-backed
  `get_evidence_bundle` path exercise store-backed source lookup checks.
- Critical path: yes - this implements the approved slice 2 selector and query
  integration work needed for the Milestone 4 store boundary.
- Milestone progress: implemented slice 2: `SourceQuery` selectors over stored
  records, deterministic selector ordering, store-aware source-ref resolution in
  `get_evidence_bundle`, query time/entity context checks, and tests proving the
  public fixture-backed path still returns gold bundles unchanged while
  exercising the hot store.
- Deferred milestone work: optional inspection CLI and any reviewer-requested
  error-report polish remain for a final slice. Simulator, OTLP ingest,
  derivation, ranking, persistence, and MCP/API surfaces remain out of scope.

## Response To Review 2

Review 2 accepted slice 1 and explicitly approved slice 2 as the next action.
This round implements only that approved slice.

The two non-blocking notes from review 2 remain conscious constraints:

- one-sided time windows are still hard fixture-load errors; no current fixture
  uses one;
- timestamp ordering and overlap checks still rely on the committed fixture
  convention of comparable RFC3339 UTC strings.

## Slice 2 Implementation

### Store Selectors

Added `SourceQuery` and `HotContextStore::select`.

Selector semantics:

- absent time window, empty entity list, and empty kind list each mean "no
  filter" for that dimension;
- populated dimensions combine with AND;
- entity and kind lists are OR within the dimension;
- records with `time_window: None` do not match time-filtered queries;
- output preserves store insertion order, which preserves fixture load order for
  fixture-backed records.

Added tests for time-window overlap selection, entity+kind filtering, and stable
fixture-order output.

### Store-Aware Query Path

Changed `get_evidence_bundle` to load the registry-backed `FixtureCase` from
`FixtureCorpus` instead of only using the narrow `expected.json` bundle loader.
The corpus load uses `CARGO_MANIFEST_DIR` so this fixture-backed stub is not
dependent on the caller's current working directory.

The public function still returns the same fixture gold bundle unchanged. Before
returning it, the function now:

- validates the query and fixture gold bundle as before;
- preserves existing budget, raw-ref, and counter-evidence checks;
- loads the same fixture into `HotContextStore`;
- resolves every returned evidence item `source_refs[*]` through the store;
- fails on missing, ambiguous, unsupported, or signal-mismatched source refs;
- checks that the query time window selects hot-store records;
- checks that query entities, when present, exist in the hot store;
- checks that query time window and query entities, when both are present,
  select at least one shared hot-store record.

This keeps Milestone 4 scoped to context lookup and contract validation. It does
not generate, rank, filter, or rewrite evidence.

### Tests

Added query-path tests proving:

- every current fixture can pass through `get_evidence_bundle` with store-backed
  source-ref resolution;
- returned bundles remain identical to the fixture gold bundle;
- entity selector checks do not rewrite the returned bundle;
- nonexistent query entities fail with a structured unsatisfied requirement;
- a time window and entity that each match separately but not on the same record
  fail the combined hot-context selector check.

## Reviewer Focus Requested

Please review slice 2 implementation first, then judge whether Milestone 4 needs
a final polish slice.

Specific points needing reviewer attention:

1. Are `SourceQuery` selector semantics correct for Milestone 4: AND across
   populated dimensions, OR within entity/kind lists, no-time records excluded
   from time-filtered queries, and stable insertion-order output?
2. Is the `get_evidence_bundle` integration strict enough now that it resolves
   every returned source ref and checks query context without filtering or
   rewriting the gold bundle?
3. Is the combined time+entity query-context check the right threshold for this
   milestone, or should it require stronger evidence-item-level alignment?
4. Is using `CARGO_MANIFEST_DIR` for the registry-backed fixture corpus
   acceptable for this fixture-backed stub, or should fixture-root handling be
   centralized with the older narrow fixture loader in a final slice?
5. Should a final slice add the optional inspection CLI and/or more public
   source-lookup failure tests, or is Milestone 4 complete after this slice once
   reviewers accept it?
6. Should the one-sided time-window behavior stay as a hard load error until a
   fixture needs it, or should the final slice decide and test that behavior now?

## Verification

- Ran `cargo fmt -- --check`: passed.
- Ran `git diff --check`: passed.
- Ran `cargo test --target-dir target/codex-test`: passed.
- Ran `cargo check --target-dir target/codex-check`: passed.
- Ran `cargo clippy --target-dir target/codex-clippy --all-targets --all-features -- -D warnings`: passed.
- Ran `cargo run --target-dir target/codex-run --bin validate_fixtures`: passed
  with 0 errors and 0 warnings.

Note: as in review 2, verification used alternate Cargo target directories to
avoid the default Windows `target/debug` executable-lock issue observed earlier
in this worktree.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->
