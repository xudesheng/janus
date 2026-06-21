use crate::{
    evidence::{EvidenceBundle, SourceRef, SourceSignal, TimeWindow, ValidationErrors},
    fixture_validation::FixtureCase,
    hot_context_store::{
        HotContextStore, HotIngestEvent, HotStoreError, MetricSeriesKey, SourceQuery,
        SourceResolution, StoredRecordKind,
    },
    references::{metric_series_ref, span_ref},
};
use serde_json::{Map, Value, json};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SimulatedSignal {
    Resource,
    Trace,
    Span,
    MetricPoint,
    Log,
    Change,
    PriorIncident,
    TelemetryGap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimulationEvent {
    pub scenario_id: String,
    pub sequence: u64,
    pub simulated_time: Option<String>,
    pub signal: SimulatedSignal,
    pub source_key: String,
    pub record_kind: StoredRecordKind,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FixtureReplayPlan {
    scenario_id: String,
    events: Vec<SimulationEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureReplaySummary {
    pub scenario_id: String,
    pub events_emitted: usize,
    pub records_stored: usize,
    pub raw_source_refs_resolved: usize,
    pub non_replayed_source_refs_skipped: usize,
    pub query_time_window_records: usize,
    pub validation_errors: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixtureSimulationError {
    MissingField {
        fixture_id: String,
        json_path: String,
        field: &'static str,
    },
    InvalidShape {
        fixture_id: String,
        json_path: String,
        message: String,
    },
}

#[derive(Debug)]
pub enum FixtureReplayError {
    Simulation(FixtureSimulationError),
    Store(HotStoreError),
    FixtureBundle(serde_json::Error),
    InvalidBundle(ValidationErrors),
    SourceLookup {
        item_id: String,
        source_ref: SourceRef,
        message: String,
    },
    QueryWindow(serde_json::Error),
    QueryContext {
        message: String,
    },
}

#[derive(Debug)]
struct EventDraft {
    input_order: u64,
    simulated_time: Option<String>,
    time_sort_key: Option<FixtureTimestampSortKey>,
    signal: SimulatedSignal,
    source_key: String,
    record_kind: StoredRecordKind,
    payload: Value,
}

#[derive(Debug)]
struct EventDraftBody {
    simulated_time: Option<String>,
    signal: SimulatedSignal,
    source_key: String,
    record_kind: StoredRecordKind,
    payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FixtureTimestampSortKey {
    base: String,
    nanos: u32,
}

#[derive(Debug, Clone, Copy)]
struct IdEventSpec {
    key: &'static str,
    signal: SimulatedSignal,
    record_kind: StoredRecordKind,
    time_field: &'static str,
    time_required: bool,
}

pub fn plan_fixture_replay(
    case: &FixtureCase,
) -> Result<FixtureReplayPlan, FixtureSimulationError> {
    FixtureReplayPlan::build(case)
}

pub fn replay_fixture_case(case: &FixtureCase) -> Result<FixtureReplaySummary, FixtureReplayError> {
    let plan = plan_fixture_replay(case)?;
    let store = replay_plan_into_store(&plan)?;
    let bundle = evidence_bundle_from_case(case)?;
    bundle
        .validate()
        .map_err(FixtureReplayError::InvalidBundle)?;
    let source_refs = validate_raw_source_refs(&store, &bundle)?;
    let query_time_window_records = query_time_window_records(&store, case)?;

    Ok(FixtureReplaySummary {
        scenario_id: case.registry_entry.id.clone(),
        events_emitted: plan.events().len(),
        records_stored: store.record_count(),
        raw_source_refs_resolved: source_refs.resolved,
        non_replayed_source_refs_skipped: source_refs.skipped,
        query_time_window_records,
        validation_errors: 0,
    })
}

pub fn replay_plan_into_store(
    plan: &FixtureReplayPlan,
) -> Result<HotContextStore, FixtureReplayError> {
    let mut store = HotContextStore::new();

    for event in plan.events() {
        let ingest_event = HotIngestEvent::try_from(event)?;
        store.ingest(ingest_event)?;
    }

    Ok(store)
}

pub fn format_dry_run_plan(plan: &FixtureReplayPlan) -> String {
    let mut output = format!(
        "fixture {} replay plan: {} event(s)\n",
        plan.scenario_id(),
        plan.events().len()
    );

    for event in plan.events() {
        let simulated_time = event.simulated_time.as_deref().unwrap_or("preload");
        output.push_str(&format!(
            "{:04} {} {} {} {}\n",
            event.sequence, simulated_time, event.signal, event.record_kind, event.source_key
        ));
    }

    output
}

pub fn format_jsonl_plan(plan: &FixtureReplayPlan) -> Result<String, serde_json::Error> {
    let mut lines = Vec::with_capacity(plan.events().len());

    for event in plan.events() {
        lines.push(serde_json::to_string(&event.to_json_value())?);
    }

    Ok(lines.join("\n"))
}

pub fn format_replay_summary(summary: &FixtureReplaySummary) -> String {
    format!(
        "fixture {} replay summary\n\
events emitted: {}\n\
records stored: {}\n\
raw source refs resolved: {}\n\
non-replayed source refs skipped: {}\n\
query time-window records: {}\n\
validation errors: {}\n",
        summary.scenario_id,
        summary.events_emitted,
        summary.records_stored,
        summary.raw_source_refs_resolved,
        summary.non_replayed_source_refs_skipped,
        summary.query_time_window_records,
        summary.validation_errors
    )
}

impl FixtureReplayPlan {
    pub fn build(case: &FixtureCase) -> Result<Self, FixtureSimulationError> {
        let fixture_id = case.registry_entry.id.as_str();
        let mut drafts = Vec::new();
        let mut input_order = 0;

        append_resources(fixture_id, &case.input, &mut input_order, &mut drafts)?;
        append_traces(fixture_id, &case.input, &mut input_order, &mut drafts)?;
        append_metrics(fixture_id, &case.input, &mut input_order, &mut drafts)?;
        append_id_records(
            fixture_id,
            &case.input,
            IdEventSpec {
                key: "logs",
                signal: SimulatedSignal::Log,
                record_kind: StoredRecordKind::Log,
                time_field: "t",
                time_required: true,
            },
            &mut input_order,
            &mut drafts,
        )?;
        append_id_records(
            fixture_id,
            &case.input,
            IdEventSpec {
                key: "changes",
                signal: SimulatedSignal::Change,
                record_kind: StoredRecordKind::Change,
                time_field: "t",
                time_required: true,
            },
            &mut input_order,
            &mut drafts,
        )?;
        append_id_records(
            fixture_id,
            &case.input,
            IdEventSpec {
                key: "prior_incidents",
                signal: SimulatedSignal::PriorIncident,
                record_kind: StoredRecordKind::PriorIncident,
                time_field: "first_seen",
                time_required: false,
            },
            &mut input_order,
            &mut drafts,
        )?;
        append_id_records(
            fixture_id,
            &case.input,
            IdEventSpec {
                key: "telemetry_gaps",
                signal: SimulatedSignal::TelemetryGap,
                record_kind: StoredRecordKind::TelemetryGap,
                time_field: "start",
                time_required: true,
            },
            &mut input_order,
            &mut drafts,
        )?;

        drafts.sort_by(compare_drafts);

        let events = drafts
            .into_iter()
            .enumerate()
            .map(|(index, draft)| SimulationEvent {
                scenario_id: fixture_id.to_string(),
                sequence: index as u64,
                simulated_time: draft.simulated_time,
                signal: draft.signal,
                source_key: draft.source_key,
                record_kind: draft.record_kind,
                payload: draft.payload,
            })
            .collect();

        Ok(Self {
            scenario_id: fixture_id.to_string(),
            events,
        })
    }

    pub fn scenario_id(&self) -> &str {
        &self.scenario_id
    }

    pub fn events(&self) -> &[SimulationEvent] {
        &self.events
    }
}

impl SimulationEvent {
    pub fn to_json_value(&self) -> Value {
        json!({
            "scenario_id": self.scenario_id,
            "sequence": self.sequence,
            "simulated_time": self.simulated_time,
            "signal": self.signal.as_str(),
            "source_key": self.source_key,
            "record_kind": self.record_kind.to_string(),
            "payload": self.payload,
        })
    }
}

impl SimulatedSignal {
    pub fn as_str(self) -> &'static str {
        match self {
            SimulatedSignal::Resource => "resource",
            SimulatedSignal::Trace => "trace",
            SimulatedSignal::Span => "span",
            SimulatedSignal::MetricPoint => "metric_point",
            SimulatedSignal::Log => "log",
            SimulatedSignal::Change => "change",
            SimulatedSignal::PriorIncident => "prior_incident",
            SimulatedSignal::TelemetryGap => "telemetry_gap",
        }
    }
}

impl TryFrom<&SimulationEvent> for HotIngestEvent {
    type Error = FixtureSimulationError;

    fn try_from(event: &SimulationEvent) -> Result<Self, Self::Error> {
        match event.signal {
            SimulatedSignal::Resource => Ok(HotIngestEvent::Resource(event.payload.clone())),
            SimulatedSignal::Trace => Ok(HotIngestEvent::Trace(event.payload.clone())),
            SimulatedSignal::Span => {
                // Fixture span keys are `trace_id/span_id`; real OTLP ingest should carry trace_id directly.
                let Some((trace_id, _span_id)) = event.source_key.split_once('/') else {
                    return Err(FixtureSimulationError::InvalidShape {
                        fixture_id: event.scenario_id.clone(),
                        json_path: "$.source_key".to_string(),
                        message: "span event source_key must be trace_id/span_id".to_string(),
                    });
                };

                Ok(HotIngestEvent::Span {
                    trace_id: trace_id.to_string(),
                    payload: event.payload.clone(),
                })
            }
            SimulatedSignal::MetricPoint => Ok(HotIngestEvent::MetricPoint {
                series: MetricSeriesKey::new(
                    required_str(&event.scenario_id, &event.payload, "$.payload", "name")?,
                    required_str(&event.scenario_id, &event.payload, "$.payload", "entity")?,
                ),
                payload: event.payload.clone(),
            }),
            SimulatedSignal::Log => Ok(HotIngestEvent::Log(event.payload.clone())),
            SimulatedSignal::Change => Ok(HotIngestEvent::Change(event.payload.clone())),
            SimulatedSignal::PriorIncident => {
                Ok(HotIngestEvent::PriorIncident(event.payload.clone()))
            }
            SimulatedSignal::TelemetryGap => {
                Ok(HotIngestEvent::TelemetryGap(event.payload.clone()))
            }
        }
    }
}

impl fmt::Display for SimulatedSignal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.as_str())
    }
}

impl fmt::Display for FixtureSimulationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FixtureSimulationError::MissingField {
                fixture_id,
                json_path,
                field,
            } => write!(
                formatter,
                "fixture `{fixture_id}` {json_path}: missing required field `{field}`"
            ),
            FixtureSimulationError::InvalidShape {
                fixture_id,
                json_path,
                message,
            } => write!(formatter, "fixture `{fixture_id}` {json_path}: {message}"),
        }
    }
}

