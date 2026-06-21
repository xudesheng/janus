use crate::{
    evidence::{TimeWindow, UnitInterval},
    fixture_validation::FixtureCase,
    hot_context_store::{
        HotContextStore, HotStoreError, SourceKey, StoredRecord, StoredRecordKind,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

pub const DERIVED_CONTEXT_RELATIVE_NUMERIC_TOLERANCE: f64 = 0.05;
pub const DERIVED_CONTEXT_ABSOLUTE_NUMERIC_TOLERANCE_FLOOR: f64 = 0.001;
pub const DERIVED_CONTEXT_UNIT_INTERVAL_TOLERANCE: f64 = 0.05;

const DROP_ANOMALY_RATIO_THRESHOLD: f64 = 0.75;
const ERROR_RATE_ABSOLUTE_DELTA_THRESHOLD: f64 = 0.015;
const ERROR_RATE_RELATIVE_THRESHOLD: f64 = 4.0;
const LOCK_WAIT_ANOMALOUS_MIN: f64 = 5.0;
const RATE_ABSOLUTE_DELTA_THRESHOLD: f64 = 100.0;
const RATE_RELATIVE_THRESHOLD: f64 = 3.0;
const GENERIC_RELATIVE_INCREASE_THRESHOLD: f64 = 3.0;
const SAWTOOTH_HIGH_RATIO: f64 = 1.8;
const SAWTOOTH_RESET_RATIO: f64 = 0.4;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DerivedContext {
    pub anomaly_windows: Vec<DerivedAnomalyWindow>,
    pub log_patterns: Vec<DerivedLogPattern>,
    pub timeline: Vec<DerivedTimelineEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_anomalies: Option<DerivedRelatedAnomalies>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_comparison: Option<WindowComparison>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedAnomalyWindow {
    pub id: String,
    pub entity: String,
    pub signal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trough: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak_observed: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    pub detector_confidence: UnitInterval,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedLogPattern {
    pub id: String,
    pub template: String,
    pub entity: String,
    pub severity: String,
    pub first_seen: String,
    pub last_seen: String,
    pub count: usize,
    pub exemplars: Vec<String>,
    pub stability: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedTimelineEvent {
    pub t: String,
    pub marker: TimelineMarker,
    pub entity: String,
    pub text: String,
    pub source_ref: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TimelineMarker {
    Change,
    Symptom,
    Propagation,
    Recovery,
    Trigger,
    Amplification,
    NonCausalChange,
    DataGap,
}

impl TimelineMarker {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Change => "change",
            Self::Symptom => "symptom",
            Self::Propagation => "propagation",
            Self::Recovery => "recovery",
            Self::Trigger => "trigger",
            Self::Amplification => "amplification",
            Self::NonCausalChange => "non-causal-change",
            Self::DataGap => "data-gap",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedRelatedAnomalies {
    #[serde(rename = "_for_capability", skip_serializing_if = "Option::is_none")]
    pub for_capability: Option<String>,
    pub seed: String,
    pub related: Vec<RelatedAnomaly>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelatedAnomaly {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<String>,
    pub relation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lag_seconds: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prior_incident: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity: Option<UnitInterval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowComparison {
    #[serde(rename = "_for_capability", skip_serializing_if = "Option::is_none")]
    pub for_capability: Option<String>,
    pub healthy: TimeWindow,
    pub anomalous: TimeWindow,
    pub deltas: Vec<WindowDelta>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowDelta {
    pub entity: String,
    pub signal: String,
    pub from: f64,
    pub to: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub factor: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DerivedContextComparison {
    pub missing_anomaly_windows: Vec<String>,
    pub extra_anomaly_windows: Vec<String>,
    pub anomaly_window_mismatches: Vec<DerivedFieldMismatch>,
    pub missing_log_patterns: Vec<String>,
    pub extra_log_patterns: Vec<String>,
    pub log_pattern_mismatches: Vec<DerivedFieldMismatch>,
    pub missing_timeline_events: Vec<TimelineIdentity>,
    pub extra_timeline_events: Vec<TimelineIdentity>,
    pub timeline_order_mismatches: Vec<TimelineOrderMismatch>,
    pub timeline_mismatches: Vec<DerivedFieldMismatch>,
    pub missing_related_anomalies: Vec<RelatedAnomalyIdentity>,
    pub extra_related_anomalies: Vec<RelatedAnomalyIdentity>,
    pub related_anomaly_mismatches: Vec<DerivedFieldMismatch>,
    pub missing_window_comparison: bool,
    pub extra_window_comparison: bool,
    pub missing_window_deltas: Vec<WindowDeltaIdentity>,
    pub extra_window_deltas: Vec<WindowDeltaIdentity>,
    pub window_comparison_mismatches: Vec<DerivedFieldMismatch>,
    pub missing_runtime_provenance: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DerivedContextComparisonOptions {
    pub require_runtime_provenance: bool,
}

impl DerivedContextComparison {
    pub fn has_expected_mismatches(&self) -> bool {
        !self.missing_anomaly_windows.is_empty()
            || !self.anomaly_window_mismatches.is_empty()
            || !self.missing_log_patterns.is_empty()
            || !self.log_pattern_mismatches.is_empty()
            || !self.missing_timeline_events.is_empty()
            || !self.timeline_order_mismatches.is_empty()
            || !self.timeline_mismatches.is_empty()
            || !self.missing_related_anomalies.is_empty()
            || !self.related_anomaly_mismatches.is_empty()
            || self.missing_window_comparison
            || !self.missing_window_deltas.is_empty()
            || !self.window_comparison_mismatches.is_empty()
            || !self.missing_runtime_provenance.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DerivedFieldMismatch {
    pub artifact: String,
    pub field: String,
    pub expected: Value,
    pub actual: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineOrderMismatch {
    pub index: usize,
    pub expected: Option<TimelineIdentity>,
    pub actual: Option<TimelineIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimelineIdentity {
    pub t: String,
    pub marker: TimelineMarker,
    pub entity: String,
    pub source_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RelatedAnomalyIdentity {
    pub window: Option<String>,
    pub prior_incident: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WindowDeltaIdentity {
    pub entity: String,
    pub signal: String,
}

#[derive(Debug)]
pub struct DerivedContextGoldError {
    pub fixture_id: String,
    pub artifact: &'static str,
    pub source: serde_json::Error,
}

pub fn load_expected_derived_context(
    case: &FixtureCase,
) -> Result<DerivedContext, DerivedContextGoldError> {
    Ok(DerivedContext {
        anomaly_windows: parse_expected_artifact(
            case,
            "anomaly_windows",
            case.expected.get("anomaly_windows"),
        )?
        .unwrap_or_default(),
        log_patterns: parse_expected_artifact(
            case,
            "log_patterns",
            case.expected.get("log_patterns"),
        )?
        .unwrap_or_default(),
        timeline: parse_expected_artifact(case, "timeline", case.expected.get("timeline"))?
            .unwrap_or_default(),
        related_anomalies: parse_expected_artifact(
            case,
            "related_anomalies",
            case.expected.get("related_anomalies"),
        )?,
        window_comparison: parse_expected_artifact(
            case,
            "window_comparison",
            case.expected.get("window_comparison"),
        )?,
    })
}

pub fn derive_metric_context(case: &FixtureCase, store: &HotContextStore) -> DerivedContext {
    let metric_series = canonical_metric_series(store);
    let anomaly_windows = derive_anomaly_windows(case, &metric_series);
    let window_comparison = derive_window_comparison(case, &metric_series, &anomaly_windows);

    DerivedContext {
        anomaly_windows,
        window_comparison,
        ..Default::default()
    }
}

fn derive_anomaly_windows(
    case: &FixtureCase,
    metric_series: &[CanonicalMetricSeries],
) -> Vec<DerivedAnomalyWindow> {
    let scenario_window = scenario_time_window(case);
    let mut windows = metric_series
        .iter()
        .filter_map(|series| derive_anomaly_window(case, scenario_window.as_ref(), series))
        .collect::<Vec<_>>();

    windows.sort_by_key(|draft| {
        (
            anomaly_window_order(case, &draft.window.signal),
            draft.input_order,
            draft.window.entity.clone(),
            draft.window.signal.clone(),
        )
    });

    for (index, draft) in windows.iter_mut().enumerate() {
        draft.window.id = format!("aw-{}", index + 1);
    }

    windows.into_iter().map(|draft| draft.window).collect()
}

fn derive_window_comparison(
    case: &FixtureCase,
    metric_series: &[CanonicalMetricSeries],
    anomaly_windows: &[DerivedAnomalyWindow],
) -> Option<WindowComparison> {
    if !case
        .registry_entry
        .capabilities
        .iter()
        .any(|capability| capability == "compare_windows")
    {
        return None;
    }

    let scenario_window = scenario_time_window(case)?;
    let onset = comparison_onset(case, anomaly_windows)?;
    let anomalous_start = comparison_anomalous_start(case, &onset, metric_series)?;
    let anomalous_end = comparison_anomalous_end(case, &onset, &anomalous_start, metric_series)?;
    let healthy_end = shift_minutes(&onset, -1).unwrap_or_else(|| onset.clone());
    let healthy = TimeWindow {
        start: scenario_window.start,
        end: healthy_end,
    };
    let anomalous = TimeWindow {
        start: anomalous_start,
        end: anomalous_end,
    };

    let mut deltas = metric_series
        .iter()
        .filter(|series| include_window_delta(case, series))
        .filter_map(|series| window_delta_for_series(case, series, &healthy, &anomalous))
        .collect::<Vec<_>>();

    deltas.sort_by_key(|delta| {
        (
            window_delta_order(case, &delta.signal, &delta.entity),
            delta.entity.clone(),
            delta.signal.clone(),
        )
    });

    if deltas.is_empty() {
        return None;
    }

    let source_refs = dedupe_stable(
        deltas
            .iter()
            .flat_map(|delta| delta.source_refs.iter().cloned())
            .collect(),
    );

    Some(WindowComparison {
        for_capability: None,
        healthy,
        anomalous,
        deltas,
        source_refs,
    })
}

pub fn compare_derived_context(
    expected: &DerivedContext,
    actual: &DerivedContext,
) -> DerivedContextComparison {
    compare_derived_context_with_options(
        expected,
        actual,
        DerivedContextComparisonOptions::default(),
    )
}

pub fn compare_derived_context_with_options(
    expected: &DerivedContext,
    actual: &DerivedContext,
    options: DerivedContextComparisonOptions,
) -> DerivedContextComparison {
    let mut comparison = DerivedContextComparison::default();

    compare_anomaly_windows(expected, actual, &mut comparison);
    compare_log_patterns(expected, actual, &mut comparison);
    compare_timeline(expected, actual, &mut comparison);
    compare_related_anomalies(
        expected.related_anomalies.as_ref(),
        actual.related_anomalies.as_ref(),
        &mut comparison,
    );
    compare_window_comparison(
        expected.window_comparison.as_ref(),
        actual.window_comparison.as_ref(),
        &mut comparison,
    );

    if options.require_runtime_provenance {
        validate_runtime_provenance(actual, &mut comparison);
    }

    comparison
}

pub fn insert_derived_context(
    store: &mut HotContextStore,
    context: &DerivedContext,
) -> Result<(), HotStoreError> {
    for window in &context.anomaly_windows {
        store.insert_record(StoredRecord {
            key: SourceKey::new(window.id.clone()),
            kind: StoredRecordKind::AnomalyWindow,
            time_window: optional_time_window(window.start.as_deref(), window.end.as_deref()),
            entities: vec![window.entity.clone()],
            payload: serde_json::to_value(window).expect("derived anomaly window should serialize"),
        })?;
    }

    for pattern in &context.log_patterns {
        store.insert_record(StoredRecord {
            key: SourceKey::new(pattern.id.clone()),
            kind: StoredRecordKind::LogPattern,
            time_window: Some(TimeWindow {
                start: pattern.first_seen.clone(),
                end: pattern.last_seen.clone(),
            }),
            entities: vec![pattern.entity.clone()],
            payload: serde_json::to_value(pattern).expect("derived log pattern should serialize"),
        })?;
    }

    for event in &context.timeline {
        store.insert_record(StoredRecord {
            key: SourceKey::new(timeline_event_store_key(event)),
            kind: StoredRecordKind::TimelineEvent,
            time_window: Some(TimeWindow {
                start: event.t.clone(),
                end: event.t.clone(),
            }),
            entities: vec![event.entity.clone()],
            payload: serde_json::to_value(event).expect("derived timeline event should serialize"),
        })?;
    }

    if let Some(related) = &context.related_anomalies {
        store.insert_record(StoredRecord {
            key: SourceKey::new(related_anomalies_store_key(&related.seed)),
            kind: StoredRecordKind::RelatedAnomaly,
            time_window: None,
            entities: Vec::new(),
            payload: serde_json::to_value(related)
                .expect("derived related anomalies should serialize"),
        })?;
    }

    if let Some(comparison) = &context.window_comparison {
        store.insert_record(StoredRecord {
            key: SourceKey::new(window_comparison_store_key(comparison)),
            kind: StoredRecordKind::WindowComparison,
            // Fixture timestamps are zero-padded UTC RFC3339 strings, so lexical min/max matches time order.
            time_window: Some(TimeWindow {
                start: comparison
                    .healthy
                    .start
                    .clone()
                    .min(comparison.anomalous.start.clone()),
                end: comparison
                    .healthy
                    .end
                    .clone()
                    .max(comparison.anomalous.end.clone()),
            }),
            entities: window_comparison_entities(comparison),
            payload: serde_json::to_value(comparison)
                .expect("derived window comparison should serialize"),
        })?;
    }

    Ok(())
}

pub fn timeline_event_store_key(event: &DerivedTimelineEvent) -> String {
    format!(
        "timeline:{}|{}|{}|{}",
        event.t,
        event.marker.as_str(),
        event.entity,
        event.source_ref
    )
}

pub fn related_anomalies_store_key(seed: &str) -> String {
    format!("related_anomalies:{seed}")
}

pub fn window_comparison_store_key(comparison: &WindowComparison) -> String {
    format!(
        "window_comparison:{}..{}|{}..{}",
        comparison.healthy.start,
        comparison.healthy.end,
        comparison.anomalous.start,
        comparison.anomalous.end
    )
}

fn parse_expected_artifact<T>(
    case: &FixtureCase,
    artifact: &'static str,
    value: Option<&Value>,
) -> Result<Option<T>, DerivedContextGoldError>
where
    T: for<'de> Deserialize<'de>,
{
    value
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|source| DerivedContextGoldError {
            fixture_id: case.registry_entry.id.clone(),
            artifact,
            source,
        })
}

#[derive(Debug, Clone)]
struct CanonicalMetricSeries {
    name: String,
    entity: String,
    source_refs: Vec<String>,
    points: Vec<MetricPoint>,
    gap: Option<MetricGap>,
    input_order: usize,
}

#[derive(Debug, Clone)]
struct MetricPoint {
    t: String,
    v: f64,
}

#[derive(Debug, Clone)]
struct MetricGap {
    start: String,
    end: String,
    reference: String,
}

#[derive(Debug, Clone)]
struct AnomalyWindowDraft {
    input_order: usize,
    window: DerivedAnomalyWindow,
}

fn canonical_metric_series(store: &HotContextStore) -> Vec<CanonicalMetricSeries> {
    let instance_entities = instance_entity_map(store);
    let mut by_identity: BTreeMap<(String, String), CanonicalMetricSeries> = BTreeMap::new();
    let mut metric_order = 0;

    for record in store.raw_source_records() {
        if record.kind != StoredRecordKind::MetricSeries {
            continue;
        }

        let Some(mut series) = metric_series_from_record(record, metric_order, &instance_entities)
        else {
            metric_order += 1;
            continue;
        };
        metric_order += 1;

        let key = (series.name.clone(), series.entity.clone());
        if let Some(existing) = by_identity.get_mut(&key) {
            existing.input_order = existing.input_order.min(series.input_order);
            existing.source_refs.append(&mut series.source_refs);
            existing.source_refs = dedupe_stable(existing.source_refs.clone());
            existing.points = merge_points_by_max(&existing.points, &series.points);
            if existing.gap.is_none() {
                existing.gap = series.gap;
            }
        } else {
            by_identity.insert(key, series);
        }
    }

    by_identity.into_values().collect()
}

fn metric_series_from_record(
    record: &StoredRecord,
    input_order: usize,
    instance_entities: &BTreeMap<String, String>,
) -> Option<CanonicalMetricSeries> {
    let name = record.payload.get("name")?.as_str()?.to_string();
    let raw_entity = record.payload.get("entity")?.as_str()?;
    let entity = instance_entities
        .get(raw_entity)
        .cloned()
        .unwrap_or_else(|| raw_entity.to_string());
    let points = record
        .payload
        .get("points")?
        .as_array()?
        .iter()
        .filter_map(metric_point_from_value)
        .collect::<Vec<_>>();
    let gap = record.payload.get("_gap").and_then(metric_gap_from_value);

    if points.is_empty() {
        return None;
    }

    Some(CanonicalMetricSeries {
        name,
        entity,
        source_refs: vec![record.key.as_str().to_string()],
        points,
        gap,
        input_order,
    })
}

fn metric_point_from_value(value: &Value) -> Option<MetricPoint> {
    Some(MetricPoint {
        t: value.get("t")?.as_str()?.to_string(),
        v: value.get("v")?.as_f64()?,
    })
}

fn metric_gap_from_value(value: &Value) -> Option<MetricGap> {
    Some(MetricGap {
        start: value.get("start")?.as_str()?.to_string(),
        end: value.get("end")?.as_str()?.to_string(),
        reference: value.get("ref")?.as_str()?.to_string(),
    })
}

fn instance_entity_map(store: &HotContextStore) -> BTreeMap<String, String> {
    let mut service_instance_counts = BTreeMap::<String, usize>::new();
    let mut resources = Vec::new();

    for record in store.raw_source_records() {
        if record.kind != StoredRecordKind::Resource {
            continue;
        }

        let Some(attributes) = record.payload.get("attributes").and_then(Value::as_object) else {
            continue;
        };
        let Some(service_name) = attributes.get("service.name").and_then(Value::as_str) else {
            continue;
        };
        let Some(instance_id) = attributes
            .get("service.instance.id")
            .and_then(Value::as_str)
        else {
            continue;
        };

        *service_instance_counts
            .entry(service_name.to_string())
            .or_default() += 1;
        resources.push((
            service_name.to_string(),
            instance_id.to_string(),
            attributes.clone(),
        ));
    }

    let mut map = BTreeMap::new();
    for (service_name, instance_id, attributes) in resources {
        let instance_entity = format!("instance:{instance_id}");
        let service_entity = if attributes
            .get("rollout")
            .and_then(Value::as_str)
            .is_some_and(|rollout| rollout == "canary")
        {
            format!("service:{service_name}@canary")
        } else if service_instance_counts
            .get(&service_name)
            .is_some_and(|count| *count > 1)
            && attributes.get("service.version").is_some()
        {
            format!("service:{service_name}@stable")
        } else {
            continue;
        };

        map.insert(instance_entity, service_entity);
    }

    map
}

fn merge_points_by_max(left: &[MetricPoint], right: &[MetricPoint]) -> Vec<MetricPoint> {
    let mut by_time = BTreeMap::<String, f64>::new();

    for point in left.iter().chain(right.iter()) {
        by_time
            .entry(point.t.clone())
            .and_modify(|value| *value = value.max(point.v))
            .or_insert(point.v);
    }

    by_time
        .into_iter()
        .map(|(t, v)| MetricPoint { t, v })
        .collect()
}

fn derive_anomaly_window(
    case: &FixtureCase,
    scenario_window: Option<&TimeWindow>,
    series: &CanonicalMetricSeries,
) -> Option<AnomalyWindowDraft> {
    if !include_anomaly_series(case, series) {
        return None;
    }

    let first_anomaly = first_anomalous_time(series);
    if first_anomaly.is_none() && !include_non_anomalous_window(case, series) {
        return None;
    }

    let start = first_anomaly
        .as_deref()
        .and_then(|first| anomaly_start(case, scenario_window, series, first));
    let end = first_anomaly
        .as_deref()
        .and_then(|first| anomaly_end(case, scenario_window, series, first));
    let baseline = anomaly_baseline(case, series, start.as_deref());
    let peak = max_point_value(series);
    let trough = min_point_value(series);
    let mut source_refs = series.source_refs.clone();

    if let Some(gap) = &series.gap {
        source_refs.push(gap.reference.clone());
    }

    let mut window = DerivedAnomalyWindow {
        id: String::new(),
        entity: series.entity.clone(),
        signal: series.name.clone(),
        start,
        end,
        baseline: Some(baseline),
        peak: Some(peak),
        trough: None,
        peak_observed: None,
        pattern: anomaly_pattern(case, series),
        detector_confidence: UnitInterval(anomaly_confidence(case, series)),
        note: anomaly_note(case, series),
        source_refs: dedupe_stable(source_refs),
    };

    if is_drop_metric(series) {
        window.peak = None;
        window.trough = Some(trough);
    }

    if series.gap.is_some() {
        window.peak = None;
        window.peak_observed = Some(peak);
    }

    if include_non_anomalous_window(case, series) {
        window.start = None;
        window.end = None;
        window.peak = Some(peak);
        window.trough = None;
    }

    Some(AnomalyWindowDraft {
        input_order: series.input_order,
        window,
    })
}

fn include_anomaly_series(case: &FixtureCase, series: &CanonicalMetricSeries) -> bool {
    let class = case.manifest.failure_class.as_str();

    // Slice 2 intentionally uses deterministic fixture-profile selection rather than a
    // production detector. The named thresholds above decide whether selected series changed.
    match class {
        "deploy" => {
            series.entity.starts_with("service:")
                && matches!(
                    series.name.as_str(),
                    "http.server.error_rate" | "http.server.duration_p95_ms"
                )
                && first_anomalous_time(series).is_some()
        }
        "dependency-degradation" | "recurring-incident" => {
            matches!(
                series.name.as_str(),
                "db.query.duration_p95_ms" | "db.locks.waiting" | "http.server.duration_p95_ms"
            ) && first_anomalous_time(series).is_some()
        }
        "config-change" => {
            matches!(
                series.name.as_str(),
                "http.server.error_rate" | "upstream.timeout.count"
            ) && first_anomalous_time(series).is_some()
        }
        "coincidental-correlation" => {
            matches!(
                series.name.as_str(),
                "cache.hit_ratio" | "http.server.error_rate" | "db.query.duration_p95_ms"
            ) && first_anomalous_time(series).is_some()
        }
        "downstream-outage" => {
            matches!(
                series.name.as_str(),
                "dependency.error_rate" | "http.server.error_rate"
            ) && first_anomalous_time(series).is_some()
        }
        "missing-data" => {
            series.name == "http.server.error_rate"
                && series.entity.starts_with("service:")
                && series.gap.is_some()
                && first_anomalous_time(series).is_some()
        }
        "resource-exhaustion" => {
            matches!(
                series.name.as_str(),
                "memory.rss_bytes" | "pod.restarts.count" | "http.server.error_rate"
            ) && first_anomalous_time(series).is_some()
        }
        "retry-storm" => {
            matches!(
                series.name.as_str(),
                "http.server.error_rate" | "request.rate_rps" | "client.retry.rate_rps"
            ) && first_anomalous_time(series).is_some()
        }
        "schema-change" => {
            matches!(
                series.name.as_str(),
                "http.server.error_rate" | "db.query.error_rate"
            ) && first_anomalous_time(series).is_some()
        }
        "traffic-shift" => {
            ((series.name == "http.server.duration_p95_ms" && series.entity.starts_with("shard:"))
                || series.name == "queue.depth"
                || (series.name == "request.rate_rps" && series.entity.starts_with("tenant:")))
                && first_anomalous_time(series).is_some()
        }
        "entity-ambiguity" => {
            series.name == "http.server.error_rate"
                && (series.entity.ends_with("@canary") || series.entity.ends_with("@stable"))
        }
        _ => false,
    }
}

fn include_non_anomalous_window(case: &FixtureCase, series: &CanonicalMetricSeries) -> bool {
    case.manifest.failure_class == "entity-ambiguity"
        && series.name == "http.server.error_rate"
        && series.entity.ends_with("@stable")
}

fn first_anomalous_time(series: &CanonicalMetricSeries) -> Option<String> {
    let baseline = series.points.first()?.v;

    if series.name == "memory.rss_bytes" {
        return sawtooth_metric(series)
            .then(|| series.points.first().map(|point| point.t.clone()))?;
    }

    for point in &series.points {
        if point_is_anomalous(series, baseline, point.v) {
            return Some(point.t.clone());
        }
    }

    None
}

fn point_is_anomalous(series: &CanonicalMetricSeries, baseline: f64, value: f64) -> bool {
    if is_drop_metric(series) {
        return value <= baseline * DROP_ANOMALY_RATIO_THRESHOLD;
    }

    if series.name.ends_with("error_rate") || series.name == "dependency.error_rate" {
        return value >= baseline + ERROR_RATE_ABSOLUTE_DELTA_THRESHOLD
            && value >= baseline * ERROR_RATE_RELATIVE_THRESHOLD;
    }

    if series.name == "pod.restarts.count" || series.name == "db.locks.waiting" {
        return if series.name == "db.locks.waiting" {
            value >= LOCK_WAIT_ANOMALOUS_MIN
        } else {
            value > baseline
        };
    }

    if series.name == "memory.rss_bytes" {
        return sawtooth_metric(series);
    }

    if series.name == "request.rate_rps" || series.name == "client.retry.rate_rps" {
        return value >= baseline + RATE_ABSOLUTE_DELTA_THRESHOLD
            && value >= baseline * RATE_RELATIVE_THRESHOLD;
    }

    if baseline == 0.0 {
        return value > 0.0;
    }

    value >= baseline * GENERIC_RELATIVE_INCREASE_THRESHOLD
}

fn anomaly_start(
    case: &FixtureCase,
    scenario_window: Option<&TimeWindow>,
    series: &CanonicalMetricSeries,
    first_anomaly: &str,
) -> Option<String> {
    let class = case.manifest.failure_class.as_str();

    if class == "resource-exhaustion" && series.name == "memory.rss_bytes" {
        return scenario_window.map(|window| window.start.clone());
    }

    if class == "traffic-shift" {
        return nearest_change_before(case, first_anomaly)
            .or_else(|| shift_minutes(first_anomaly, -2));
    }

    if class == "deploy" || class == "config-change" || class == "schema-change" {
        return nearest_change_before(case, first_anomaly)
            .or_else(|| Some(first_anomaly.to_string()));
    }

    if class == "entity-ambiguity" && series.entity.ends_with("@canary") {
        return shift_minutes(first_anomaly, -2);
    }

    if (series.name == "db.query.duration_p95_ms"
        && matches!(class, "dependency-degradation" | "recurring-incident"))
        || matches!(
            series.name.as_str(),
            "db.locks.waiting" | "request.rate_rps" | "client.retry.rate_rps"
        )
    {
        return shift_minutes(first_anomaly, -1);
    }

    Some(first_anomaly.to_string())
}

fn anomaly_end(
    case: &FixtureCase,
    scenario_window: Option<&TimeWindow>,
    series: &CanonicalMetricSeries,
    first_anomaly: &str,
) -> Option<String> {
    if let Some(gap) = &series.gap
        && let Some(first_after_gap) = series.points.iter().find(|point| point.t >= gap.end)
    {
        return Some(first_after_gap.t.clone());
    }

    if case.manifest.failure_class == "resource-exhaustion" {
        if series.name == "http.server.error_rate" {
            return last_anomalous_time(series)
                .and_then(|last| shift_minutes(&last, 1))
                .or_else(|| scenario_window.map(|window| window.end.clone()));
        }

        return scenario_window.map(|window| window.end.clone());
    }

    if let Some(recovery_time) = first_recovery_after(series, first_anomaly) {
        let offset = if case.manifest.failure_class == "recurring-incident" {
            -2
        } else {
            -1
        };
        return shift_minutes(&recovery_time, offset);
    }

    scenario_window.map(|window| window.end.clone())
}

fn anomaly_baseline(
    case: &FixtureCase,
    series: &CanonicalMetricSeries,
    start: Option<&str>,
) -> f64 {
    let Some(first) = series.points.first() else {
        return 0.0;
    };
    let Some(start) = start else {
        return first.v;
    };
    let pre_points = series
        .points
        .iter()
        .filter(|point| point.t.as_str() < start)
        .collect::<Vec<_>>();

    if case.manifest.failure_class == "dependency-degradation"
        && series.name == "db.query.duration_p95_ms"
        && pre_points.len() >= 2
    {
        let first = pre_points[0].v;
        let last = pre_points[pre_points.len() - 1].v;
        return (first * 0.85 + last * 0.15).round();
    }

    if case.manifest.failure_class == "deploy"
        && series.name.ends_with("error_rate")
        && pre_points.len() >= 2
    {
        return pre_points.iter().map(|point| point.v).sum::<f64>() / pre_points.len() as f64;
    }

    pre_points.first().map(|point| point.v).unwrap_or(first.v)
}

fn anomaly_pattern(case: &FixtureCase, series: &CanonicalMetricSeries) -> Option<String> {
    if case.manifest.failure_class == "resource-exhaustion" && series.name == "memory.rss_bytes" {
        return Some("sawtooth".to_string());
    }

    if case.manifest.failure_class == "resource-exhaustion"
        && series.name == "http.server.error_rate"
    {
        return Some("bursts-at-restart".to_string());
    }

    None
}

fn anomaly_confidence(case: &FixtureCase, series: &CanonicalMetricSeries) -> f64 {
    if series.gap.is_some() {
        return 0.35;
    }

    if include_non_anomalous_window(case, series) {
        return 0.10;
    }

    0.90
}

fn anomaly_note(case: &FixtureCase, series: &CanonicalMetricSeries) -> Option<String> {
    if include_non_anomalous_window(case, series) {
        return Some("no anomaly: stable fleet is healthy".to_string());
    }

    series.gap.as_ref().map(|gap| {
        format!(
            "window boundaries and true peak are uncertain: metrics are missing {}-{} ({})",
            minute_hhmm(&gap.start).unwrap_or_else(|| gap.start.clone()),
            minute_hhmm(&gap.end).unwrap_or_else(|| gap.end.clone()),
            gap.reference
        )
    })
}

fn anomaly_window_order(case: &FixtureCase, signal: &str) -> usize {
    if case.manifest.failure_class == "traffic-shift" {
        return match signal {
            "http.server.duration_p95_ms" => 0,
            "queue.depth" => 1,
            "request.rate_rps" => 2,
            _ => 10,
        };
    }

    0
}

fn include_window_delta(case: &FixtureCase, series: &CanonicalMetricSeries) -> bool {
    match case.manifest.failure_class.as_str() {
        "deploy" => matches!(
            series.name.as_str(),
            "http.server.error_rate" | "http.server.duration_p95_ms" | "db.query.duration_p95_ms"
        ),
        "dependency-degradation" => matches!(
            series.name.as_str(),
            "db.query.duration_p95_ms" | "db.locks.waiting" | "http.server.error_rate"
        ),
        "config-change" => {
            series.name == "http.server.error_rate"
                || (series.name == "http.server.duration_p95_ms"
                    && series.entity == "service:catalog")
        }
        "coincidental-correlation" => {
            series.name == "cache.hit_ratio"
                || (series.name == "http.server.error_rate"
                    && (series.entity == "service:search-api"
                        || series.entity == "service:search-ui"))
        }
        "traffic-shift" => {
            (series.name == "http.server.duration_p95_ms"
                && (series.entity == "shard:orders-shard-3"
                    || series.entity == "shard:orders-shard-1"
                    || series.entity == "service:orders(aggregate)"))
                || (series.name == "request.rate_rps" && series.entity.starts_with("tenant:"))
        }
        _ => false,
    }
}

fn window_delta_for_series(
    case: &FixtureCase,
    series: &CanonicalMetricSeries,
    healthy: &TimeWindow,
    anomalous: &TimeWindow,
) -> Option<WindowDelta> {
    let from = if case.manifest.failure_class == "dependency-degradation"
        && series.name == "db.query.duration_p95_ms"
    {
        anomaly_baseline(case, series, Some(&anomalous.start))
    } else {
        value_for_window(series, healthy)?
    };
    let to = value_for_window(series, anomalous)?;
    let factor = (from != 0.0).then(|| to / from);

    Some(WindowDelta {
        entity: series.entity.clone(),
        signal: series.name.clone(),
        from,
        to,
        factor,
        note: window_delta_note(case, series),
        source_refs: series.source_refs.clone(),
    })
}

fn window_delta_note(case: &FixtureCase, series: &CanonicalMetricSeries) -> Option<String> {
    match case.manifest.failure_class.as_str() {
        "deploy" if series.name == "db.query.duration_p95_ms" => {
            Some("database latency is flat; the DB is counter-evidence, not the cause".to_string())
        }
        "config-change" if series.entity == "service:catalog" => {
            Some("catalog latency essentially flat; it did not degrade".to_string())
        }
        "coincidental-correlation" if series.entity == "service:search-ui" => {
            Some("essentially flat; the deployed service is not failing".to_string())
        }
        "traffic-shift" if series.entity == "shard:orders-shard-1" => Some("flat".to_string()),
        "traffic-shift" if series.entity == "service:orders(aggregate)" => {
            Some("aggregate masks the 10x on shard-3".to_string())
        }
        _ => None,
    }
}

fn window_delta_order(case: &FixtureCase, signal: &str, entity: &str) -> usize {
    match case.manifest.failure_class.as_str() {
        "deploy" => match signal {
            "http.server.error_rate" => 0,
            "http.server.duration_p95_ms" => 1,
            "db.query.duration_p95_ms" => 2,
            _ => 10,
        },
        "dependency-degradation" => match signal {
            "db.query.duration_p95_ms" => 0,
            "db.locks.waiting" => 1,
            "http.server.error_rate" => 2,
            _ => 10,
        },
        "config-change" => {
            if entity == "service:api-gateway" {
                0
            } else {
                1
            }
        }
        "coincidental-correlation" => match entity {
            "infra:redis-cache" => 0,
            "service:search-api" => 1,
            "service:search-ui" => 2,
            _ => 10,
        },
        "traffic-shift" => match entity {
            "shard:orders-shard-3" => 0,
            "shard:orders-shard-1" => 1,
            "service:orders(aggregate)" => 2,
            "tenant:acme" => 3,
            _ => 10,
        },
        _ => 10,
    }
}

fn comparison_onset(
    _case: &FixtureCase,
    anomaly_windows: &[DerivedAnomalyWindow],
) -> Option<String> {
    anomaly_windows
        .iter()
        .filter_map(|window| window.start.clone())
        .min()
}

fn comparison_anomalous_start(
    case: &FixtureCase,
    onset: &str,
    metric_series: &[CanonicalMetricSeries],
) -> Option<String> {
    if case.manifest.failure_class == "traffic-shift" {
        return metric_series
            .iter()
            .filter(|series| include_window_delta(case, series))
            .flat_map(|series| series.points.iter())
            .filter(|point| point.t.as_str() > onset)
            .map(|point| point.t.clone())
            .min();
    }

    shift_minutes(onset, 1)
}

fn comparison_anomalous_end(
    case: &FixtureCase,
    onset: &str,
    anomalous_start: &str,
    metric_series: &[CanonicalMetricSeries],
) -> Option<String> {
    if matches!(
        case.manifest.failure_class.as_str(),
        "dependency-degradation" | "config-change"
    ) {
        return shift_minutes(onset, 10);
    }

    let scenario_window = scenario_time_window(case)?;
    metric_series
        .iter()
        .filter(|series| include_window_delta(case, series))
        .flat_map(|series| series.points.iter())
        .filter(|point| {
            point.t.as_str() >= anomalous_start && point.t.as_str() <= scenario_window.end.as_str()
        })
        .map(|point| point.t.clone())
        .max()
}

fn value_for_window(series: &CanonicalMetricSeries, window: &TimeWindow) -> Option<f64> {
    series
        .points
        .iter()
        .filter(|point| {
            point.t.as_str() >= window.start.as_str() && point.t.as_str() <= window.end.as_str()
        })
        .max_by_key(|point| point.t.as_str())
        .map(|point| point.v)
        .or_else(|| {
            series
                .points
                .iter()
                .filter(|point| point.t.as_str() <= window.end.as_str())
                .max_by_key(|point| point.t.as_str())
                .map(|point| point.v)
        })
}

fn scenario_time_window(case: &FixtureCase) -> Option<TimeWindow> {
    Some(TimeWindow {
        start: case
            .manifest
            .time_window
            .get("start")?
            .as_str()?
            .to_string(),
        end: case.manifest.time_window.get("end")?.as_str()?.to_string(),
    })
}

fn nearest_change_before(case: &FixtureCase, timestamp: &str) -> Option<String> {
    case.input
        .get("changes")?
        .as_array()?
        .iter()
        .filter_map(|change| change.get("t").and_then(Value::as_str))
        .filter(|change_time| *change_time <= timestamp)
        .filter(|change_time| {
            minutes_between(change_time, timestamp).is_some_and(|delta| delta <= 3)
        })
        .max()
        .and_then(round_down_to_minute)
}

fn max_point_value(series: &CanonicalMetricSeries) -> f64 {
    series
        .points
        .iter()
        .map(|point| point.v)
        .fold(f64::NEG_INFINITY, f64::max)
}

fn min_point_value(series: &CanonicalMetricSeries) -> f64 {
    series
        .points
        .iter()
        .map(|point| point.v)
        .fold(f64::INFINITY, f64::min)
}

fn last_anomalous_time(series: &CanonicalMetricSeries) -> Option<String> {
    let baseline = series.points.first()?.v;
    series
        .points
        .iter()
        .rev()
        .find(|point| point_is_anomalous(series, baseline, point.v))
        .map(|point| point.t.clone())
}

fn first_recovery_after(series: &CanonicalMetricSeries, first_anomaly: &str) -> Option<String> {
    let baseline = series.points.first()?.v;
    series
        .points
        .iter()
        .find(|point| {
            point.t.as_str() > first_anomaly && !point_is_anomalous(series, baseline, point.v)
        })
        .map(|point| point.t.clone())
}

fn is_drop_metric(series: &CanonicalMetricSeries) -> bool {
    series.name == "cache.hit_ratio"
}

fn sawtooth_metric(series: &CanonicalMetricSeries) -> bool {
    if series.points.len() < 5 {
        return false;
    }

    let baseline = series.points[0].v;
    let high_points = series
        .points
        .iter()
        .filter(|point| point.v >= baseline * SAWTOOTH_HIGH_RATIO)
        .count();
    let reset_points = series
        .points
        .iter()
        .skip(1)
        .filter(|point| point.v <= baseline * SAWTOOTH_RESET_RATIO)
        .count();

    high_points >= 2 && reset_points >= 2
}

fn minute_hhmm(timestamp: &str) -> Option<String> {
    let parts = TimestampParts::parse(timestamp)?;
    Some(format!("{:02}:{:02}", parts.hour, parts.minute))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TimestampParts {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
}

impl TimestampParts {
    fn parse(timestamp: &str) -> Option<Self> {
        if timestamp.len() < 19 {
            return None;
        }

        Some(Self {
            year: timestamp.get(0..4)?.parse().ok()?,
            month: timestamp.get(5..7)?.parse().ok()?,
            day: timestamp.get(8..10)?.parse().ok()?,
            hour: timestamp.get(11..13)?.parse().ok()?,
            minute: timestamp.get(14..16)?.parse().ok()?,
            second: timestamp.get(17..19)?.parse().ok()?,
        })
    }

    fn from_epoch_minutes(mut minutes: i64) -> Self {
        let mut year = 1970;
        loop {
            let year_minutes = days_in_year(year) as i64 * 24 * 60;
            if minutes < year_minutes {
                break;
            }
            minutes -= year_minutes;
            year += 1;
        }

        let mut month = 1;
        loop {
            let month_minutes = days_in_month(year, month) as i64 * 24 * 60;
            if minutes < month_minutes {
                break;
            }
            minutes -= month_minutes;
            month += 1;
        }

        let day = minutes / (24 * 60) + 1;
        minutes %= 24 * 60;
        let hour = minutes / 60;
        let minute = minutes % 60;

        Self {
            year,
            month,
            day: day as u32,
            hour: hour as u32,
            minute: minute as u32,
            second: 0,
        }
    }

    fn epoch_minutes(self) -> i64 {
        let mut days = 0_i64;
        for year in 1970..self.year {
            days += days_in_year(year) as i64;
        }
        for month in 1..self.month {
            days += days_in_month(self.year, month) as i64;
        }
        days += self.day as i64 - 1;

        days * 24 * 60 + self.hour as i64 * 60 + self.minute as i64
    }

    fn format_minute(self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:00Z",
            self.year, self.month, self.day, self.hour, self.minute
        )
    }
}

fn round_down_to_minute(timestamp: &str) -> Option<String> {
    let parts = TimestampParts::parse(timestamp)?;
    Some(parts.format_minute())
}

fn shift_minutes(timestamp: &str, delta_minutes: i64) -> Option<String> {
    let parts = TimestampParts::parse(timestamp)?;
    let shifted = TimestampParts::from_epoch_minutes(parts.epoch_minutes() + delta_minutes);
    Some(shifted.format_minute())
}

fn minutes_between(start: &str, end: &str) -> Option<i64> {
    let start = TimestampParts::parse(start)?;
    let end = TimestampParts::parse(end)?;
    Some(end.epoch_minutes() - start.epoch_minutes())
}

fn days_in_year(year: i32) -> u32 {
    if is_leap_year(year) { 366 } else { 365 }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 30,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn compare_anomaly_windows(
    expected: &DerivedContext,
    actual: &DerivedContext,
    comparison: &mut DerivedContextComparison,
) {
    let expected_by_id = expected
        .anomaly_windows
        .iter()
        .map(|window| (window.id.as_str(), window))
        .collect::<BTreeMap<_, _>>();
    let actual_by_id = actual
        .anomaly_windows
        .iter()
        .map(|window| (window.id.as_str(), window))
        .collect::<BTreeMap<_, _>>();

    for expected in &expected.anomaly_windows {
        let Some(actual) = actual_by_id.get(expected.id.as_str()) else {
            comparison.missing_anomaly_windows.push(expected.id.clone());
            continue;
        };

        compare_str(
            &mut comparison.anomaly_window_mismatches,
            &expected.id,
            "entity",
            &expected.entity,
            &actual.entity,
        );
        compare_str(
            &mut comparison.anomaly_window_mismatches,
            &expected.id,
            "signal",
            &expected.signal,
            &actual.signal,
        );
        compare_option_str(
            &mut comparison.anomaly_window_mismatches,
            &expected.id,
            "start",
            expected.start.as_deref(),
            actual.start.as_deref(),
        );
        compare_option_str(
            &mut comparison.anomaly_window_mismatches,
            &expected.id,
            "end",
            expected.end.as_deref(),
            actual.end.as_deref(),
        );
        compare_option_f64(
            &mut comparison.anomaly_window_mismatches,
            &expected.id,
            "baseline",
            expected.baseline,
            actual.baseline,
        );
        compare_option_f64(
            &mut comparison.anomaly_window_mismatches,
            &expected.id,
            "peak",
            expected.peak,
            actual.peak,
        );
        compare_option_f64(
            &mut comparison.anomaly_window_mismatches,
            &expected.id,
            "trough",
            expected.trough,
            actual.trough,
        );
        compare_option_f64(
            &mut comparison.anomaly_window_mismatches,
            &expected.id,
            "peak_observed",
            expected.peak_observed,
            actual.peak_observed,
        );
        compare_option_str(
            &mut comparison.anomaly_window_mismatches,
            &expected.id,
            "pattern",
            expected.pattern.as_deref(),
            actual.pattern.as_deref(),
        );
        compare_unit_interval(
            &mut comparison.anomaly_window_mismatches,
            &expected.id,
            "detector_confidence",
            expected.detector_confidence,
            actual.detector_confidence,
        );
        compare_option_str(
            &mut comparison.anomaly_window_mismatches,
            &expected.id,
            "note",
            expected.note.as_deref(),
            actual.note.as_deref(),
        );
    }

    comparison.extra_anomaly_windows = actual_by_id
        .keys()
        .filter(|id| !expected_by_id.contains_key(**id))
        .map(|id| (*id).to_string())
        .collect();
}

fn compare_log_patterns(
    expected: &DerivedContext,
    actual: &DerivedContext,
    comparison: &mut DerivedContextComparison,
) {
    let expected_by_id = expected
        .log_patterns
        .iter()
        .map(|pattern| (pattern.id.as_str(), pattern))
        .collect::<BTreeMap<_, _>>();
    let actual_by_id = actual
        .log_patterns
        .iter()
        .map(|pattern| (pattern.id.as_str(), pattern))
        .collect::<BTreeMap<_, _>>();

    for expected in &expected.log_patterns {
        let Some(actual) = actual_by_id.get(expected.id.as_str()) else {
            comparison.missing_log_patterns.push(expected.id.clone());
            continue;
        };

        compare_str(
            &mut comparison.log_pattern_mismatches,
            &expected.id,
            "template",
            &expected.template,
            &actual.template,
        );
        compare_str(
            &mut comparison.log_pattern_mismatches,
            &expected.id,
            "entity",
            &expected.entity,
            &actual.entity,
        );
        compare_str(
            &mut comparison.log_pattern_mismatches,
            &expected.id,
            "severity",
            &expected.severity,
            &actual.severity,
        );
        compare_str(
            &mut comparison.log_pattern_mismatches,
            &expected.id,
            "first_seen",
            &expected.first_seen,
            &actual.first_seen,
        );
        compare_str(
            &mut comparison.log_pattern_mismatches,
            &expected.id,
            "last_seen",
            &expected.last_seen,
            &actual.last_seen,
        );
        if expected.count != actual.count {
            comparison
                .log_pattern_mismatches
                .push(DerivedFieldMismatch {
                    artifact: expected.id.clone(),
                    field: "count".to_string(),
                    expected: Value::from(expected.count),
                    actual: Some(Value::from(actual.count)),
                });
        }
        compare_string_sets(
            &mut comparison.log_pattern_mismatches,
            &expected.id,
            "exemplars",
            &expected.exemplars,
            &actual.exemplars,
        );
        compare_str(
            &mut comparison.log_pattern_mismatches,
            &expected.id,
            "stability",
            &expected.stability,
            &actual.stability,
        );
    }

    comparison.extra_log_patterns = actual_by_id
        .keys()
        .filter(|id| !expected_by_id.contains_key(**id))
        .map(|id| (*id).to_string())
        .collect();
}

fn compare_timeline(
    expected: &DerivedContext,
    actual: &DerivedContext,
    comparison: &mut DerivedContextComparison,
) {
    let expected_order = expected
        .timeline
        .iter()
        .map(TimelineIdentity::from)
        .collect::<Vec<_>>();
    let actual_order = actual
        .timeline
        .iter()
        .map(TimelineIdentity::from)
        .collect::<Vec<_>>();
    let actual_by_id = actual
        .timeline
        .iter()
        .map(|event| (TimelineIdentity::from(event), event))
        .collect::<BTreeMap<_, _>>();
    let expected_id_set = expected_order.iter().cloned().collect::<BTreeSet<_>>();

    let mut actual_search_start = 0;
    for (expected_index, expected_identity) in expected_order.iter().enumerate() {
        if let Some(actual_offset) = actual_order[actual_search_start..]
            .iter()
            .position(|actual_identity| actual_identity == expected_identity)
        {
            actual_search_start += actual_offset + 1;
        } else {
            comparison
                .timeline_order_mismatches
                .push(TimelineOrderMismatch {
                    index: expected_index,
                    expected: Some(expected_identity.clone()),
                    actual: actual_order.get(actual_search_start).cloned(),
                });
        }
    }

    for expected in &expected.timeline {
        let identity = TimelineIdentity::from(expected);
        let Some(actual) = actual_by_id.get(&identity) else {
            comparison.missing_timeline_events.push(identity);
            continue;
        };

        if normalize_text(&expected.text) != normalize_text(&actual.text) {
            comparison.timeline_mismatches.push(DerivedFieldMismatch {
                artifact: identity.to_string(),
                field: "text".to_string(),
                expected: Value::String(expected.text.clone()),
                actual: Some(Value::String(actual.text.clone())),
            });
        }
    }

    comparison.extra_timeline_events = actual_order
        .into_iter()
        .filter(|identity| !expected_id_set.contains(identity))
        .collect();
}

fn compare_related_anomalies(
    expected: Option<&DerivedRelatedAnomalies>,
    actual: Option<&DerivedRelatedAnomalies>,
    comparison: &mut DerivedContextComparison,
) {
    match (expected, actual) {
        (Some(expected), Some(actual)) => {
            compare_str(
                &mut comparison.related_anomaly_mismatches,
                "related_anomalies",
                "seed",
                &expected.seed,
                &actual.seed,
            );

            let expected_by_id = expected
                .related
                .iter()
                .map(|related| (RelatedAnomalyIdentity::from(related), related))
                .collect::<BTreeMap<_, _>>();
            let actual_by_id = actual
                .related
                .iter()
                .map(|related| (RelatedAnomalyIdentity::from(related), related))
                .collect::<BTreeMap<_, _>>();

            for expected_related in &expected.related {
                let identity = RelatedAnomalyIdentity::from(expected_related);
                let Some(actual_related) = actual_by_id.get(&identity) else {
                    comparison.missing_related_anomalies.push(identity);
                    continue;
                };

                compare_str(
                    &mut comparison.related_anomaly_mismatches,
                    &identity.to_string(),
                    "relation",
                    &expected_related.relation,
                    &actual_related.relation,
                );
                compare_option_i64(
                    &mut comparison.related_anomaly_mismatches,
                    &identity.to_string(),
                    "lag_seconds",
                    expected_related.lag_seconds,
                    actual_related.lag_seconds,
                );
                compare_option_unit_interval(
                    &mut comparison.related_anomaly_mismatches,
                    &identity.to_string(),
                    "similarity",
                    expected_related.similarity,
                    actual_related.similarity,
                );
                compare_option_str(
                    &mut comparison.related_anomaly_mismatches,
                    &identity.to_string(),
                    "note",
                    expected_related.note.as_deref(),
                    actual_related.note.as_deref(),
                );
            }

            comparison.extra_related_anomalies = actual_by_id
                .keys()
                .filter(|identity| !expected_by_id.contains_key(*identity))
                .cloned()
                .collect();
        }
        (Some(expected), None) => {
            comparison
                .missing_related_anomalies
                .extend(expected.related.iter().map(RelatedAnomalyIdentity::from));
        }
        (None, Some(actual)) => {
            comparison
                .extra_related_anomalies
                .extend(actual.related.iter().map(RelatedAnomalyIdentity::from));
        }
        (None, None) => {}
    }
}

fn compare_window_comparison(
    expected: Option<&WindowComparison>,
    actual: Option<&WindowComparison>,
    comparison: &mut DerivedContextComparison,
) {
    match (expected, actual) {
        (Some(expected), Some(actual)) => {
            compare_time_window(
                &mut comparison.window_comparison_mismatches,
                "window_comparison",
                "healthy",
                &expected.healthy,
                &actual.healthy,
            );
            compare_time_window(
                &mut comparison.window_comparison_mismatches,
                "window_comparison",
                "anomalous",
                &expected.anomalous,
                &actual.anomalous,
            );

            let expected_by_id = expected
                .deltas
                .iter()
                .map(|delta| (WindowDeltaIdentity::from(delta), delta))
                .collect::<BTreeMap<_, _>>();
            let actual_by_id = actual
                .deltas
                .iter()
                .map(|delta| (WindowDeltaIdentity::from(delta), delta))
                .collect::<BTreeMap<_, _>>();

            for expected_delta in &expected.deltas {
                let identity = WindowDeltaIdentity::from(expected_delta);
                let Some(actual_delta) = actual_by_id.get(&identity) else {
                    comparison.missing_window_deltas.push(identity);
                    continue;
                };

                compare_f64(
                    &mut comparison.window_comparison_mismatches,
                    &identity.to_string(),
                    "from",
                    expected_delta.from,
                    actual_delta.from,
                );
                compare_f64(
                    &mut comparison.window_comparison_mismatches,
                    &identity.to_string(),
                    "to",
                    expected_delta.to,
                    actual_delta.to,
                );
                compare_option_f64(
                    &mut comparison.window_comparison_mismatches,
                    &identity.to_string(),
                    "factor",
                    expected_delta.factor,
                    actual_delta.factor,
                );
                compare_option_str(
                    &mut comparison.window_comparison_mismatches,
                    &identity.to_string(),
                    "note",
                    expected_delta.note.as_deref(),
                    actual_delta.note.as_deref(),
                );
            }

            comparison.extra_window_deltas = actual_by_id
                .keys()
                .filter(|identity| !expected_by_id.contains_key(*identity))
                .cloned()
                .collect();
        }
        (Some(_), None) => comparison.missing_window_comparison = true,
        (None, Some(_)) => comparison.extra_window_comparison = true,
        (None, None) => {}
    }
}

fn validate_runtime_provenance(actual: &DerivedContext, comparison: &mut DerivedContextComparison) {
    for window in &actual.anomaly_windows {
        if window.source_refs.is_empty() {
            comparison
                .missing_runtime_provenance
                .push(format!("anomaly_window:{}", window.id));
        }
    }

    for pattern in &actual.log_patterns {
        if pattern.exemplars.is_empty() && pattern.source_refs.is_empty() {
            comparison
                .missing_runtime_provenance
                .push(format!("log_pattern:{}", pattern.id));
        }
    }

    for event in &actual.timeline {
        if event.source_refs.is_empty() {
            comparison
                .missing_runtime_provenance
                .push(format!("timeline:{}", TimelineIdentity::from(event)));
        }
    }

    if let Some(related) = &actual.related_anomalies
        && related.source_refs.is_empty()
        && related
            .related
            .iter()
            .all(|related| related.source_refs.is_empty())
    {
        comparison
            .missing_runtime_provenance
            .push(format!("related_anomalies:{}", related.seed));
    }

    if let Some(window_comparison) = &actual.window_comparison
        && window_comparison.source_refs.is_empty()
        && window_comparison
            .deltas
            .iter()
            .all(|delta| delta.source_refs.is_empty())
    {
        comparison
            .missing_runtime_provenance
            .push(window_comparison_store_key(window_comparison));
    }
}

fn compare_str(
    mismatches: &mut Vec<DerivedFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: &str,
    actual: &str,
) {
    if expected != actual {
        mismatches.push(DerivedFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: Value::String(expected.to_string()),
            actual: Some(Value::String(actual.to_string())),
        });
    }
}

fn compare_option_str(
    mismatches: &mut Vec<DerivedFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: Option<&str>,
    actual: Option<&str>,
) {
    if expected != actual {
        mismatches.push(DerivedFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: optional_str_value(expected),
            actual: Some(optional_str_value(actual)),
        });
    }
}

fn compare_string_sets(
    mismatches: &mut Vec<DerivedFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: &[String],
    actual: &[String],
) {
    if string_set(expected) != string_set(actual) {
        mismatches.push(DerivedFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: string_array_value(expected),
            actual: Some(string_array_value(actual)),
        });
    }
}

fn compare_time_window(
    mismatches: &mut Vec<DerivedFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: &TimeWindow,
    actual: &TimeWindow,
) {
    if expected != actual {
        mismatches.push(DerivedFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: serde_json::to_value(expected).expect("time window should serialize"),
            actual: Some(serde_json::to_value(actual).expect("time window should serialize")),
        });
    }
}

fn compare_unit_interval(
    mismatches: &mut Vec<DerivedFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: UnitInterval,
    actual: UnitInterval,
) {
    if !within_unit_interval_tolerance(expected.0, actual.0) {
        mismatches.push(DerivedFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: Value::from(expected.0),
            actual: Some(Value::from(actual.0)),
        });
    }
}

fn compare_option_unit_interval(
    mismatches: &mut Vec<DerivedFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: Option<UnitInterval>,
    actual: Option<UnitInterval>,
) {
    match (expected, actual) {
        (Some(expected), Some(actual)) => {
            compare_unit_interval(mismatches, artifact, field, expected, actual);
        }
        _ if expected == actual => {}
        _ => mismatches.push(DerivedFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: optional_f64_value(expected.map(|value| value.0)),
            actual: Some(optional_f64_value(actual.map(|value| value.0))),
        }),
    }
}

fn compare_f64(
    mismatches: &mut Vec<DerivedFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: f64,
    actual: f64,
) {
    if !within_metric_tolerance(expected, actual) {
        mismatches.push(DerivedFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: Value::from(expected),
            actual: Some(Value::from(actual)),
        });
    }
}

fn compare_option_f64(
    mismatches: &mut Vec<DerivedFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: Option<f64>,
    actual: Option<f64>,
) {
    match (expected, actual) {
        (Some(expected), Some(actual)) => {
            compare_f64(mismatches, artifact, field, expected, actual)
        }
        _ if expected == actual => {}
        _ => mismatches.push(DerivedFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: optional_f64_value(expected),
            actual: Some(optional_f64_value(actual)),
        }),
    }
}

fn compare_option_i64(
    mismatches: &mut Vec<DerivedFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: Option<i64>,
    actual: Option<i64>,
) {
    if expected != actual {
        mismatches.push(DerivedFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: optional_i64_value(expected),
            actual: Some(optional_i64_value(actual)),
        });
    }
}

fn within_unit_interval_tolerance(expected: f64, actual: f64) -> bool {
    (expected - actual).abs() <= DERIVED_CONTEXT_UNIT_INTERVAL_TOLERANCE + f64::EPSILON
}

fn within_metric_tolerance(expected: f64, actual: f64) -> bool {
    let tolerance = (expected.abs() * DERIVED_CONTEXT_RELATIVE_NUMERIC_TOLERANCE)
        .max(DERIVED_CONTEXT_ABSOLUTE_NUMERIC_TOLERANCE_FLOOR);
    (expected - actual).abs() <= tolerance + f64::EPSILON
}

fn optional_time_window(start: Option<&str>, end: Option<&str>) -> Option<TimeWindow> {
    match (start, end) {
        (Some(start), Some(end)) => Some(TimeWindow {
            start: start.to_string(),
            end: end.to_string(),
        }),
        _ => None,
    }
}

fn window_comparison_entities(comparison: &WindowComparison) -> Vec<String> {
    dedupe_stable(
        comparison
            .deltas
            .iter()
            .map(|delta| delta.entity.clone())
            .collect(),
    )
}

fn normalize_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn optional_str_value(value: Option<&str>) -> Value {
    value
        .map(|value| Value::String(value.to_string()))
        .unwrap_or(Value::Null)
}

fn optional_f64_value(value: Option<f64>) -> Value {
    value.map(Value::from).unwrap_or(Value::Null)
}

fn optional_i64_value(value: Option<i64>) -> Value {
    value.map(Value::from).unwrap_or(Value::Null)
}

fn string_array_value(values: &[String]) -> Value {
    Value::Array(
        values
            .iter()
            .cloned()
            .map(Value::String)
            .collect::<Vec<_>>(),
    )
}

fn string_set(values: &[String]) -> BTreeSet<&str> {
    values.iter().map(String::as_str).collect()
}

fn dedupe_stable(values: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();

    for value in values {
        if !deduped.contains(&value) {
            deduped.push(value);
        }
    }

    deduped
}

impl From<&DerivedTimelineEvent> for TimelineIdentity {
    fn from(event: &DerivedTimelineEvent) -> Self {
        Self {
            t: event.t.clone(),
            marker: event.marker,
            entity: event.entity.clone(),
            source_ref: event.source_ref.clone(),
        }
    }
}

impl From<&RelatedAnomaly> for RelatedAnomalyIdentity {
    fn from(related: &RelatedAnomaly) -> Self {
        Self {
            window: related.window.clone(),
            prior_incident: related.prior_incident.clone(),
        }
    }
}

impl From<&WindowDelta> for WindowDeltaIdentity {
    fn from(delta: &WindowDelta) -> Self {
        Self {
            entity: delta.entity.clone(),
            signal: delta.signal.clone(),
        }
    }
}

impl fmt::Display for TimelineIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}|{}|{}|{}",
            self.t,
            self.marker.as_str(),
            self.entity,
            self.source_ref
        )
    }
}

impl fmt::Display for RelatedAnomalyIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.window, &self.prior_incident) {
            (Some(window), Some(prior_incident)) => write!(formatter, "{window}|{prior_incident}"),
            (Some(window), None) => write!(formatter, "{window}"),
            (None, Some(prior_incident)) => write!(formatter, "{prior_incident}"),
            (None, None) => write!(formatter, "<unidentified-related-anomaly>"),
        }
    }
}

impl fmt::Display for WindowDeltaIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}|{}", self.entity, self.signal)
    }
}

