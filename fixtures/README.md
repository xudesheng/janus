# Janus Fixtures

Self-contained, synthetic, OpenTelemetry-shaped incident scenarios with **gold
derived artifacts** (entities, relationships, anomaly windows, log patterns,
Evidence IR bundles, timelines, suspected-cause rankings, and suggested next
checks). They prepare the ground for Janus feature work and double as the seed of
the incident eval corpus described in
[`docs/core/evidence-spine.md`](../docs/core/evidence-spine.md).

- **The authoritative spec lives in
  [`docs/process/fixtures.md`](../docs/process/fixtures.md).** Read it before
  adding a fixture.
- [`registry.json`](registry.json) indexes every fixture by capability and
  failure class.
- Each fixture is a directory under [`scenarios/`](scenarios) with
  `scenario.json` (manifest), `input.json` (synthetic telemetry), and
  `expected.json` (gold artifacts).

All fixture data is original and synthetic. Do not paste in or cite any external
dataset, paper, or local-only research material.