impl std::error::Error for FixtureSimulationError {}

impl From<FixtureSimulationError> for FixtureReplayError {
    fn from(error: FixtureSimulationError) -> Self {
        Self::Simulation(error)
    }
}

impl From<HotStoreError> for FixtureReplayError {
    fn from(error: HotStoreError) -> Self {
        Self::Store(error)
    }
}

impl fmt::Display for FixtureReplayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FixtureReplayError::Simulation(error) => write!(formatter, "{error}"),
            FixtureReplayError::Store(error) => write!(formatter, "{error}"),
            FixtureReplayError::FixtureBundle(error) => {
                write!(
                    formatter,
                    "failed to parse fixture evidence bundle: {error}"
                )
            }
            FixtureReplayError::InvalidBundle(error) => {
                write!(formatter, "fixture evidence bundle is invalid: {error}")
            }
            FixtureReplayError::SourceLookup {
                item_id,
                source_ref,
                message,
            } => write!(
                formatter,
                "evidence item `{item_id}` source ref {:?} did not resolve: {message}",
                source_ref
            ),
            FixtureReplayError::QueryWindow(error) => {
                write!(
                    formatter,
                    "failed to parse fixture query time window: {error}"
                )
            }
            FixtureReplayError::QueryContext { message } => write!(formatter, "{message}"),
        }
    }
}