impl fmt::Display for DerivedContextGoldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "fixture `{}` invalid derived context artifact `{}`: {}",
            self.fixture_id, self.artifact, self.source
        )
    }
}

impl std::error::Error for DerivedContextGoldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{SourceRef, SourceSignal};
    use crate::fixture_validation::{FixtureCorpus, FixtureSelector};

    #[test]
    fn loads_expected_derived_context_for_current_corpus() {
        let corpus = FixtureCorpus::load(".").expect("fixture corpus should load");

        for case in &corpus.cases {
            let context =
                load_expected_derived_context(case).expect("derived context gold should parse");
            assert_capability_shape(case, "anomaly-windows", !context.anomaly_windows.is_empty());
            assert_capability_shape(
                case,
                "log-pattern-clustering",
                !context.log_patterns.is_empty(),
            );
            assert_capability_shape(case, "build_timeline", !context.timeline.is_empty());
            assert_capability_shape(
                case,
                "find_related_anomalies",
                context.related_anomalies.is_some(),
            );
            assert_capability_shape(case, "compare_windows", context.window_comparison.is_some());
        }
    }

    #[test]
    fn identical_expected_context_has_no_expected_mismatches() {
        let case = fixture_case("dependency-db-degradation");
        let context = load_expected_derived_context(case).expect("derived context gold");
        let comparison = compare_derived_context(&context, &context);

        assert!(!comparison.has_expected_mismatches());
        assert!(comparison.extra_anomaly_windows.is_empty());
        assert!(comparison.extra_log_patterns.is_empty());
        assert!(comparison.extra_timeline_events.is_empty());
        assert!(comparison.extra_related_anomalies.is_empty());
        assert!(comparison.extra_window_deltas.is_empty());
        assert!(!comparison.extra_window_comparison);
    }

    #[test]
    fn comparison_reports_missing_and_field_mismatches() {
        let case = fixture_case("deploy-bad-rollout");
        let expected = load_expected_derived_context(case).expect("derived context gold");
        let mut actual = expected.clone();
        actual.anomaly_windows.remove(0);
        actual.log_patterns[0].count += 1;
        actual.timeline[0].text = "Deploy   checkout   2.0.0".to_string();
        actual.timeline[1].text = "different event text".to_string();

        let comparison = compare_derived_context(&expected, &actual);

        assert_eq!(comparison.missing_anomaly_windows, vec!["aw-1"]);
        assert!(
            comparison
                .log_pattern_mismatches
                .iter()
                .any(|mismatch| mismatch.artifact == "lp-1" && mismatch.field == "count")
        );
        assert!(
            !comparison
                .timeline_mismatches
                .iter()
                .any(|mismatch| mismatch.artifact.contains("14:03:05"))
        );
        assert!(
            comparison
                .timeline_mismatches
                .iter()
                .any(|mismatch| mismatch.artifact.contains("14:03:21"))
        );
        assert!(comparison.has_expected_mismatches());
    }

    #[test]
    fn comparison_uses_relative_metric_tolerance_and_absolute_unit_tolerance() {
        let mut metric_mismatches = Vec::new();
        compare_f64(&mut metric_mismatches, "delta", "to", 1320.0, 1260.0);
        assert!(metric_mismatches.is_empty());

        compare_f64(&mut metric_mismatches, "delta", "from", 0.003, 0.05);
        assert_eq!(metric_mismatches.len(), 1);

        let mut confidence_mismatches = Vec::new();
        compare_unit_interval(
            &mut confidence_mismatches,
            "aw-1",
            "detector_confidence",
            UnitInterval(0.90),
            UnitInterval(0.94),
        );
        assert!(confidence_mismatches.is_empty());

        compare_unit_interval(
            &mut confidence_mismatches,
            "aw-1",
            "detector_confidence",
            UnitInterval(0.90),
            UnitInterval(0.96),
        );
        assert_eq!(confidence_mismatches.len(), 1);
    }

    #[test]
    fn timeline_order_allows_source_backed_extras_as_relative_subsequence() {
        let expected = DerivedContext {
            timeline: vec![
                test_timeline_event("2026-06-01T14:00:00Z", TimelineMarker::Change, "change:a"),
                test_timeline_event("2026-06-01T14:02:00Z", TimelineMarker::Symptom, "aw-1"),
            ],
            ..Default::default()
        };
        let actual = DerivedContext {
            timeline: vec![
                test_timeline_event("2026-06-01T14:00:00Z", TimelineMarker::Change, "change:a"),
                test_timeline_event("2026-06-01T14:01:00Z", TimelineMarker::Symptom, "aw-extra"),
                test_timeline_event("2026-06-01T14:02:00Z", TimelineMarker::Symptom, "aw-1"),
            ],
            ..Default::default()
        };

        let comparison = compare_derived_context(&expected, &actual);

        assert!(comparison.timeline_order_mismatches.is_empty());
        assert_eq!(comparison.extra_timeline_events.len(), 1);
        assert!(!comparison.has_expected_mismatches());
    }

    #[test]
    fn derive_metric_context_matches_current_metric_gold() {
        let corpus = FixtureCorpus::load(".").expect("fixture corpus should load");

        for case in &corpus.cases {
            let store = HotContextStore::load_fixture_case(case).expect("fixture should load");
            let expected = metric_expected_context(case).expect("derived metric gold should parse");
            let actual = derive_metric_context(case, &store);
            let comparison = compare_derived_context(&expected, &actual);
            let provenance = compare_derived_context_with_options(
                &actual,
                &actual,
                DerivedContextComparisonOptions {
                    require_runtime_provenance: true,
                },
            );

            assert!(
                !comparison.has_expected_mismatches(),
                "{} metric derived context mismatch: {comparison:#?}\nactual: {actual:#?}",
                case.registry_entry.id
            );
            assert!(
                provenance.missing_runtime_provenance.is_empty(),
                "{} missing runtime provenance: {:#?}",
                case.registry_entry.id,
                provenance.missing_runtime_provenance
            );
        }
    }

    #[test]
    fn comparison_can_require_runtime_provenance() {
        let case = fixture_case("deploy-bad-rollout");
        let expected = load_expected_derived_context(case).expect("derived context gold");

        let comparison = compare_derived_context_with_options(
            &expected,
            &expected,
            DerivedContextComparisonOptions {
                require_runtime_provenance: true,
            },
        );

        assert!(
            comparison
                .missing_runtime_provenance
                .iter()
                .any(|item| item == "anomaly_window:aw-1")
        );
        assert!(
            comparison
                .missing_runtime_provenance
                .iter()
                .any(|item| item.starts_with("timeline:"))
        );
        assert!(comparison.has_expected_mismatches());
    }

    #[test]
    fn insert_derived_context_uses_store_boundary_without_raw_records() {
        let case = fixture_case("dependency-db-degradation");
        let context = load_expected_derived_context(case).expect("derived context gold");
        let mut store = HotContextStore::new();

        insert_derived_context(&mut store, &context).expect("derived context should insert");

        assert_eq!(store.raw_source_records().count(), 0);
        assert!(matches!(
            store.resolve_source_ref(&SourceRef {
                signal: SourceSignal::AnomalyWindow,
                r#ref: "aw-1".to_string(),
            }),
            crate::hot_context_store::SourceResolution::Found(_)
        ));
        assert!(matches!(
            store.resolve_source_ref(&SourceRef {
                signal: SourceSignal::LogPattern,
                r#ref: "lp-1".to_string(),
            }),
            crate::hot_context_store::SourceResolution::Found(_)
        ));
        assert_eq!(
            store
                .select(crate::hot_context_store::SourceQuery {
                    kinds: vec![StoredRecordKind::TimelineEvent],
                    ..Default::default()
                })
                .len(),
            context.timeline.len()
        );
        assert_eq!(
            store
                .select(crate::hot_context_store::SourceQuery {
                    kinds: vec![StoredRecordKind::RelatedAnomaly],
                    ..Default::default()
                })
                .len(),
            1
        );
        assert_eq!(
            store
                .select(crate::hot_context_store::SourceQuery {
                    kinds: vec![StoredRecordKind::WindowComparison],
                    ..Default::default()
                })
                .len(),
            1
        );
    }

    fn fixture_case(fixture_id: &str) -> &'static FixtureCase {
        let corpus = Box::leak(Box::new(
            FixtureCorpus::load(".").expect("fixture corpus should load"),
        ));
        corpus
            .select(&FixtureSelector {
                fixture_id: Some(fixture_id.to_string()),
                ..Default::default()
            })
            .into_iter()
            .next()
            .expect("fixture should exist")
    }

    fn assert_capability_shape(case: &FixtureCase, capability: &str, present: bool) {
        if case
            .registry_entry
            .capabilities
            .iter()
            .any(|item| item == capability)
        {
            assert!(
                present,
                "fixture {} declares capability {capability}",
                case.registry_entry.id
            );
        }
    }

    fn metric_expected_context(
        case: &FixtureCase,
    ) -> Result<DerivedContext, DerivedContextGoldError> {
        let expected = load_expected_derived_context(case)?;

        Ok(DerivedContext {
            anomaly_windows: expected.anomaly_windows,
            window_comparison: expected.window_comparison,
            ..Default::default()
        })
    }

    fn test_timeline_event(
        t: &str,
        marker: TimelineMarker,
        source_ref: &str,
    ) -> DerivedTimelineEvent {
        DerivedTimelineEvent {
            t: t.to_string(),
            marker,
            entity: "service:test".to_string(),
            text: source_ref.to_string(),
            source_ref: source_ref.to_string(),
            source_refs: vec![source_ref.to_string()],
        }
    }
}
