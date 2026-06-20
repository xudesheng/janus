use crate::{
    fixture_validation::FixtureCase,
    hot_context_store::StoredRecordKind,
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
struct EventDraft {
    input_order: u64,
    simulated_time: Option<String>,
    signal: SimulatedSignal,
    source_key: String,
    record_kind: StoredRecordKind,
    payload: Value,
}

#[derive(Debug, Clone, Copy)]
struct IdEventSpec {
    key: &'static str,
    signal: SimulatedSignal,
    record_kind: StoredRecordKind,
    time_field: &'static str,
}

pub fn plan_fixture_replay(
    case: &FixtureCase,
) -> Result<FixtureReplayPlan, FixtureSimulationError> {
    FixtureReplayPlan::build(case)
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
            drafts,
            input_order,
            None,
            SimulatedSignal::Resource,
            id.to_string(),
            StoredRecordKind::Resource,
            resource.clone(),
        );
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
            drafts,
            input_order,
            trace_time,
            SimulatedSignal::Trace,
            trace_id.to_string(),
            StoredRecordKind::Trace,
            trace.clone(),
        );

        let Some(spans) = optional_array_field(fixture_id, trace, &trace_path, "spans")? else {
            continue;
        };

        for (span_index, span) in spans.iter().enumerate() {
            let span_path = format!("{trace_path}.spans[{span_index}]");
            let span_id = required_str(fixture_id, span, &span_path, "span_id")?;

            push_draft(
                drafts,
                input_order,
                optional_str_field(fixture_id, span, &span_path, "start")?.map(ToString::to_string),
                SimulatedSignal::Span,
                span_ref(trace_id, span_id),
                StoredRecordKind::Span,
                span.clone(),
            );
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
                drafts,
                input_order,
                optional_str_field(fixture_id, point, &point_path, "t")?.map(ToString::to_string),
                SimulatedSignal::MetricPoint,
                source_key.clone(),
                StoredRecordKind::MetricSeries,
                metric_point_payload(metric, point),
            );
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
            drafts,
            input_order,
            optional_str_field(fixture_id, record, &json_path, spec.time_field)?
                .map(ToString::to_string),
            spec.signal,
            id.to_string(),
            spec.record_kind,
            record.clone(),
        );
    }

    Ok(())
}

fn push_draft(
    drafts: &mut Vec<EventDraft>,
    input_order: &mut u64,
    simulated_time: Option<String>,
    signal: SimulatedSignal,
    source_key: String,
    record_kind: StoredRecordKind,
    payload: Value,
) {
    drafts.push(EventDraft {
        input_order: *input_order,
        simulated_time,
        signal,
        source_key,
        record_kind,
        payload,
    });
    *input_order += 1;
}

fn compare_drafts(left: &EventDraft, right: &EventDraft) -> std::cmp::Ordering {
    match (
        left.simulated_time.as_deref(),
        right.simulated_time.as_deref(),
    ) {
        (None, None) => left.input_order.cmp(&right.input_order),
        (None, Some(_)) => std::cmp::Ordering::Less,
        (Some(_), None) => std::cmp::Ordering::Greater,
        (Some(left_time), Some(right_time)) => left_time
            .cmp(right_time)
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
            starts.push(start.to_string());
        }
    }

    starts.sort();

    Ok(starts.into_iter().next())
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
