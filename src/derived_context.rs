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

pub const DERIVED_CONTEXT_NUMERIC_TOLERANCE: f64 = 0.05;

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

    for (index, expected_identity) in expected_order.iter().enumerate() {
        if actual_order.get(index) != Some(expected_identity) {
            comparison
                .timeline_order_mismatches
                .push(TimelineOrderMismatch {
                    index,
                    expected: Some(expected_identity.clone()),
                    actual: actual_order.get(index).cloned(),
                });
        }
    }

    for (index, actual_identity) in actual_order.iter().enumerate().skip(expected_order.len()) {
        comparison
            .timeline_order_mismatches
            .push(TimelineOrderMismatch {
                index,
                expected: None,
                actual: Some(actual_identity.clone()),
            });
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
    if !within_tolerance(expected.0, actual.0) {
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
    if !within_tolerance(expected, actual) {
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

fn within_tolerance(expected: f64, actual: f64) -> bool {
    (expected - actual).abs() <= DERIVED_CONTEXT_NUMERIC_TOLERANCE
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
}
