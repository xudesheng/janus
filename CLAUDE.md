# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Janus is a **greenfield** Rust project. The repository currently contains only `cargo init`
scaffolding — `src/main.rs` prints "Hello, world!" and `Cargo.toml` has no dependencies. The
substance of the project lives in its design document, not yet in code.

**Read [`docs/core/what_and_why.md`](docs/core/what_and_why.md) before any design or implementation
work.** It is the canonical source of truth for the project's purpose and direction. (It is written
in Chinese; per the global English-only rule, code/comments/docs you add should still be English.)
The summary below is orientation, not a replacement for the doc.

## Project vision

Janus is an **observability backend whose first-class consumer is an AI Agent**, sitting *downstream*
of the OpenTelemetry pipeline. It starts where OTel's boundary ends:

> If the first consumer of an observability backend is an AI Agent rather than a human dashboard
> user, how should the backend store, organize, summarize, and retrieve telemetry?

- It is **not** a dashboard database, time-series database, log search engine, or trace viewer.
- It **is** a *context and evidence system* — helping an Agent rapidly understand a system's recent
  operational state: what entities exist, how they relate, what changed, what became anomalous, and
  what evidence supports or refutes a hypothesis.
- Janus does **not** redefine OTel instrumentation. OTel API/SDK/semantic-conventions/OTLP/Collector
  stay the upstream contract; Janus consumes standard telemetry and re-organizes it for investigation.

## Design priorities (drive implementation trade-offs)

1. On the hot path, **Agent answer quality beats storage efficiency**.
2. **Second-to-minute latency is acceptable** — this is not a millisecond-query system.
3. **Context is a first-class stored object** (entities, relationships, change events, anomaly
   windows, pattern clusters, summaries, evidence bundles) — not just raw spans/metrics/logs.
4. Recent data matters most; storage is **time-layered (hot → warm → cold)** with different
   priorities per layer (hot may be redundant and enriched; cold optimizes for cost).
5. **Evidence must be traceable** back to source spans/logs/metrics/change records.
6. The query surface supports **investigation, not just retrieval**.

## Planned architecture (design responsibilities, not yet code)

The doc defines a chain of conceptual responsibilities — these may begin as a few tables, indexes,
and background jobs rather than separate services, as long as the boundaries stay clear:

> OTel ingestion → raw telemetry store → entity resolver → relationship builder → change ingestor →
> anomaly detector → pattern clusterer → summarizer → evidence ranker → investigation API →
> retention/compaction pipeline

The intended public contract is an **Agent-oriented query surface of investigation primitives**
(e.g. `explain_symptom`, `build_timeline`, `find_related_anomalies`, `compare_windows`,
`get_evidence_bundle`, `expand_entity_context`, `rank_suspected_causes`, `suggest_next_checks`) that
return *structured context and inspectable evidence* rather than rows. Keep new code consistent with
this layering and naming so the design doc stays the canonical map.

## Commands

```bash
cargo build            # build
cargo run              # run the binary
cargo test             # run all tests
cargo test <name>      # run a single test (matches by name substring)
cargo check            # fast type-check, no binary
cargo clippy           # lint
cargo fmt              # format
```

- Rust **edition 2024** (toolchain 1.96 in use).
- Single binary crate, no workspace yet. If the responsibility chain above grows into real modules, a
  Cargo workspace with one crate per responsibility is the natural structure.
