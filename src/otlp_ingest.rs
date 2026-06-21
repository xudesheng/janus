use crate::{
    evidence::{SourceRef, SourceSignal},
    hot_context_store::{
        HotContextStore, HotIngestEvent, IngestOutcome, MetricSeriesKey, SourceResolution,
        StoredRecordKind,
    },
};
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs, io,
    path::PathBuf,
};

const PROVENANCE_KEY: &str = "_janus.provenance";

#[derive(Debug)]
pub struct OtlpIngestResult {
    pub store: HotContextStore,
    pub summary: OtlpIngestSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OtlpIngestSummary {
    pub inputs: Vec<String>,
    pub accepted: SignalCounts,
    pub rejected: SignalCounts,
    pub inserted_records: usize,
    pub updated_records: usize,
    pub records_stored: usize,
    pub low_quality_entity_hints: usize,
    pub missing_entity_hints: usize,
    pub explicit_log_ids: usize,
    pub generated_log_ids: usize,
    pub duplicate_source_keys: usize,
    pub source_refs_resolved: usize,
    pub emitted_source_refs: Vec<String>,
    pub errors: Vec<OtlpIssue>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct SignalCounts {
    pub resources: usize,
    pub traces: usize,
    pub spans: usize,
    pub metric_points: usize,
    pub logs: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OtlpIssue {
    pub input: String,
    pub path: String,
    pub signal: Option<String>,
    pub message: String,
}

#[derive(Debug)]
pub enum OtlpIngestError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OtlpSignal {
    Resource,
    Trace,
    Span,
    MetricPoint,
    Log,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntityHintQuality {
    High,
    Low,
    Missing,
}

#[derive(Debug, Clone)]
struct ResourceContext {
    key: String,
    entity: String,
    quality: EntityHintQuality,
    signature: String,
    payload: Value,
}

#[derive(Debug)]
struct ResourceEvent {
    quality: EntityHintQuality,
    payload: Value,
}

#[derive(Debug)]
struct NormalizedEvent {
    signal: OtlpSignal,
    source_key: String,
    entity_hint_quality: Option<EntityHintQuality>,
    event: HotIngestEvent,
}

struct EventApplyState<'a> {
    store: &'a mut HotContextStore,
    accepted: &'a mut SignalCounts,
    rejected: &'a mut SignalCounts,
    issues: &'a mut Vec<OtlpIssue>,
    source_refs: &'a mut BTreeSet<String>,
    inserted_records: &'a mut usize,
    updated_records: &'a mut usize,
    duplicate_source_keys: &'a mut usize,
    low_quality_entity_hints: &'a mut usize,
    missing_entity_hints: &'a mut usize,
}

#[derive(Debug, Default)]
struct OtlpAdapter {
    resources: BTreeMap<String, ResourceEvent>,
    resource_signatures: BTreeMap<String, String>,
    traces: BTreeMap<String, TraceDraft>,
    events: Vec<NormalizedEvent>,
    issues: Vec<OtlpIssue>,
    accepted: SignalCounts,
    rejected: SignalCounts,
    low_quality_entity_hints: usize,
    missing_entity_hints: usize,
    explicit_log_ids: usize,
    generated_log_ids: usize,
    log_sequence: u64,
}

#[derive(Debug, Default)]
struct TraceDraft {
    spans: Vec<Value>,
    first_path: String,
}

#[derive(Debug)]
struct MetricPointSet<'a> {
    aggregation: &'static str,
    points: &'a Vec<Value>,
}

impl OtlpIngestSummary {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

pub fn ingest_otlp_json_files(paths: &[PathBuf]) -> Result<OtlpIngestResult, OtlpIngestError> {
    let mut inputs = Vec::new();
    let mut adapter = OtlpAdapter::default();

    for path in paths {
        let label = path.display().to_string();
        let bytes = fs::read(path).map_err(|source| OtlpIngestError::Io {
            path: path.clone(),
            source,
        })?;
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|source| OtlpIngestError::Json {
                path: path.clone(),
                source,
            })?;
        inputs.push(label.clone());
        adapter.collect_input(&label, &value);
    }

    Ok(adapter.finish(inputs))
}

pub fn ingest_otlp_json_value(input: impl Into<String>, value: &Value) -> OtlpIngestResult {
    let input = input.into();
    let mut adapter = OtlpAdapter::default();
    adapter.collect_input(&input, value);
    adapter.finish(vec![input])
}

pub fn format_text_summary(summary: &OtlpIngestSummary) -> String {
    let mut output = format!(
        "otlp ingest summary\n\
inputs: {}\n\
accepted resources: {}\n\
accepted traces: {}\n\
accepted spans: {}\n\
accepted metric points: {}\n\
accepted logs: {}\n\
records inserted: {}\n\
records updated: {}\n\
records stored: {}\n\
source refs resolved: {}\n\
explicit log ids: {}\n\
generated log ids: {}\n\
low-quality entity hints: {}\n\
missing entity hints: {}\n\
duplicate source keys: {}\n\
errors: {}\n",
        summary.inputs.len(),
        summary.accepted.resources,
        summary.accepted.traces,
        summary.accepted.spans,
        summary.accepted.metric_points,
        summary.accepted.logs,
        summary.inserted_records,
        summary.updated_records,
        summary.records_stored,
        summary.source_refs_resolved,
        summary.explicit_log_ids,
        summary.generated_log_ids,
        summary.low_quality_entity_hints,
        summary.missing_entity_hints,
        summary.duplicate_source_keys,
        summary.errors.len()
    );

    for error in &summary.errors {
        output.push_str(&format!("error: {error}\n"));
    }

    output
}

pub fn describe_scalar_resolution(resolution: SourceResolution<'_>) -> String {
    match resolution {
        SourceResolution::Found(record) => {
            format!("found {} {}", record.kind, record.key.as_str())
        }
        SourceResolution::Ambiguous {
            raw_ref,
            candidates,
        } => {
            format!("ambiguous {raw_ref}: {} candidate(s)", candidates.len())
        }
        SourceResolution::SignalMismatch {
            raw_ref,
            signal,
            candidates,
        } => format!(
            "signal mismatch {raw_ref}: signal {signal:?}, {} candidate(s)",
            candidates.len()
        ),
        SourceResolution::Missing { raw_ref } => format!("missing {raw_ref}"),
        SourceResolution::Unsupported { raw_ref, signal } => {
            format!("unsupported {raw_ref}: signal {signal:?}")
        }
    }
}

impl OtlpAdapter {
    fn collect_input(&mut self, input: &str, root: &Value) {
        let mut saw_supported = false;

        if root.get("resourceSpans").is_some() {
            saw_supported = true;
            self.collect_resource_spans(input, root);
        }
        if root.get("resourceMetrics").is_some() {
            saw_supported = true;
            self.collect_resource_metrics(input, root);
        }
        if root.get("resourceLogs").is_some() {
            saw_supported = true;
            self.collect_resource_logs(input, root);
        }

        if !saw_supported {
            self.issue(
                input,
                "$",
                None,
                "expected at least one of resourceSpans, resourceMetrics, or resourceLogs",
            );
        }
    }

    fn finish(mut self, inputs: Vec<String>) -> OtlpIngestResult {
        let mut store = HotContextStore::new();
        let mut source_refs = BTreeSet::new();
        let mut inserted_records = 0;
        let mut updated_records = 0;
        let mut duplicate_source_keys = 0;

        {
            let mut apply_state = EventApplyState {
                store: &mut store,
                accepted: &mut self.accepted,
                rejected: &mut self.rejected,
                issues: &mut self.issues,
                source_refs: &mut source_refs,
                inserted_records: &mut inserted_records,
                updated_records: &mut updated_records,
                duplicate_source_keys: &mut duplicate_source_keys,
                low_quality_entity_hints: &mut self.low_quality_entity_hints,
                missing_entity_hints: &mut self.missing_entity_hints,
            };

            for (key, payload) in self.resources {
                apply_state.apply(NormalizedEvent {
                    signal: OtlpSignal::Resource,
                    source_key: key,
                    entity_hint_quality: Some(payload.quality),
                    event: HotIngestEvent::Resource(payload.payload),
                });
            }

            for (trace_id, draft) in self.traces {
                let payload = json!({
                    "trace_id": trace_id,
                    "spans": draft.spans,
                    PROVENANCE_KEY: {
                        "source": "otlp-json",
                        "envelope_path": draft.first_path,
                    }
                });
                apply_state.apply(NormalizedEvent {
                    signal: OtlpSignal::Trace,
                    source_key: trace_id,
                    entity_hint_quality: None,
                    event: HotIngestEvent::Trace(payload),
                });
            }

            for event in self.events {
                apply_state.apply(event);
            }
        }

        let emitted_source_refs = source_refs.into_iter().collect::<Vec<_>>();
        let source_refs_resolved = emitted_source_refs
            .iter()
            .filter(|source_ref| {
                matches!(
                    store.resolve_scalar_ref(source_ref),
                    SourceResolution::Found(_)
                )
            })
            .count();

        let summary = OtlpIngestSummary {
            inputs,
            accepted: self.accepted,
            rejected: self.rejected,
            inserted_records,
            updated_records,
            records_stored: store.record_count(),
            low_quality_entity_hints: self.low_quality_entity_hints,
            missing_entity_hints: self.missing_entity_hints,
            explicit_log_ids: self.explicit_log_ids,
            generated_log_ids: self.generated_log_ids,
            duplicate_source_keys,
            source_refs_resolved,
            emitted_source_refs,
            errors: self.issues,
        };

        OtlpIngestResult { store, summary }
    }

    fn collect_resource_spans(&mut self, input: &str, root: &Value) {
        let Some(resource_spans) = self.array_field(input, root, "$", "resourceSpans", None) else {
            return;
        };

        for (resource_index, resource_span) in resource_spans.iter().enumerate() {
            let resource_path = format!("$.resourceSpans[{resource_index}].resource");
            let Some(resource) = resource_span.get("resource") else {
                self.issue(
                    input,
                    &resource_path,
                    Some(OtlpSignal::Resource),
                    "missing required field `resource`",
                );
                continue;
            };
            let Some(resource_context) =
                self.resource_context(input, resource, &resource_path, Some(OtlpSignal::Trace))
            else {
                continue;
            };
            self.push_resource(input, &resource_context, &resource_path);
            let scope_spans_path = format!("$.resourceSpans[{resource_index}]");
            let Some(scope_spans) = self.array_field(
                input,
                resource_span,
                &scope_spans_path,
                "scopeSpans",
                Some(OtlpSignal::Trace),
            ) else {
                continue;
            };

            for (scope_index, scope_span) in scope_spans.iter().enumerate() {
                let scope_path =
                    format!("$.resourceSpans[{resource_index}].scopeSpans[{scope_index}]");
                let scope = scope_value(scope_span);
                let Some(spans) = self.array_field(
                    input,
                    scope_span,
                    &scope_path,
                    "spans",
                    Some(OtlpSignal::Trace),
                ) else {
                    continue;
                };

                for (span_index, span) in spans.iter().enumerate() {
                    let span_path = format!("{scope_path}.spans[{span_index}]");
                    if let Some(span_payload) =
                        self.normalize_span(input, span, &span_path, &resource_context, &scope)
                    {
                        let trace_id = span_payload["trace_id"].as_str().unwrap().to_string();
                        let span_id = span_payload["span_id"].as_str().unwrap().to_string();
                        self.traces
                            .entry(trace_id.clone())
                            .and_modify(|draft| draft.spans.push(span_payload.clone()))
                            .or_insert_with(|| TraceDraft {
                                spans: vec![span_payload.clone()],
                                first_path: span_path.clone(),
                            });
                        self.events.push(NormalizedEvent {
                            signal: OtlpSignal::Span,
                            source_key: format!("{trace_id}/{span_id}"),
                            entity_hint_quality: Some(resource_context.quality),
                            event: HotIngestEvent::Span {
                                trace_id,
                                payload: span_payload,
                            },
                        });
                    }
                }
            }
        }
    }

    fn collect_resource_metrics(&mut self, input: &str, root: &Value) {
        let Some(resource_metrics) = self.array_field(input, root, "$", "resourceMetrics", None)
        else {
            return;
        };

        for (resource_index, resource_metric) in resource_metrics.iter().enumerate() {
            let resource_path = format!("$.resourceMetrics[{resource_index}].resource");
            let Some(resource) = resource_metric.get("resource") else {
                self.issue(
                    input,
                    &resource_path,
                    Some(OtlpSignal::Resource),
                    "missing required field `resource`",
                );
                continue;
            };
            let Some(resource_context) = self.resource_context(
                input,
                resource,
                &resource_path,
                Some(OtlpSignal::MetricPoint),
            ) else {
                continue;
            };
            self.push_resource(input, &resource_context, &resource_path);
            let scope_metrics_path = format!("$.resourceMetrics[{resource_index}]");
            let Some(scope_metrics) = self.array_field(
                input,
                resource_metric,
                &scope_metrics_path,
                "scopeMetrics",
                Some(OtlpSignal::MetricPoint),
            ) else {
                continue;
            };

            for (scope_index, scope_metric) in scope_metrics.iter().enumerate() {
                let scope_path =
                    format!("$.resourceMetrics[{resource_index}].scopeMetrics[{scope_index}]");
                let scope = scope_value(scope_metric);
                let Some(metrics) = self.array_field(
                    input,
                    scope_metric,
                    &scope_path,
                    "metrics",
                    Some(OtlpSignal::MetricPoint),
                ) else {
                    continue;
                };

                for (metric_index, metric) in metrics.iter().enumerate() {
                    let metric_path = format!("{scope_path}.metrics[{metric_index}]");
                    self.normalize_metric(input, metric, &metric_path, &resource_context, &scope);
                }
            }
        }
    }

    fn collect_resource_logs(&mut self, input: &str, root: &Value) {
        let Some(resource_logs) = self.array_field(input, root, "$", "resourceLogs", None) else {
            return;
        };

        for (resource_index, resource_log) in resource_logs.iter().enumerate() {
            let resource_path = format!("$.resourceLogs[{resource_index}].resource");
            let Some(resource) = resource_log.get("resource") else {
                self.issue(
                    input,
                    &resource_path,
                    Some(OtlpSignal::Resource),
                    "missing required field `resource`",
                );
                continue;
            };
            let Some(resource_context) =
                self.resource_context(input, resource, &resource_path, Some(OtlpSignal::Log))
            else {
                continue;
            };
            self.push_resource(input, &resource_context, &resource_path);
            let scope_logs_path = format!("$.resourceLogs[{resource_index}]");
            let Some(scope_logs) = self.array_field(
                input,
                resource_log,
                &scope_logs_path,
                "scopeLogs",
                Some(OtlpSignal::Log),
            ) else {
                continue;
            };

            for (scope_index, scope_log) in scope_logs.iter().enumerate() {
                let scope_path =
                    format!("$.resourceLogs[{resource_index}].scopeLogs[{scope_index}]");
                let scope = scope_value(scope_log);
                let Some(log_records) = self.array_field(
                    input,
                    scope_log,
                    &scope_path,
                    "logRecords",
                    Some(OtlpSignal::Log),
                ) else {
                    continue;
                };

                for (log_index, log_record) in log_records.iter().enumerate() {
                    let log_path = format!("{scope_path}.logRecords[{log_index}]");
                    if let Some(log_payload) =
                        self.normalize_log(input, log_record, &log_path, &resource_context, &scope)
                    {
                        let source_key = log_payload["id"].as_str().unwrap().to_string();
                        self.events.push(NormalizedEvent {
                            signal: OtlpSignal::Log,
                            source_key,
                            entity_hint_quality: Some(resource_context.quality),
                            event: HotIngestEvent::Log(log_payload),
                        });
                    }
                }
            }
        }
    }

    fn normalize_span(
        &mut self,
        input: &str,
        span: &Value,
        path: &str,
        resource: &ResourceContext,
        scope: &Value,
    ) -> Option<Value> {
        let trace_id = self.required_hex_id(input, span, path, "traceId", 32, OtlpSignal::Span)?;
        let span_id = self.required_hex_id(input, span, path, "spanId", 16, OtlpSignal::Span)?;
        let start = self.required_time(input, span, path, "startTimeUnixNano", OtlpSignal::Span)?;
        let end = self.required_time(input, span, path, "endTimeUnixNano", OtlpSignal::Span)?;
        let attributes = self.attributes(input, span, path, OtlpSignal::Span)?;
        let entity = self.entity_hint(resource);
        let mut payload = Map::new();

        payload.insert("trace_id".to_string(), Value::String(trace_id.clone()));
        payload.insert("span_id".to_string(), Value::String(span_id));
        if let Some(parent_span_id) =
            self.optional_hex_id(input, span, path, "parentSpanId", 16, OtlpSignal::Span)?
        {
            payload.insert("parent_span_id".to_string(), Value::String(parent_span_id));
        }
        insert_optional_string(&mut payload, span, "name");
        insert_optional_value(&mut payload, span, "kind");
        payload.insert("start".to_string(), Value::String(start));
        payload.insert("end".to_string(), Value::String(end));
        payload.insert("resource".to_string(), Value::String(resource.key.clone()));
        payload.insert("entity".to_string(), Value::String(entity));
        payload.insert(
            "entity_hint_quality".to_string(),
            Value::String(entity_quality_name(resource.quality).to_string()),
        );
        payload.insert("attributes".to_string(), Value::Object(attributes));
        payload.insert("instrumentation_scope".to_string(), scope.clone());
        insert_optional_value(&mut payload, span, "status");
        payload.insert(PROVENANCE_KEY.to_string(), provenance(input, path, "trace"));

        Some(Value::Object(payload))
    }

    fn normalize_metric(
        &mut self,
        input: &str,
        metric: &Value,
        path: &str,
        resource: &ResourceContext,
        scope: &Value,
    ) {
        let Some(name) = self.required_string(input, metric, path, "name", OtlpSignal::MetricPoint)
        else {
            return;
        };
        let Some(points) = self.metric_points(input, metric, path) else {
            return;
        };
        let entity = self.entity_hint(resource);
        let series = MetricSeriesKey::new(name.clone(), entity.clone());

        for (point_index, point) in points.points.iter().enumerate() {
            let point_path = format!("{path}.{}.dataPoints[{point_index}]", points.aggregation);
            let Some(time) = self.required_time(
                input,
                point,
                &point_path,
                "timeUnixNano",
                OtlpSignal::MetricPoint,
            ) else {
                continue;
            };
            let Some(value) = metric_point_value(point) else {
                self.issue(
                    input,
                    &point_path,
                    Some(OtlpSignal::MetricPoint),
                    "metric point must include asDouble or asInt",
                );
                continue;
            };
            let Some(point_attributes) =
                self.attributes(input, point, &point_path, OtlpSignal::MetricPoint)
            else {
                continue;
            };
            let mut point_payload = Map::new();
            point_payload.insert("t".to_string(), Value::String(time));
            point_payload.insert("v".to_string(), value);
            point_payload.insert("attributes".to_string(), Value::Object(point_attributes));
            insert_optional_value(&mut point_payload, point, "startTimeUnixNano");
            point_payload.insert(
                PROVENANCE_KEY.to_string(),
                provenance(input, &point_path, "metric"),
            );

            let mut payload = Map::new();
            payload.insert("name".to_string(), Value::String(name.clone()));
            payload.insert("entity".to_string(), Value::String(entity.clone()));
            payload.insert(
                "entity_hint_quality".to_string(),
                Value::String(entity_quality_name(resource.quality).to_string()),
            );
            insert_optional_string(&mut payload, metric, "unit");
            payload.insert(
                "aggregation".to_string(),
                Value::String(points.aggregation.to_string()),
            );
            payload.insert("resource".to_string(), Value::String(resource.key.clone()));
            payload.insert("instrumentation_scope".to_string(), scope.clone());
            payload.insert("point".to_string(), Value::Object(point_payload));

            self.events.push(NormalizedEvent {
                signal: OtlpSignal::MetricPoint,
                source_key: format!("{}@{}", name, entity),
                entity_hint_quality: Some(resource.quality),
                event: HotIngestEvent::MetricPoint {
                    series: series.clone(),
                    payload: Value::Object(payload),
                },
            });
        }
    }

    fn normalize_log(
        &mut self,
        input: &str,
        log: &Value,
        path: &str,
        resource: &ResourceContext,
        scope: &Value,
    ) -> Option<Value> {
        let time = match self.optional_time(log, "timeUnixNano") {
            Some(time) => time,
            None => {
                self.required_time(input, log, path, "observedTimeUnixNano", OtlpSignal::Log)?
            }
        };
        let attributes = self.attributes(input, log, path, OtlpSignal::Log)?;
        let trace_id = self.optional_hex_id(input, log, path, "traceId", 32, OtlpSignal::Log)?;
        let span_id = self.optional_hex_id(input, log, path, "spanId", 16, OtlpSignal::Log)?;
        let id = match attributes.get("janus.log.id").and_then(Value::as_str) {
            Some(id) if !id.trim().is_empty() => {
                self.explicit_log_ids += 1;
                id.to_string()
            }
            _ => {
                self.generated_log_ids += 1;
                let id = format!(
                    "log:{}:{}:{}:{}",
                    trace_id.as_deref().unwrap_or("none"),
                    span_id.as_deref().unwrap_or("none"),
                    time,
                    self.log_sequence
                );
                self.log_sequence += 1;
                id
            }
        };
        let entity = self.entity_hint(resource);
        let mut payload = Map::new();

        payload.insert("id".to_string(), Value::String(id));
        payload.insert("t".to_string(), Value::String(time));
        payload.insert("resource".to_string(), Value::String(resource.key.clone()));
        payload.insert("entity".to_string(), Value::String(entity));
        payload.insert(
            "entity_hint_quality".to_string(),
            Value::String(entity_quality_name(resource.quality).to_string()),
        );
        if let Some(trace_id) = trace_id {
            payload.insert("trace_id".to_string(), Value::String(trace_id));
        }
        if let Some(span_id) = span_id {
            payload.insert("span_id".to_string(), Value::String(span_id));
        }
        insert_optional_value(&mut payload, log, "severityText");
        insert_optional_value(&mut payload, log, "severityNumber");
        if let Some(body) = log.get("body") {
            payload.insert("body".to_string(), any_value_to_json(body));
        }
        payload.insert("attributes".to_string(), Value::Object(attributes));
        payload.insert("instrumentation_scope".to_string(), scope.clone());
        payload.insert(PROVENANCE_KEY.to_string(), provenance(input, path, "log"));

        Some(Value::Object(payload))
    }

    fn resource_context(
        &mut self,
        input: &str,
        resource: &Value,
        path: &str,
        signal: Option<OtlpSignal>,
    ) -> Option<ResourceContext> {
        let attributes = self.attributes(
            input,
            resource,
            path,
            signal.unwrap_or(OtlpSignal::Resource),
        )?;
        let service_name = attributes
            .get("service.name")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty());
        let service_instance = attributes
            .get("service.instance.id")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty());
        let (key, entity, quality) = match (service_name, service_instance) {
            (Some(service_name), Some(service_instance)) => (
                format!("resource:{service_name}@{service_instance}"),
                format!("service:{service_name}"),
                EntityHintQuality::High,
            ),
            (Some(service_name), None) => (
                format!("resource:{service_name}"),
                format!("service:{service_name}"),
                EntityHintQuality::High,
            ),
            (None, _) if attributes.is_empty() => {
                let key = format!("resource:attrs:{:016x}", stable_hash(path));
                (key.clone(), key, EntityHintQuality::Missing)
            }
            (None, _) => {
                let canonical = if attributes.is_empty() {
                    path.to_string()
                } else {
                    canonical_attributes(&attributes)
                };
                let key = format!("resource:attrs:{:016x}", stable_hash(&canonical));
                (key.clone(), key, EntityHintQuality::Low)
            }
        };
        let mut payload = Map::new();

        payload.insert("id".to_string(), Value::String(key.clone()));
        payload.insert("entity".to_string(), Value::String(entity.clone()));
        payload.insert(
            "entity_hint_quality".to_string(),
            Value::String(entity_quality_name(quality).to_string()),
        );
        payload.insert("attributes".to_string(), Value::Object(attributes.clone()));
        payload.insert(
            PROVENANCE_KEY.to_string(),
            provenance(input, path, "resource"),
        );

        Some(ResourceContext {
            key,
            entity,
            quality,
            signature: canonical_attributes(&attributes),
            payload: Value::Object(payload),
        })
    }

