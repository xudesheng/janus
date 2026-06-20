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
