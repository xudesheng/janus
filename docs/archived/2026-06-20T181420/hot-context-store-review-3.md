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

## Review (by Claude)

### Direction Verdict

On critical path: yes. Milestone progress: slice 2 accepted — and with it the
**Milestone 4 Definition Of Done is met**. Next action: stop the mandatory round
loop and report completion; the remaining CLI work is optional, at the User's
discretion, not a milestone requirement.

This is an implementation round, so I judged progress before defects. Slice 2
delivers exactly the approved scope — `SourceQuery`/`select` selectors and a
store-aware `get_evidence_bundle` — and it closes the milestone. I verified the
reviewed tree independently: baseline `4ed66fa` is the parent of the review-3
commit (so the pushed slice-2 code is what I reviewed), the slice-2 commit touched
only `src/query.rs`, `src/hot_context_store.rs`, and the two test files (no locked
review body was edited), `cargo test` is fully green (get_evidence_bundle suite now
12, hot_context_store 9), `cargo clippy --all-targets --all-features -D warnings`
exits 0, and the corpus still validates 0/0.

### Milestone 4 DoD check (the reason this is a completion verdict)

Walking the design's Definition Of Done against the merged slices 1+2:

- all registered fixture inputs load into the store — done (slice 1, tested).
- all current evidence source refs resolve to concrete records — done, and now
  *enforced* on the query path (`ensure_source_refs_resolve`, `query.rs:380`).
- stable time-window and entity selectors — done (`select` /
  `record_matches_query`, `hot_context_store.rs:244,1086`), deterministic via
  insertion order, tested for overlap + entity/kind filtering + stable order.
- `get_evidence_bundle` (or helper) uses the store for source lookup without
  generating evidence — done; the function returns the gold bundle byte-for-byte
  (tests assert `bundle == expected` for every fixture) while adding lookup +
  context checks.
- source lookup failures are structured and test-covered — done
  (`SourceLookup`, `UnsatisfiedRequirement` with stable `requirement` tags;
  negative tests for missing entity and disjoint time+entity).
- design keeps OTLP/simulator as follow-up adapters — unchanged, still true.
- `cargo fmt`/`test`/`clippy`/`validate_fixtures` pass — verified.

The only thing not done is the **CLI**, which the design lists as "Optional
inspection CLI" and "not the main contract," and which is absent from the DoD. So
the milestone artifact exists. Per Round Termination, since I leave no required
changes and the milestone work is complete, the implementor should **not** emit a
mandatory `review-4`; report completion and let the User decide on the optional
polish slice.

### What I verified in the code

- **Selector semantics** match the design and the round's description: AND across
  populated dimensions, OR within entity/kind lists, `time_window: None` records
  excluded from time-filtered queries (`record_overlaps_window` uses
  `is_some_and`), inclusive interval overlap (`windows_overlap`,
  `hot_context_store.rs:1105`), insertion-order output.
- **No bundle rewriting:** the gold bundle is loaded from
  `case.expected["evidence_bundle"]` and returned unchanged; the new checks are
  side-effect-free reads. Tests assert equality with the fixture gold for all
  fixtures and for the entity-filtered case.
- **Check ordering is correct:** query/budget/raw-ref/counter checks run before the
  store is even built, so the new substrate checks don't perturb the existing
  error contract — which is exactly why the prior get_evidence_bundle tests still
  pass unchanged.

### Answers to the requested reviewer questions

1. **Selector semantics correct?** Yes — AND across dimensions, OR within
   entity/kind, no-time records excluded from time queries, stable order. Correct
   and tested.
2. **Integration strict enough without rewriting?** Yes — every returned source ref
   must resolve to `Found` or the call hard-fails with a structured `SourceLookup`
   error, and the bundle is returned unchanged.
3. **Combined time+entity check the right threshold?** Acceptable for M4. Treat it
   as a *substrate-connectivity* check ("the store actually holds records the query
   can reach"), not evidence validation. One caveat to record: requiring the query
   time window to always select ≥1 record couples `get_evidence_bundle` success to
   window overlap, so a legitimate but tight real-world query window (vs. the
   fixture's own window) could false-negative. Fine now — every fixture passes —
   but revisit when M6 introduces query-dependent selection; don't harden it
   further this milestone.
4. **`CARGO_MANIFEST_DIR` acceptable?** Yes for a fixture-backed stub. Note that
   there are now two fixture-loading paths — the registry-backed corpus (used by
   `get_evidence_bundle`) and the older path-based `load_bundle_by_scenario_id`
   (now effectively a test-only helper in `src/fixtures.rs`). If you do an optional
   polish slice, centralize fixture-root handling and either retire
   `load_bundle_by_scenario_id` or document it as test-only; not required for DoD.
5. **Final slice needed, or done?** Done for Milestone 4. The CLI and extra
   failure tests are optional polish, not DoD items — User's call.
6. **One-sided time window: keep hard error or decide now?** Keep it as-is. No
   fixture needs it and the DoD is met; deciding/testing the one-sided case is
   reasonable optional-slice work but not required now. (Same position as my
   review-2 note.)

### Non-blocking notes (optional-slice candidates, no change required)

- Two fixture-loading conventions now coexist (see Q4) — consolidation is the main
  cleanup worth doing if a polish slice happens.
- The lexicographic-timestamp and one-sided-window assumptions from review 2 still
  hold and are still fine under the committed fixture convention.

Net: slice 2 accepted, Milestone 4 DoD met, no required follow-up round. Stop and
report completion to the User; treat the CLI/polish slice as optional.