    fn push_resource(&mut self, input: &str, resource: &ResourceContext, path: &str) {
        let signature = &resource.signature;
        match self.resource_signatures.get(&resource.key) {
            Some(existing) if existing == signature => {}
            Some(_) => {
                self.issue(
                    input,
                    path,
                    Some(OtlpSignal::Resource),
                    "duplicate resource source key with different payload",
                );
            }
            None => {
                self.resource_signatures
                    .insert(resource.key.clone(), signature.clone());
                self.resources.insert(
                    resource.key.clone(),
                    ResourceEvent {
                        quality: resource.quality,
                        payload: resource.payload.clone(),
                    },
                );
            }
        }
    }

    fn entity_hint(&self, resource: &ResourceContext) -> String {
        resource.entity.clone()
    }

    fn metric_points<'a>(
        &mut self,
        input: &str,
        metric: &'a Value,
        path: &str,
    ) -> Option<MetricPointSet<'a>> {
        for aggregation in ["gauge", "sum"] {
            if let Some(container) = metric.get(aggregation) {
                let Some(points) = self.array_field(
                    input,
                    container,
                    &format!("{path}.{aggregation}"),
                    "dataPoints",
                    Some(OtlpSignal::MetricPoint),
                ) else {
                    continue;
                };
                return Some(MetricPointSet {
                    aggregation,
                    points,
                });
            }
        }

        self.issue(
            input,
            path,
            Some(OtlpSignal::MetricPoint),
            "unsupported metric shape; expected gauge.dataPoints or sum.dataPoints",
        );
        None
    }

    fn attributes(
        &mut self,
        input: &str,
        value: &Value,
        path: &str,
        signal: OtlpSignal,
    ) -> Option<Map<String, Value>> {
        let Some(attributes) = value.get("attributes") else {
            return Some(Map::new());
        };
        let Some(attributes) = attributes.as_array() else {
            self.issue(
                input,
                &format!("{path}.attributes"),
                Some(signal),
                "attributes must be an array",
            );
            return None;
        };
        let mut mapped = Map::new();

        for (index, attribute) in attributes.iter().enumerate() {
            let attribute_path = format!("{path}.attributes[{index}]");
            let Some(key) = self.required_string(input, attribute, &attribute_path, "key", signal)
            else {
                continue;
            };
            let Some(value) = attribute.get("value") else {
                self.issue(
                    input,
                    &attribute_path,
                    Some(signal),
                    "missing required field `value`",
                );
                continue;
            };

            mapped.insert(key, any_value_to_json(value));
        }

        Some(mapped)
    }

    fn array_field<'a>(
        &mut self,
        input: &str,
        value: &'a Value,
        path: &str,
        field: &'static str,
        signal: Option<OtlpSignal>,
    ) -> Option<&'a Vec<Value>> {
        match value.get(field) {
            Some(Value::Array(values)) => Some(values),
            Some(_) => {
                self.issue(
                    input,
                    &format!("{path}.{field}"),
                    signal,
                    "must be an array",
                );
                None
            }
            None => {
                self.issue(
                    input,
                    &format!("{path}.{field}"),
                    signal,
                    &format!("missing required field `{field}`"),
                );
                None
            }
        }
    }

    fn required_string(
        &mut self,
        input: &str,
        value: &Value,
        path: &str,
        field: &'static str,
        signal: OtlpSignal,
    ) -> Option<String> {
        match value.get(field) {
            Some(Value::String(value)) if !value.trim().is_empty() => Some(value.clone()),
            Some(Value::String(_)) | None => {
                self.issue(
                    input,
                    path,
                    Some(signal),
                    &format!("missing required field `{field}`"),
                );
                None
            }
            Some(_) => {
                self.issue(
                    input,
                    &format!("{path}.{field}"),
                    Some(signal),
                    "must be a string",
                );
                None
            }
        }
    }

    fn required_time(
        &mut self,
        input: &str,
        value: &Value,
        path: &str,
        field: &'static str,
        signal: OtlpSignal,
    ) -> Option<String> {
        match self.optional_time(value, field) {
            Some(value) => Some(value),
            None => {
                self.issue(
                    input,
                    path,
                    Some(signal),
                    &format!("missing required field `{field}`"),
                );
                None
            }
        }
    }

    fn optional_time(&self, value: &Value, field: &'static str) -> Option<String> {
        match value.get(field) {
            Some(Value::String(value)) if !value.trim().is_empty() => Some(value.clone()),
            Some(Value::Number(value)) => Some(value.to_string()),
            _ => None,
        }
    }

    fn required_hex_id(
        &mut self,
        input: &str,
        value: &Value,
        path: &str,
        field: &'static str,
        len: usize,
        signal: OtlpSignal,
    ) -> Option<String> {
        let id = self.required_string(input, value, path, field, signal)?;
        self.normalize_hex_id(input, &id, &format!("{path}.{field}"), len, signal)
    }

    fn optional_hex_id(
        &mut self,
        input: &str,
        value: &Value,
        path: &str,
        field: &'static str,
        len: usize,
        signal: OtlpSignal,
    ) -> Option<Option<String>> {
        match value.get(field) {
            Some(Value::String(value)) if !value.trim().is_empty() => self
                .normalize_hex_id(input, value, &format!("{path}.{field}"), len, signal)
                .map(Some),
            Some(Value::String(_)) | None => Some(None),
            Some(_) => {
                self.issue(
                    input,
                    &format!("{path}.{field}"),
                    Some(signal),
                    "must be a string",
                );
                None
            }
        }
    }

    fn normalize_hex_id(
        &mut self,
        input: &str,
        value: &str,
        path: &str,
        len: usize,
        signal: OtlpSignal,
    ) -> Option<String> {
        if value.len() == len && value.chars().all(|character| character.is_ascii_hexdigit()) {
            Some(value.to_ascii_lowercase())
        } else {
            self.issue(
                input,
                path,
                Some(signal),
                &format!("must be {len} hex characters"),
            );
            None
        }
    }

    fn issue(&mut self, input: &str, path: &str, signal: Option<OtlpSignal>, message: &str) {
        count_rejected(&mut self.rejected, signal);
        self.issues.push(OtlpIssue {
            input: input.to_string(),
            path: path.to_string(),
            signal: signal.map(|signal| signal.as_str().to_string()),
            message: message.to_string(),
        });
    }
}