impl std::error::Error for FixtureReplayError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FixtureReplayError::Simulation(error) => Some(error),
            FixtureReplayError::Store(error) => Some(error),
            FixtureReplayError::FixtureBundle(error) | FixtureReplayError::QueryWindow(error) => {
                Some(error)
            }
            FixtureReplayError::InvalidBundle(error) => Some(error),
            FixtureReplayError::SourceLookup { .. } | FixtureReplayError::QueryContext { .. } => {
                None
            }
        }
    }
}

struct RawSourceRefValidation {
    resolved: usize,
    skipped: usize,
}

fn evidence_bundle_from_case(case: &FixtureCase) -> Result<EvidenceBundle, FixtureReplayError> {
    serde_json::from_value(case.expected["evidence_bundle"].clone())
        .map_err(FixtureReplayError::FixtureBundle)
}

fn validate_raw_source_refs(
    store: &HotContextStore,
    bundle: &EvidenceBundle,
) -> Result<RawSourceRefValidation, FixtureReplayError> {
    let mut resolved = 0;
    let mut skipped = 0;

    for item in &bundle.items {
        for source_ref in item.source_refs.iter() {
            if !is_raw_replay_source_signal(source_ref.signal) {
                skipped += 1;
                continue;
            }

            match store.resolve_source_ref(source_ref) {
                SourceResolution::Found(_) => resolved += 1,
                resolution => {
                    return Err(FixtureReplayError::SourceLookup {
                        item_id: item.id.clone(),
                        source_ref: source_ref.clone(),
                        message: describe_resolution(resolution),
                    });
                }
            }
        }
    }

    Ok(RawSourceRefValidation { resolved, skipped })
}

