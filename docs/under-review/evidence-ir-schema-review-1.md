# Evidence IR Schema Review 1

- Baseline SHA: `0b7fc966aec002e26d4d0d15897c486084369b5e`
- Current milestone: Milestone 1 Evidence IR Contract implemented as exported Rust types, generated JSON Schema, narrow fixture loading, and fixture-backed tests
- Critical path: yes - this is the contract-before-storage artifact required by the roadmap before `get_evidence_bundle`, fixture validation, storage, MCP tools, or evidence compilation
- Milestone progress: implemented and pushed the Evidence IR Rust contract, validation helpers, narrow loader, schema generation path, committed schema artifacts, and tests against all current fixture evidence bundles
- Deferred milestone work: none for Milestone 1; `EvidenceQuery`, `get_evidence_bundle`, source-ref resolution, registry-wide fixture validation, MCP schemas, retrieval, ranking, and storage remain later-milestone work by design

`review-0` has one reviewer section, from Claude. Its Direction Verdict agreed
with the design and said to continue once any other active reviewers agreed.
There are no other reviewer sections in the current file, so implementation
proceeded under the design gate.

## Response To Review 0

Claude left no blocking findings. I carried the non-blocking implementation notes
into the formal doc and code:

- `_`-prefixed fixture annotations inside Evidence IR: chose the strict path.
  `docs/core/evidence-ir-schema.md` now records that `_` helper keys may exist
  outside `evidence_bundle`, but `EvidenceBundle`, `EvidenceItem`, `SourceRef`,
  and budget objects reject unknown fields, including `_` keys.
- Open `confidence` map vs strict tool validators: kept the open
  `map string -> 0..1 number` representation for Milestone 1, and recorded that
  Milestone 7 should revisit whether MCP tool schemas need a closed
  representation.
- `signal: "log"` pointing at `lp-*` refs: left fixtures unchanged. The
  `SourceSignal` enum accepts the current fixture values and reserves
  `log_pattern` for a future split.
- Array schema strictness: added a test that recursively fails if any generated
  schema object with `type: array` lacks `items`.

## Implementation Summary

Added the library surface:

- `src/lib.rs` exports `evidence` and `fixtures`.
- `src/evidence.rs` defines `EvidenceItem`, `EvidenceBundle`, `TimeWindow`,
  `SourceRef`, `EvidenceBudget`, enum vocabularies, `UnitInterval`,
  `SourceRefs`, validation helpers, and schema generation helpers.
- `src/fixtures.rs` implements the narrow fixture loader for a single scenario id
  or explicit `expected.json` path. It deserializes only `evidence_bundle` and
  rejects traversal/path-separator scenario ids.
- `src/bin/generate_schemas.rs` writes committed schema artifacts.

Added generated artifacts:

- `schemas/evidence-ir/evidence-item.schema.json`
- `schemas/evidence-ir/evidence-bundle.schema.json`

Added tests in `tests/evidence_ir.rs` that:

- deserialize and validate every current `fixtures/scenarios/*/expected.json`
  `evidence_bundle`;
- load a bundle by scenario id;
- reject path traversal scenario ids;
- serialize a bundle back to JSON;
- verify generated schemas match committed artifacts;
- verify generated array schemas declare `items`;
- verify `SourceRefs` schema has `minItems: 1`.

Added dependencies:

- `serde`
- `serde_json`
- `schemars`

## Review Focus

Please focus on:

1. Whether the implementation stayed inside Milestone 1 and avoided pulling in
   `EvidenceQuery`, `get_evidence_bundle`, source-ref resolution, full fixture
   validation, MCP schemas, ranking, retrieval, or storage.
2. Whether the Rust types preserve the design's strict boundary while remaining
   compatible with all current fixtures.
3. Whether the validation helpers are appropriately small, especially the
   bundle-level checks for `question`/`hypothesis`, budget token use, item count,
   non-empty `source_refs`, and `0..1` strength/confidence values.
4. Whether the generated schemas are suitable as the first agent-facing
   Evidence IR artifacts, especially `additionalProperties: false`,
   explicit enums, `date-time` strings, array `items`, and
   `source_refs.minItems`.
5. Whether `SourceRefs` as a transparent Rust newtype with custom schema is the
   right way to enforce `minItems: 1` without changing fixture JSON shape.
6. Whether this round completes Milestone 1 or leaves actionable defects that
   require `evidence-ir-schema-review-2.md`.

## Verification

- `git pull --ff-only`: already up to date before work began.
- Read `docs/review-framework.md`.
- Read `AGENTS.md`.
- Read `docs/core/evidence-ir-schema.md`.
- Read `docs/under-review/evidence-ir-schema-review-0.md`.
- Confirmed current branch with `git status --short --branch` and
  `git rev-parse --abbrev-ref HEAD`: `evidence-ir-schema`.
- `cargo fmt`: passed.
- `cargo run --bin generate_schemas`: passed and generated committed schema
  artifacts.
- `cargo test`: passed. Integration tests report 7 passed and cover all current
  fixture evidence bundles.
- `cargo clippy --all-targets --all-features`: passed.
- `git diff --check`: passed before staging the covered implementation and
  formal-doc changes.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

**Milestone 1 is complete. Accept. Next action: stop.** This round is on the
critical path (the contract-before-storage vertebra), and it produces exactly the
milestone artifact: exported `serde` Evidence IR types, generated and committed
JSON Schema, a narrow fixture loader, validation helpers, and tests that pass
against the whole current corpus. I find **no actionable defects, no open
questions, and no new requirements**, so per "Round Termination" no
`evidence-ir-schema-review-2.md` is warranted — report completion and wait for
the next instruction. The notes below are forward-looking polish for later
milestones, explicitly **not** blockers for Milestone 1.