impl OtlpSignal {
    fn as_str(self) -> &'static str {
        match self {
            OtlpSignal::Resource => "resource",
            OtlpSignal::Trace => "trace",
            OtlpSignal::Span => "span",
            OtlpSignal::MetricPoint => "metric_point",
            OtlpSignal::Log => "log",
        }
    }
}

impl fmt::Display for OtlpIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.signal {
            Some(signal) => write!(
                formatter,
                "{} {} {}: {}",
                self.input, signal, self.path, self.message
            ),
            None => write!(formatter, "{} {}: {}", self.input, self.path, self.message),
        }
    }
}

impl fmt::Display for OtlpIngestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OtlpIngestError::Io { path, source } => {
                write!(formatter, "failed to read {}: {source}", path.display())
            }
            OtlpIngestError::Json { path, source } => {
                write!(
                    formatter,
                    "failed to parse {} as JSON: {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for OtlpIngestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OtlpIngestError::Io { source, .. } => Some(source),
            OtlpIngestError::Json { source, .. } => Some(source),
        }
    }
}

impl EventApplyState<'_> {
    fn apply(&mut self, event: NormalizedEvent) {
        let signal = event.signal;
        let source_key = event.source_key;
        let entity_hint_quality = event.entity_hint_quality;

        match self.store.ingest(event.event) {
            Ok(IngestOutcome::Inserted { .. }) => {
                count_accepted(self.accepted, signal);
                count_entity_hint_quality(
                    self.low_quality_entity_hints,
                    self.missing_entity_hints,
                    entity_hint_quality,
                );
                *self.inserted_records += 1;
                self.source_refs.insert(source_key);
            }
            Ok(IngestOutcome::Updated { .. }) => {
                count_accepted(self.accepted, signal);
                *self.updated_records += 1;
                self.source_refs.insert(source_key);
            }
            Err(error) => {
                if matches!(
                    error,
                    crate::hot_context_store::HotStoreError::DuplicatePrimaryKey { .. }
                ) {
                    *self.duplicate_source_keys += 1;
                }
                count_rejected(self.rejected, Some(signal));
                self.issues.push(OtlpIssue {
                    input: "<hot-store>".to_string(),
                    path: "$".to_string(),
                    signal: Some(signal.as_str().to_string()),
                    message: error.to_string(),
                });
            }
        }
    }
}

