# Evidence IR Schema Review 0

- Baseline SHA: `23caf936f8e7182d209bc890d48fe55bc9cc9ca4`
- Current milestone: approved Milestone 1 Evidence IR contract in `docs/core/evidence-ir-schema.md`, ready for Rust implementation after reviewer agreement
- Critical path: yes - the user explicitly blocked coding until all reviewers agree on the design, and Milestone 1 cannot start safely without a reviewed contract
- Milestone progress: clarified the formal design gate, implementation phases, and source-reference fixture compatibility note; submitted the design for focused review
- Deferred milestone work: Rust types, fixture loader, generated schemas, and tests are deferred until all reviewers' Direction Verdicts agree that implementation may proceed

This is a design-only first round. There are no previous review findings to
answer.

The design draft under review is `docs/core/evidence-ir-schema.md`. It narrows
Milestone 1 to the executable Evidence IR response contract:

- `EvidenceItem`, `EvidenceBundle`, and supporting response-side types;
- strict enum vocabulary for current Evidence IR values;
- mandatory, non-empty `source_refs` on every evidence item;
- structural uncertainty through `direction`, `missing_data`, optional
  confidence dimensions, and explicit counter-evidence items;
- a narrow read-only loader for one fixture's `expected.json` `evidence_bundle`;
- generated JSON Schema artifacts for `EvidenceItem` and `EvidenceBundle`;
- tests that deserialize and validate current fixture evidence bundles.

The design keeps these out of scope for this topic:

- `EvidenceQuery` and `get_evidence_bundle` behavior;
- MCP tool schemas;
- source-reference resolution back into `input.json`;
- registry-wide fixture validation;
- scoring, ranking, storage, live ingest, and retrieval.

Reviewers should focus on these points:

1. Is this the right Milestone 1 boundary, or does any excluded request/query or
   validation work need to move into `evidence-ir-schema` before code starts?
2. Are `source_refs`, `direction`, `missing_data`, `confidence`, and
   `counter_evidence` strong enough as false-causality guard primitives for
   later milestones?
3. Should Milestone 1 accept the current fixture shape exactly, including
   `source_refs[].signal: "log"` for some `lp-*` log-pattern refs, or should the
   fixtures be migrated to `log_pattern` before implementation?
4. Is the proposed strictness right: reject unknown fields on Evidence IR
   response objects, keep enum values strict, but allow open-ended confidence
   dimension names with numeric `0..1` values?
5. Is it acceptable for invariants such as "at least one of `question` or
   `hypothesis`" and budget checks to live in Rust validation helpers when JSON
   Schema generation cannot express them cleanly enough?
6. Does the three-phase implementation sequence in the design doc need another
   design round, or can implementation begin once all current reviewers'
   Direction Verdicts say to continue?

## Verification

Design/doc verification only; no Rust code was written this round.

- Read `docs/review-framework.md`.
- Read `docs/core/what_and_why.md` before design work.
- Read `docs/core/evidence-ir-schema.md` carefully.
- Read supporting docs `docs/core/evidence-spine.md`,
  `docs/core/roadmap.md`, and `docs/process/fixtures.md`.
- Checked current branch with `git status --short --branch`: branch is
  `evidence-ir-schema` tracking `origin/evidence-ir-schema`.
- Checked current same-topic review files in `docs/under-review/`: directory did
  not exist, so this is `review-0`.
- Scanned all current `fixtures/scenarios/*/expected.json` evidence bundles.
  Observed evidence kinds are `change_event`, `counter_evidence`,
  `log_cluster`, `metric_anomaly`, `missing_data`, `previous_incident`, and
  `trace_exemplar`; directions are `supports`, `weakens`, `contradicts`, and
  `neutral`; freshness values are `settled` and `changing`; source signals are
  `anomaly_window`, `change`, `log`, `metric`, `prior_incident`,
  `telemetry_gap`, and `trace`.
- The fixture scan did not report missing required Evidence IR fields or empty
  `source_refs` on evidence items.
- Ran `git diff --check` before committing the formal design-doc update; it
  passed.

<!-- Reviewer appends below; the Implementor must not edit past this line. -->

## Review (by Claude)

### Direction Verdict

**Agree with the design.** On the critical path: **yes** — this is roadmap
Milestone 1, the contract-before-storage vertebra that every later topic
references. Next action: **continue** — implementation may proceed once every
other active reviewer's Direction Verdict also agrees, per the design gate in
`evidence-ir-schema.md` and the User's instruction that no reviewer disagreement
remain before coding.

The milestone boundary is correct. Keeping `EvidenceQuery`,
`get_evidence_bundle`, MCP schemas, source-ref resolution, and registry/full
fixture validation out of this topic matches the roadmap's Milestone 1 vs.
Milestone 2/3 split exactly. This round is a legitimate design-only round: its
milestone progress is the alignment it secures, and it unblocks the next round's
code.

### Independent verification of the load-bearing claim

Contract Principle 1 is "match the fixture shape first," so I validated the
proposed types against every current `fixtures/scenarios/*/expected.json`
`evidence_bundle` rather than trusting the implementor's scan. Result across
**12 bundles / 51 evidence items: zero mismatches.**

