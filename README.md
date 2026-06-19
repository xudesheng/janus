<p align="center">
  <img src="assets/janus-mark.svg" alt="Janus logo" width="112" height="112" />
</p>

<h1 align="center">Janus</h1>

<p align="center">
  An early exploration of an AI-first evidence backend for OpenTelemetry-shaped data.
</p>

---

## Status

Janus is a personal weekend exploration project.

I am using it to think through a specific question: what should exist after the
OpenTelemetry pipeline if the primary consumer is an AI agent, not a human
dashboard user?

The project is intentionally early. Time is limited, the code is not useful yet,
and many ideas are still being clarified. Please be patient with rough edges,
unfinished documents, changing terminology, and incomplete implementation.

This repository is here to claim the problem space, make the thinking public,
and create a place where the design can become concrete.

## Why

OpenTelemetry is becoming the common contract for producing and exchanging
telemetry. It standardizes APIs, SDK behavior, semantic conventions, resources,
traces, metrics, logs, OTLP, and the Collector pipeline.

That is the right boundary for OpenTelemetry.

But after telemetry leaves an SDK exporter or Collector exporter, the backend
still has to decide how to store, index, summarize, correlate, retain, and expose
that data. Most observability backends were designed around human workflows:
dashboards, log search, trace waterfalls, metric queries, alert rules, and
manual investigation.

AI agents have a different bottleneck.

They do not need another dashboard-shaped database. They need the right evidence
in the right shape:

- recent operational context;
- entities and relationships;
- changes and deployments;
- anomaly windows;
- representative traces, logs, and metric segments;
- source-backed summaries;
- counter-evidence;
- uncertainty;
- provenance;
- a compact evidence bundle that fits into an agent's working context.

Janus explores the idea that the next useful layer is not a better charting
system, but an operational evidence compiler.

In short:

> Janus tries to turn telemetry into structured, inspectable evidence for AI
> agents.

## What Janus Is

Janus is intended to be an AI-first backend layer for OpenTelemetry-shaped data.

The goal is to ingest standard telemetry, preserve links back to source data, and
derive a hot/warm operational memory that agents can use for investigation.

Janus should help agents answer questions like:

- What changed near the time this service became unhealthy?
- Which entities are implicated?
- Which evidence supports this hypothesis?
- Which evidence weakens or contradicts it?
- What data is missing?
- What should the agent inspect next?
- Has this happened before?

## What Janus Is Not

Janus is not trying to replace OpenTelemetry.

Janus is not trying to be a full APM product first.

Janus is not trying to be a dashboard-first observability backend.

Janus is not trying to be the RCA agent itself.

The intended boundary is narrower:

> Existing agents should be able to use Janus as an evidence substrate.

## Core Ideas

### Evidence IR

Janus should expose a structured intermediate representation for operational
evidence. An evidence item should be more than a text summary. It should carry
fields such as:

- claim;
- evidence kind;
- time window;
- related entities;
- source references;
- strength;
- direction, such as supports or contradicts;
- freshness;
- missing data;
- token cost;
- privacy scope.

### Hot/Warm/Cold Memory

Recent data is usually the most valuable data for agent work.

Janus should keep the hot layer rich and redundant, even if that costs more,
then compact older data into summaries, representative examples, incident memory,
and backlinks.

### False Causality Guard

For operations work, a plausible but wrong explanation can be worse than no
answer. Janus should make false causality harder by preserving:

- time alignment quality;
- dependency direction;
- blast-radius direction;
- counter-evidence;
- entity-resolution uncertainty;
- missing signals.

### Agent-Oriented APIs

Instead of exposing only raw metric/log/trace queries, Janus should expose
investigation primitives:

- `get_evidence_bundle`;
- `build_timeline`;
- `expand_entity_context`;
- `find_related_anomalies`;
- `compare_windows`;
- `suggest_next_checks`.

These may be exposed directly as APIs and through MCP tools.

## Things To Build

The first useful version should stay small and measurable.

### 1. Ingestion

- Accept OTLP traces, metrics, and logs.
- Preserve source references for raw telemetry.
- Accept change events from CI/CD, deployment systems, Kubernetes, or a simple
  manual API.

### 2. Raw Event Substrate

- Store recent telemetry in a queryable substrate.
- Prefer practical storage over novelty.
- Start with a columnar store or simple local implementation before inventing
  new storage machinery.

### 3. Entity Resolution

- Resolve services, routes, hosts, pods, deployments, dependencies, tenants, and
  environments from telemetry attributes.
- Carry confidence for entity mappings.
- Make identity uncertainty visible to the agent.

### 4. Derived Operational Context

- Build dependency relationships.
- Build anomaly windows.
- Cluster related log and error patterns.
- Attach representative traces, logs, and metric segments.
- Track recent changes and deployment context.

### 5. Evidence IR

- Define the evidence schema.
- Link every summary back to source evidence.
- Include support, contradiction, missing data, confidence, and freshness.

### 6. Agent Interface

- Expose the first investigation primitives.
- Provide MCP tools for agent integration.
- Optimize returned evidence for token budget and diagnostic value.

### 7. Evaluation Harness

- Build a small incident corpus.
- Compare an agent using raw backend access against the same agent using Janus.
- Measure suspicious-entity accuracy, timeline quality, false-causality rate,
  token cost, missing-data awareness, and auditability.

## Initial Architecture Plan

```text
                 OpenTelemetry SDKs / Agents
                            |
                            v
                    OTLP / Collector
                            |
                            v
                     Janus Ingestion
                            |
          +-----------------+-----------------+
          |                                   |
          v                                   v
  Raw Telemetry Store                  Change Event Store
  traces / metrics / logs              deploys / config / CI
          |                                   |
          +-----------------+-----------------+
                            |
                            v
                 Derivation Pipeline
          entity resolution / topology / anomalies
          log patterns / summaries / source refs
                            |
                            v
                     Evidence Store
          Evidence IR / timelines / bundles / memory
                            |
          +-----------------+-----------------+
          |                                   |
          v                                   v
  Investigation API                    MCP Tools
          |                                   |
          +-----------------+-----------------+
                            |
                            v
                         AI Agents
```

The first implementation does not need this to be a set of separate services.
These are responsibilities. A small Rust service with a practical storage layer
and background derivation jobs is enough to start proving the contract.

## Design Notes

More detailed thinking lives under:

- [`docs/core/what_and_why.md`](docs/core/what_and_why.md)
- [`references/llm-aiops/janus_takeaways.md`](references/llm-aiops/janus_takeaways.md)

## License

License is not finalized yet.

Until the project is ready for broader use, treat this repository as exploratory
design and prototype work.