fn count_entity_hint_quality(
    low_quality_entity_hints: &mut usize,
    missing_entity_hints: &mut usize,
    quality: Option<EntityHintQuality>,
) {
    match quality {
        Some(EntityHintQuality::Low) => *low_quality_entity_hints += 1,
        Some(EntityHintQuality::Missing) => *missing_entity_hints += 1,
        Some(EntityHintQuality::High) | None => {}
    }
}

fn count_accepted(counts: &mut SignalCounts, signal: OtlpSignal) {
    match signal {
        OtlpSignal::Resource => counts.resources += 1,
        OtlpSignal::Trace => counts.traces += 1,
        OtlpSignal::Span => counts.spans += 1,
        OtlpSignal::MetricPoint => counts.metric_points += 1,
        OtlpSignal::Log => counts.logs += 1,
    }
}

fn count_rejected(counts: &mut SignalCounts, signal: Option<OtlpSignal>) {
    match signal {
        Some(OtlpSignal::Resource) => counts.resources += 1,
        Some(OtlpSignal::Trace) => counts.traces += 1,
        Some(OtlpSignal::Span) => counts.spans += 1,
        Some(OtlpSignal::MetricPoint) => counts.metric_points += 1,
        Some(OtlpSignal::Log) => counts.logs += 1,
        None => {}
    }
}