- Every required `EvidenceItem` field is present on all 51 items; no item has
  empty `source_refs`.
- Observed enum values are a strict subset of the proposed vocabulary:
  - `kind`: `metric_anomaly`, `log_cluster`, `missing_data`, `counter_evidence`,
    `change_event`, `trace_exemplar`, `previous_incident` (proposed enum also
    adds `dependency_edge`, `profile_hotspot` from `what_and_why.md` — fine as a
    superset).
  - `direction`: all four (`supports`, `weakens`, `contradicts`, `neutral`).
  - `freshness`: `settled`, `changing`.
  - `source_refs[].signal`: `metric`, `change`, `anomaly_window`, `log`, `trace`,
    `telemetry_gap`, `prior_incident` — all within the proposed list.
  - `confidence` dimensions: exactly the eight named in the design.
- `strength`, all `confidence` values in `0..1`; every `token_cost` is a
  non-negative integer; no bundle has `items > max_items`; no `tokens_used >
  max_tokens`.
- All 12 bundles carry `question` and none carry `hypothesis` — consistent with
  the "at least one of `question`/`hypothesis`" invariant and the note that
  current fixtures use `question`.
- No unknown keys on any item, bundle, or `source_ref`, and **no `_`-prefixed
  annotation keys inside any `evidence_bundle`** — so `deny_unknown_fields` is
  safe against the corpus as it stands today.

This is a strong design: it is faithful to the fixtures it commits to matching,
and the strictness choices (reject unknown fields, strict enums, open-ended
numeric `confidence` map) are well-judged. The points below are **non-blocking**;
none gate the start of implementation.

### Answers to the implementor's six questions

1. **Milestone boundary** — correct as drawn; nothing excluded needs to move in.
2. **False-causality primitives** — `source_refs` + `direction` +
   `missing_data` + `confidence` + explicit `counter_evidence` items are
   sufficient as the *contract substrate*. Guard *behavior* is correctly deferred
   to Milestone 6; this round only needs them representable, and they are.
3. **`log` vs `log_pattern` migration** — do **not** migrate now. The four
   `signal:"log"` + `lp-*` items are confirmed (`deploy-bad-rollout/ev-2`,
   `resource-exhaustion-memory/ev-2`, `retry-storm-amplification/ev-3`,
   `schema-migration-errors/ev-2`). Churning fixtures during the first executable
   contract adds risk with no Milestone-1 benefit, since source-ref *resolution*
   does not exist until Milestone 3/4. Accept the current shape; keep
   `log_pattern` reserved. (Forward debt, recorded below.)
4. **Strictness asymmetry** — yes, accept it. Deny-unknown + strict enums + open
   numeric `confidence` map matches the corpus and the "store confidence, don't
   freeze its names" lesson.
5. **Invariants in Rust helpers** — acceptable. "At least one of
   `question`/`hypothesis`" and the budget relations are awkward in generated
   JSON Schema; Rust validation is the right home. (JSON Schema *can* express the
   one-of via `anyOf`/`required`, but it is not worth the generator complexity
   this round.)
6. **Three-phase sequence** — no further design round required for sequencing.
   The three phases are intra-milestone ordering; implementation can begin on
   reviewer agreement.

### Non-blocking notes to carry into implementation

- **`deny_unknown_fields` vs. fixtures' `_`-prefixed annotations.**
  `docs/process/fixtures.md` blesses `_`-prefixed non-normative keys (`_note`,
  `_for_capability`) in `expected.json`. They currently appear only at the
  `expected.json` top level — *outside* the `evidence_bundle` the loader extracts
  — so deny-unknown is safe today. Decide explicitly what happens if a future
  fixture puts a `_`-key *inside* `evidence_bundle` or an item: either the strict
  Evidence IR types reject it (and fixtures.md must say `_`-keys are not allowed
  inside the bundle), or the deserializer skips `_`-prefixed keys. Pick one and
  record it so the two docs cannot drift. This is a contract decision, not a code
  detail.
- **Open `confidence` map vs. strict tool-use validators (Schema And Agent
  Surface Checks).** An open-ended map serializes as
  `additionalProperties: {type: number}`, which the strictest tool-use validators
  (closed-object, all-properties-required) reject. It is fine for Milestone 1
  because `confidence` is response-side and MCP schemas are out of scope — but the
  design's own line "schema compatibility with strict tool validators matters"
  makes this a known tension to revisit at `mcp-agent-surface` (Milestone 7), not
  to solve now. Worth one sentence in the doc so it is a recorded trade-off rather
  than a surprise.
- **`log` signal pointing at `lp-*` refs is forward debt for Milestone 3.** When
  `fixture-validation-harness` resolves `source_refs` back into `input.json` /
  derived artifacts, a `signal:"log"` ref to a `log_pattern` id will need either
  resolution-time handling or the deferred fixture migration. Accept now; flag it
  there.
- **Schema generation: assert `items` on every `type: array`.** The framework's
  array-items check is explicit. The design already commits to it; make the
  schema round-trip test fail loudly if any generated array property lacks
  `items` (notably `source_refs`, `items`, `entities`, `missing_data`).

No blocking defects. Once the other active reviewers agree, proceed to the Phase
1 implementation round.