fn query_time_window_records(
    store: &HotContextStore,
    case: &FixtureCase,
) -> Result<usize, FixtureReplayError> {
    let time_window: TimeWindow = serde_json::from_value(case.manifest.time_window.clone())
        .map_err(FixtureReplayError::QueryWindow)?;
    let matches = store.select(SourceQuery {
        time_window: Some(time_window),
        ..SourceQuery::default()
    });

    if matches.is_empty() {
        return Err(FixtureReplayError::QueryContext {
            message: "fixture manifest time window matched no replayed hot-store records"
                .to_string(),
        });
    }

    Ok(matches.len())
}

fn is_raw_replay_source_signal(signal: SourceSignal) -> bool {
    matches!(
        signal,
        SourceSignal::Trace
            | SourceSignal::Metric
            | SourceSignal::Log
            | SourceSignal::Change
            | SourceSignal::PriorIncident
            | SourceSignal::TelemetryGap
    )
}

fn describe_resolution(resolution: SourceResolution<'_>) -> String {
    match resolution {
        SourceResolution::Found(_) => "found".to_string(),
        SourceResolution::Missing { raw_ref } => {
            format!("source ref `{raw_ref}` was missing")
        }
        SourceResolution::Unsupported { raw_ref, signal } => {
            format!("source ref `{raw_ref}` has unsupported signal {signal:?}")
        }
        SourceResolution::SignalMismatch {
            raw_ref,
            signal,
            candidates,
        } => format!(
            "source ref `{raw_ref}` had {} candidate(s), but none matched signal {signal:?}",
            candidates.len()
        ),
        SourceResolution::Ambiguous {
            raw_ref,
            candidates,
        } => format!(
            "source ref `{raw_ref}` matched {} candidate(s)",
            candidates.len()
        ),
    }
}