fn any_value_to_json(value: &Value) -> Value {
    if let Some(value) = value.get("stringValue") {
        return value.clone();
    }
    if let Some(value) = value.get("boolValue") {
        return value.clone();
    }
    if let Some(value) = value.get("intValue") {
        return value.clone();
    }
    if let Some(value) = value.get("doubleValue") {
        return value.clone();
    }
    if let Some(value) = value.get("bytesValue") {
        return value.clone();
    }
    if let Some(values) = value
        .get("arrayValue")
        .and_then(|array| array.get("values"))
        .and_then(Value::as_array)
    {
        return Value::Array(values.iter().map(any_value_to_json).collect());
    }
    if let Some(values) = value
        .get("kvlistValue")
        .and_then(|list| list.get("values"))
        .and_then(Value::as_array)
    {
        let mut object = Map::new();
        for entry in values {
            if let (Some(key), Some(value)) =
                (entry.get("key").and_then(Value::as_str), entry.get("value"))
            {
                object.insert(key.to_string(), any_value_to_json(value));
            }
        }
        return Value::Object(object);
    }

    value.clone()
}

fn metric_point_value(point: &Value) -> Option<Value> {
    point
        .get("asDouble")
        .cloned()
        .or_else(|| point.get("asInt").cloned())
}

fn scope_value(value: &Value) -> Value {
    value.get("scope").cloned().unwrap_or_else(|| json!({}))
}