### Milestone progress (judged before local defects)

Every "Definition Of Done" criterion in `docs/core/evidence-ir-schema.md` is met,
verified independently rather than from the implementor's summary:

1. Evidence IR Rust types exist and are exported from the library surface
   (`src/lib.rs` re-exports `evidence` and `fixtures`). ✓
2. All current fixture evidence bundles deserialize without ad hoc parsing
   (`current_fixture_bundles_deserialize_and_validate` loads + validates every
   `fixtures/scenarios/*/expected.json`). ✓
3. Evidence IR invariants are validated in tests via `EvidenceBundle::validate`. ✓
4. `evidence-item.schema.json` and `evidence-bundle.schema.json` are generated and
   committed. ✓
5. The generation path is repeatable and guarded: `generate_schemas` writes the
   artifacts and `generated_schemas_match_committed_artifacts` fails if they
   drift. ✓
6. `cargo fmt --check`, `cargo clippy --all-targets --all-features`, and
   `cargo test` all pass on my machine (**7/7 integration tests**). ✓

The scope boundary held: no `EvidenceQuery`, no `get_evidence_bundle`, no
source-ref resolution, no registry/full-fixture validation, no MCP schemas, no
ranking/retrieval/storage. The loader deserializes only `evidence_bundle` and
ignores the rest of `expected.json`, exactly as Milestone 1 requires.

The four review-0 notes were carried through correctly. The formal doc now
records Contract Principle 8 (`_`-keys stay outside Evidence IR; bundle/item/
source-ref/budget reject unknown fields), the open-`confidence`-map vs.
strict-validator trade-off deferred to Milestone 7, and the array-`items` test
requirement — and all three are backed by code: `#[serde(deny_unknown_fields)]`
on every Evidence IR struct, and `generated_array_schemas_declare_items`.

### Verification I ran

- `cargo fmt --check` → clean; `cargo clippy --all-targets --all-features` → no
  warnings; `cargo test` → 7 passed, 0 failed.
- Inspected both committed schema artifacts: `additionalProperties: false` on
  `EvidenceItem`, `EvidenceBundle`, `EvidenceBudget`, `SourceRef`, `TimeWindow`;
  explicit `snake_case` enums for kind/direction/freshness/signal; `UnitInterval`
  carries `minimum: 0 / maximum: 1`; `SourceRefs` carries `minItems: 1` and
  `items`; `TimeWindow.start/end` are `date-time` strings; `token_cost` and the
  budget counters are `uint32` with `minimum: 0`. The bundle schema correctly
  marks `budget`/`items`/`time_window` required and `question`/`hypothesis`
  optional.
- Confirmed the baseline `0b7fc96` bundles the covered code *and* the formal-doc
  edit, with the review document as its own later commit (`bac4e2b`) — the commit
  gate and frozen-baseline rules are satisfied.

### Answers to the implementor's six questions

1. **Stayed inside Milestone 1?** Yes — no later-milestone surface leaked in.
2. **Strict boundary + fixture compatibility?** Yes. `deny_unknown_fields`
   everywhere, strict enums, and the full corpus still deserializes (the enums
   are supersets of the observed values; `SourceSignal` accepts the current
   `signal:"log"` shape and reserves `log_pattern`).
3. **Validation helpers appropriately small?** Yes. `validate()` checks only the
   contract invariants that plain deserialization cannot:
   `question`/`hypothesis` presence (trim-aware), `tokens_used <= max_tokens`,
   `items <= max_items`, non-empty `source_refs`, non-empty `ref`, and `0..1`
   strength/confidence. No retrieval or scoring logic crept in.
4. **Schemas suitable as first agent-facing artifacts?** Yes, for Milestone 1.
   `additionalProperties:false`, explicit enums, `date-time`, array `items`, and
   `source_refs.minItems:1` are all present. (Draft-07 + open `confidence` map is
   the one thing strict tool-use validators may dislike — already deferred to
   Milestone 7; see notes.)
5. **`SourceRefs` newtype with custom schema?** Yes, this is the right call. A
   `#[serde(transparent)]` newtype keeps the fixture JSON shape (a bare array)
   while letting the hand-written `JsonSchema` impl emit `minItems:1`, which serde
   derive alone cannot express. Clean and well-targeted.
6. **Completes Milestone 1 or needs review-2?** Completes it. No review-2.

### Non-blocking notes (for later milestones, not this round)

- **`load_*` returns an unvalidated bundle.** The loader parses but does not call
  `validate()`; only the test does. That is consistent with the design's
  parse/validate split, but Milestone 2's `get_evidence_bundle` stub will consume
  this loader — decide there whether the public load path should validate by
  default or expose a `load_and_validate` helper, so a downstream caller cannot
  silently accept an empty `source_refs` / out-of-range `strength` bundle that the
  schema would reject. Pure forward ergonomics; nothing to change now.
- **Round-trip test is shallow.** `can_serialize_a_bundle_back_to_json` asserts
  two fields. The design only asked for "serialize at least one bundle," so this
  satisfies Milestone 1, but a parse → serialize → re-parse → `validate()`
  assertion would be a stronger, cheap guarantee when round-trip fidelity starts
  to matter in Milestone 2.
- **Over-budget escape hatch.** `validate()` rejects `tokens_used > max_tokens`
  unconditionally; the design hedged "unless a fixture explicitly marks an
  over-budget case later." Correct for now (no current fixture is over budget);
  revisit only if/when such a fixture is authored.
- **Draft-07 + open `confidence` map.** Already recorded for Milestone 7
  (`mcp-agent-surface`). No action this round.

No further review round is required. Milestone 1 is done.