fn append_resources(
    fixture_id: &str,
    input: &Value,
    input_order: &mut u64,
    drafts: &mut Vec<EventDraft>,
) -> Result<(), FixtureSimulationError> {
    let Some(resources) = array_at(fixture_id, input, "resources")? else {
        return Ok(());
    };

    for (index, resource) in resources.iter().enumerate() {
        let json_path = format!("$.resources[{index}]");
        let id = required_str(fixture_id, resource, &json_path, "id")?;
        push_draft(
            fixture_id,
            &json_path,
            drafts,
            input_order,
            EventDraftBody {
                simulated_time: None,
                signal: SimulatedSignal::Resource,
                source_key: id.to_string(),
                record_kind: StoredRecordKind::Resource,
                payload: resource.clone(),
            },
        )?;
    }

    Ok(())
}

fn append_traces(
    fixture_id: &str,
    input: &Value,
    input_order: &mut u64,
    drafts: &mut Vec<EventDraft>,
) -> Result<(), FixtureSimulationError> {
    let Some(traces) = array_at(fixture_id, input, "traces")? else {
        return Ok(());
    };

    for (trace_index, trace) in traces.iter().enumerate() {
        let trace_path = format!("$.traces[{trace_index}]");
        let trace_id = required_str(fixture_id, trace, &trace_path, "trace_id")?;
        let trace_time = trace_start_time(fixture_id, trace, &trace_path)?;

        push_draft(
            fixture_id,
            &trace_path,
            drafts,
            input_order,
            EventDraftBody {
                simulated_time: trace_time,
                signal: SimulatedSignal::Trace,
                source_key: trace_id.to_string(),
                record_kind: StoredRecordKind::Trace,
                payload: trace.clone(),
            },
        )?;

        let Some(spans) = optional_array_field(fixture_id, trace, &trace_path, "spans")? else {
            continue;
        };

        for (span_index, span) in spans.iter().enumerate() {
            let span_path = format!("{trace_path}.spans[{span_index}]");
            let span_id = required_str(fixture_id, span, &span_path, "span_id")?;

            push_draft(
                fixture_id,
                &format!("{span_path}.start"),
                drafts,
                input_order,
                EventDraftBody {
                    simulated_time: Some(
                        required_str(fixture_id, span, &span_path, "start")?.to_string(),
                    ),
                    signal: SimulatedSignal::Span,
                    source_key: span_ref(trace_id, span_id),
                    record_kind: StoredRecordKind::Span,
                    payload: span.clone(),
                },
            )?;
        }
    }

    Ok(())
}

fn append_metrics(
    fixture_id: &str,
    input: &Value,
    input_order: &mut u64,
    drafts: &mut Vec<EventDraft>,
) -> Result<(), FixtureSimulationError> {
    let Some(metrics) = array_at(fixture_id, input, "metrics")? else {
        return Ok(());
    };

    for (metric_index, metric) in metrics.iter().enumerate() {
        let metric_path = format!("$.metrics[{metric_index}]");
        let name = required_str(fixture_id, metric, &metric_path, "name")?;
        let entity = required_str(fixture_id, metric, &metric_path, "entity")?;
        let source_key = metric_series_ref(name, entity);
        let Some(points) = optional_array_field(fixture_id, metric, &metric_path, "points")? else {
            continue;
        };

        for (point_index, point) in points.iter().enumerate() {
            let point_path = format!("{metric_path}.points[{point_index}]");
            push_draft(
                fixture_id,
                &format!("{point_path}.t"),
                drafts,
                input_order,
                EventDraftBody {
                    simulated_time: Some(
                        required_str(fixture_id, point, &point_path, "t")?.to_string(),
                    ),
                    signal: SimulatedSignal::MetricPoint,
                    source_key: source_key.clone(),
                    record_kind: StoredRecordKind::MetricSeries,
                    payload: metric_point_payload(metric, point),
                },
            )?;
        }
    }

    Ok(())
}