fn provenance(input: &str, path: &str, signal: &str) -> Value {
    json!({
        "source": "otlp-json",
        "input": input,
        "envelope_path": path,
        "signal": signal,
    })
}

fn insert_optional_string(payload: &mut Map<String, Value>, source: &Value, field: &'static str) {
    if let Some(value) = source.get(field).and_then(Value::as_str) {
        payload.insert(
            snake_case(field).to_string(),
            Value::String(value.to_string()),
        );
    }
}

fn insert_optional_value(payload: &mut Map<String, Value>, source: &Value, field: &'static str) {
    if let Some(value) = source.get(field) {
        payload.insert(snake_case(field).to_string(), value.clone());
    }
}

fn snake_case(field: &'static str) -> &'static str {
    match field {
        "parentSpanId" => "parent_span_id",
        "severityText" => "severity_text",
        "severityNumber" => "severity_number",
        "startTimeUnixNano" => "start_time_unix_nano",
        _ => field,
    }
}

fn canonical_attributes(attributes: &Map<String, Value>) -> String {
    let ordered = attributes
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<BTreeMap<_, _>>();

    serde_json::to_string(&ordered).unwrap_or_default()
}

fn stable_hash(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn entity_quality_name(quality: EntityHintQuality) -> &'static str {
    match quality {
        EntityHintQuality::High => "high",
        EntityHintQuality::Low => "low",
        EntityHintQuality::Missing => "missing",
    }
}

pub fn source_ref_for_key(kind: StoredRecordKind, key: impl Into<String>) -> Option<SourceRef> {
    let signal = match kind {
        StoredRecordKind::Trace | StoredRecordKind::Span => SourceSignal::Trace,
        StoredRecordKind::MetricSeries => SourceSignal::Metric,
        StoredRecordKind::Log => SourceSignal::Log,
        _ => return None,
    };

    Some(SourceRef {
        signal,
        r#ref: key.into(),
    })
}
