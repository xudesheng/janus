use crate::evidence::SourceSignal;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RefCategory {
    Resource,
    Trace,
    Span,
    Metric,
    Log,
    Change,
    PriorIncident,
    TelemetryGap,
    Entity,
    Relationship,
    AnomalyWindow,
    LogPattern,
    EvidenceItem,
}

#[derive(Debug, Clone, Default)]
pub struct ReferenceIndex {
    refs: BTreeMap<String, BTreeSet<RefCategory>>,
}

impl ReferenceIndex {
    pub fn build(input: &Value, expected: &Value) -> Self {
        let mut index = Self::default();
        index.add_input_refs(input);
        index.add_expected_refs(expected);
        index
    }

    pub fn resolve(&self, raw_ref: &str) -> Option<&BTreeSet<RefCategory>> {
        resolve_ref_map(&self.refs, raw_ref)
    }

    pub fn add(&mut self, raw_ref: impl Into<String>, category: RefCategory) {
        let raw_ref = raw_ref.into();
        if raw_ref.trim().is_empty() {
            return;
        }

        self.refs.entry(raw_ref).or_default().insert(category);
    }

    fn add_input_refs(&mut self, input: &Value) {
        add_ids_from_array(self, input, "resources", RefCategory::Resource);
        add_ids_from_array(self, input, "logs", RefCategory::Log);
        add_ids_from_array(self, input, "changes", RefCategory::Change);
        add_ids_from_array(self, input, "prior_incidents", RefCategory::PriorIncident);
        add_ids_from_array(self, input, "telemetry_gaps", RefCategory::TelemetryGap);

        if let Some(traces) = array_at(input, "traces") {
            for trace in traces {
                if let Some(trace_id) = trace.get("trace_id").and_then(Value::as_str) {
                    self.add(trace_id, RefCategory::Trace);

                    if let Some(spans) = trace.get("spans").and_then(Value::as_array) {
                        for span in spans {
                            if let Some(span_id) = span.get("span_id").and_then(Value::as_str) {
                                self.add(span_ref(trace_id, span_id), RefCategory::Span);
                            }
                        }
                    }
                }
            }
        }

        if let Some(metrics) = array_at(input, "metrics") {
            for metric in metrics {
                if let (Some(name), Some(entity)) = (
                    metric.get("name").and_then(Value::as_str),
                    metric.get("entity").and_then(Value::as_str),
                ) {
                    self.add(metric_series_ref(name, entity), RefCategory::Metric);
                }

                if let Some(gap_ref) = metric
                    .get("_gap")
                    .and_then(|gap| gap.get("ref"))
                    .and_then(Value::as_str)
                {
                    self.add(gap_ref, RefCategory::TelemetryGap);
                }
            }
        }
    }

    fn add_expected_refs(&mut self, expected: &Value) {
        add_ids_from_array(self, expected, "entities", RefCategory::Entity);
        add_ids_from_array(self, expected, "relationships", RefCategory::Relationship);
        add_ids_from_array(
            self,
            expected,
            "anomaly_windows",
            RefCategory::AnomalyWindow,
        );
        add_ids_from_array(self, expected, "log_patterns", RefCategory::LogPattern);

        if let Some(items) = expected
            .pointer("/evidence_bundle/items")
            .and_then(Value::as_array)
        {
            for item in items {
                if let Some(id) = item.get("id").and_then(Value::as_str) {
                    self.add(id, RefCategory::EvidenceItem);
                }
            }
        }
    }
}

pub fn resolve_ref_map<'a, T>(refs: &'a BTreeMap<String, T>, raw_ref: &str) -> Option<&'a T> {
    if let Some(value) = refs.get(raw_ref) {
        return Some(value);
    }

    raw_ref
        .strip_prefix("trace:")
        .and_then(|stripped| refs.get(stripped))
}

pub fn span_ref(trace_id: &str, span_id: &str) -> String {
    format!("{trace_id}/{span_id}")
}

pub fn metric_series_ref(name: &str, entity: &str) -> String {
    format!("{name}@{entity}")
}

pub fn categories_for_signal(signal: SourceSignal) -> &'static [RefCategory] {
    match signal {
        SourceSignal::Trace => &[RefCategory::Trace, RefCategory::Span],
        SourceSignal::Metric => &[RefCategory::Metric],
        SourceSignal::Log => &[RefCategory::Log],
        SourceSignal::Change => &[RefCategory::Change],
        SourceSignal::Profile => &[],
        SourceSignal::AnomalyWindow => &[RefCategory::AnomalyWindow],
        SourceSignal::LogPattern => &[RefCategory::LogPattern],
        SourceSignal::PriorIncident => &[RefCategory::PriorIncident],
        SourceSignal::TelemetryGap => &[RefCategory::TelemetryGap],
        SourceSignal::Entity => &[RefCategory::Entity],
        SourceSignal::Relationship => &[RefCategory::Relationship],
        SourceSignal::External => &[],
    }
}

pub fn source_signal_name(signal: SourceSignal) -> &'static str {
    match signal {
        SourceSignal::Trace => "trace",
        SourceSignal::Metric => "metric",
        SourceSignal::Log => "log",
        SourceSignal::Change => "change",
        SourceSignal::Profile => "profile",
        SourceSignal::AnomalyWindow => "anomaly_window",
        SourceSignal::LogPattern => "log_pattern",
        SourceSignal::PriorIncident => "prior_incident",
        SourceSignal::TelemetryGap => "telemetry_gap",
        SourceSignal::Entity => "entity",
        SourceSignal::Relationship => "relationship",
        SourceSignal::External => "external",
    }
}

pub fn display_categories<'a>(categories: impl IntoIterator<Item = &'a RefCategory>) -> String {
    categories
        .into_iter()
        .map(|category| match category {
            RefCategory::Resource => "resource",
            RefCategory::Trace => "trace",
            RefCategory::Span => "span",
            RefCategory::Metric => "metric",
            RefCategory::Log => "log",
            RefCategory::Change => "change",
            RefCategory::PriorIncident => "prior_incident",
            RefCategory::TelemetryGap => "telemetry_gap",
            RefCategory::Entity => "entity",
            RefCategory::Relationship => "relationship",
            RefCategory::AnomalyWindow => "anomaly_window",
            RefCategory::LogPattern => "log_pattern",
            RefCategory::EvidenceItem => "evidence_item",
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn array_at<'a>(value: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
    value.get(key).and_then(Value::as_array)
}

fn add_ids_from_array(index: &mut ReferenceIndex, value: &Value, key: &str, category: RefCategory) {
    if let Some(values) = array_at(value, key) {
        for value in values {
            if let Some(id) = value.get("id").and_then(Value::as_str) {
                index.add(id, category);
            }
        }
    }
}