fn append_id_records(
    fixture_id: &str,
    input: &Value,
    spec: IdEventSpec,
    input_order: &mut u64,
    drafts: &mut Vec<EventDraft>,
) -> Result<(), FixtureSimulationError> {
    let Some(records) = array_at(fixture_id, input, spec.key)? else {
        return Ok(());
    };

    for (index, record) in records.iter().enumerate() {
        let json_path = format!("$.{}[{index}]", spec.key);
        let id = required_str(fixture_id, record, &json_path, "id")?;
        push_draft(
            fixture_id,
            &format!("{json_path}.{}", spec.time_field),
            drafts,
            input_order,
            EventDraftBody {
                simulated_time: event_time(fixture_id, record, &json_path, &spec)?,
                signal: spec.signal,
                source_key: id.to_string(),
                record_kind: spec.record_kind,
                payload: record.clone(),
            },
        )?;
    }

    Ok(())
}

fn event_time(
    fixture_id: &str,
    value: &Value,
    json_path: &str,
    spec: &IdEventSpec,
) -> Result<Option<String>, FixtureSimulationError> {
    if spec.time_required {
        required_str(fixture_id, value, json_path, spec.time_field)
            .map(|timestamp| Some(timestamp.to_string()))
    } else {
        optional_str_field(fixture_id, value, json_path, spec.time_field)
            .map(|timestamp| timestamp.map(ToString::to_string))
    }
}

fn push_draft(
    fixture_id: &str,
    time_json_path: &str,
    drafts: &mut Vec<EventDraft>,
    input_order: &mut u64,
    body: EventDraftBody,
) -> Result<(), FixtureSimulationError> {
    let time_sort_key = body
        .simulated_time
        .as_deref()
        .map(|timestamp| fixture_timestamp_sort_key(fixture_id, timestamp, time_json_path))
        .transpose()?;

    drafts.push(EventDraft {
        input_order: *input_order,
        simulated_time: body.simulated_time,
        time_sort_key,
        signal: body.signal,
        source_key: body.source_key,
        record_kind: body.record_kind,
        payload: body.payload,
    });
    *input_order += 1;

    Ok(())
}

fn compare_drafts(left: &EventDraft, right: &EventDraft) -> std::cmp::Ordering {
    match (
        left.simulated_time.as_deref(),
        right.simulated_time.as_deref(),
    ) {
        (None, None) => left.input_order.cmp(&right.input_order),
        (None, Some(_)) => std::cmp::Ordering::Less,
        (Some(_), None) => std::cmp::Ordering::Greater,
        (Some(_), Some(_)) => left
            .time_sort_key
            .cmp(&right.time_sort_key)
            .then_with(|| left.input_order.cmp(&right.input_order)),
    }
}

fn trace_start_time(
    fixture_id: &str,
    trace: &Value,
    trace_path: &str,
) -> Result<Option<String>, FixtureSimulationError> {
    let Some(spans) = optional_array_field(fixture_id, trace, trace_path, "spans")? else {
        return optional_str_field(fixture_id, trace, trace_path, "start")
            .map(|value| value.map(ToString::to_string));
    };

    let mut starts = Vec::new();
    for (span_index, span) in spans.iter().enumerate() {
        let span_path = format!("{trace_path}.spans[{span_index}]");
        if let Some(start) = optional_str_field(fixture_id, span, &span_path, "start")? {
            starts.push((
                fixture_timestamp_sort_key(fixture_id, start, &format!("{span_path}.start"))?,
                start.to_string(),
            ));
        }
    }

    starts.sort();

    match starts.into_iter().next() {
        Some((_sort_key, start)) => Ok(Some(start)),
        None => optional_str_field(fixture_id, trace, trace_path, "start")
            .map(|value| value.map(ToString::to_string)),
    }
}

