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