fn fixture_timestamp_sort_key(
    fixture_id: &str,
    timestamp: &str,
    json_path: &str,
) -> Result<FixtureTimestampSortKey, FixtureSimulationError> {
    let Some(trimmed) = timestamp.strip_suffix('Z') else {
        return Err(FixtureSimulationError::InvalidShape {
            fixture_id: fixture_id.to_string(),
            json_path: json_path.to_string(),
            message: "timestamp must use UTC Z suffix".to_string(),
        });
    };
    let (base, fraction) = trimmed.split_once('.').unwrap_or((trimmed, ""));
    if base.is_empty() || fraction.contains('.') {
        return Err(FixtureSimulationError::InvalidShape {
            fixture_id: fixture_id.to_string(),
            json_path: json_path.to_string(),
            message: "timestamp must be a UTC RFC3339-like string".to_string(),
        });
    }
    if timestamp.contains('.') && fraction.is_empty() {
        return Err(FixtureSimulationError::InvalidShape {
            fixture_id: fixture_id.to_string(),
            json_path: json_path.to_string(),
            message: "timestamp fractional seconds must not be empty".to_string(),
        });
    }
    if !fraction.chars().all(|character| character.is_ascii_digit()) {
        return Err(FixtureSimulationError::InvalidShape {
            fixture_id: fixture_id.to_string(),
            json_path: json_path.to_string(),
            message: "timestamp fractional seconds must be digits".to_string(),
        });
    }

    let mut nanos = fraction.to_string();
    nanos.truncate(9);
    while nanos.len() < 9 {
        nanos.push('0');
    }

    Ok(FixtureTimestampSortKey {
        base: base.to_string(),
        nanos: nanos.parse().unwrap_or(0),
    })
}

fn metric_point_payload(metric: &Value, point: &Value) -> Value {
    let mut payload = Map::new();

    if let Some(metric_object) = metric.as_object() {
        for (key, value) in metric_object {
            if key != "points" {
                payload.insert(key.clone(), value.clone());
            }
        }
    }

    payload.insert("point".to_string(), point.clone());
    Value::Object(payload)
}

fn array_at<'a>(
    fixture_id: &str,
    root: &'a Value,
    key: &'static str,
) -> Result<Option<&'a Vec<Value>>, FixtureSimulationError> {
    match root.get(key) {
        Some(Value::Array(values)) => Ok(Some(values)),
        Some(_) => Err(FixtureSimulationError::InvalidShape {
            fixture_id: fixture_id.to_string(),
            json_path: format!("$.{key}"),
            message: "must be an array".to_string(),
        }),
        None => Ok(None),
    }
}

fn optional_array_field<'a>(
    fixture_id: &str,
    value: &'a Value,
    json_path: &str,
    field: &'static str,
) -> Result<Option<&'a Vec<Value>>, FixtureSimulationError> {
    match value.get(field) {
        Some(Value::Array(values)) => Ok(Some(values)),
        Some(Value::Null) | None => Ok(None),
        Some(_) => Err(FixtureSimulationError::InvalidShape {
            fixture_id: fixture_id.to_string(),
            json_path: format!("{json_path}.{field}"),
            message: "must be an array".to_string(),
        }),
    }
}

fn required_str<'a>(
    fixture_id: &str,
    value: &'a Value,
    json_path: &str,
    field: &'static str,
) -> Result<&'a str, FixtureSimulationError> {
    match optional_str_field(fixture_id, value, json_path, field)? {
        Some(value) => Ok(value),
        None => Err(FixtureSimulationError::MissingField {
            fixture_id: fixture_id.to_string(),
            json_path: json_path.to_string(),
            field,
        }),
    }
}

fn optional_str_field<'a>(
    fixture_id: &str,
    value: &'a Value,
    json_path: &str,
    field: &'static str,
) -> Result<Option<&'a str>, FixtureSimulationError> {
    match value.get(field) {
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(Some(value)),
        Some(Value::String(_)) | Some(Value::Null) | None => Ok(None),
        Some(_) => Err(FixtureSimulationError::InvalidShape {
            fixture_id: fixture_id.to_string(),
            json_path: format!("{json_path}.{field}"),
            message: "must be a string".to_string(),
        }),
    }
}
