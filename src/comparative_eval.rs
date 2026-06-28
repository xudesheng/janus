use crate::{
    evidence::{
        EvidenceBundle, EvidenceDirection, EvidenceItem, EvidenceKind, SourceRef, SourceSignal,
        TimeWindow,
    },
    fixture_validation::{FixtureCase, FixtureCorpus, FixtureCorpusLoadError},
    query::{
        EvidenceQuery, EvidenceQueryBudget, EvidenceQueryIntent, FreshnessPreference,
        GetEvidenceBundleError, get_evidence_bundle,
    },
    references::source_signal_name,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::Path,
};

pub const COMPARATIVE_EVAL_SCHEMA_VERSION: &str = "comparative-eval/v1";
pub const DEFAULT_MAX_ITEMS: u32 = 6;
pub const DEFAULT_MAX_TOKENS: u32 = 1200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvalBudget {
    pub max_items: u32,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvalFixtureSelector {
    pub fixture_id: Option<String>,
    pub capability: Option<String>,
    pub failure_class: Option<String>,
    pub difficulty: Option<String>,
    pub false_causality_trap: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalAccessPath {
    Janus,
    Raw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalMetric {
    SuspiciousEntityAccuracy,
    FalseCausalityRisk,
    MissingDataAwareness,
    Auditability,
    TokenEfficiency,
    TimelineQuality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalMetricRole {
    Required,
    ReportOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvalMetricDefinition {
    pub metric: EvalMetric,
    pub role: EvalMetricRole,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvalSubmission {
    pub scenario_id: String,
    pub access_path: EvalAccessPath,
    pub budget: EvalBudget,
    pub serialized_context: Value,
    pub measured_tokens: u32,
    #[serde(default)]
    pub candidate_entities: Vec<EvalCandidateEntity>,
    #[serde(default)]
    pub timeline_events: Vec<EvalTimelineEvent>,
    #[serde(default)]
    pub evidence_refs: Vec<EvalSourceRef>,
    #[serde(default)]
    pub counter_evidence_refs: Vec<EvalSourceRef>,
    #[serde(default)]
    pub missing_data_refs: Vec<EvalSourceRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvalSubmissionInput {
    pub scenario_id: String,
    pub access_path: EvalAccessPath,
    pub budget: EvalBudget,
    pub serialized_context: Value,
    pub candidate_entities: Vec<EvalCandidateEntity>,
    pub timeline_events: Vec<EvalTimelineEvent>,
    pub evidence_refs: Vec<EvalSourceRef>,
    pub counter_evidence_refs: Vec<EvalSourceRef>,
    pub missing_data_refs: Vec<EvalSourceRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvalCandidateEntity {
    pub entity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvalTimelineEvent {
    pub t: String,
    pub marker: String,
    pub entity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<EvalSourceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvalSourceRef {
    pub signal: String,
    pub r#ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvalFixtureRegistryReport {
    pub schema_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparativeEvalReport {
    pub schema_version: String,
    pub repo_sha: String,
    pub fixture_registry: EvalFixtureRegistryReport,
    pub budget: EvalBudget,
    pub metrics: Vec<EvalMetricDefinition>,
    pub summary: ComparativeEvalSummary,
    pub scenarios: Vec<ScenarioEvalReport>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparativeEvalSummary {
    pub fixture_count: usize,
    pub janus: BTreeMap<String, Value>,
    pub raw: BTreeMap<String, Value>,
    pub delta: BTreeMap<String, Value>,
    pub false_causality_traps: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioEvalReport {
    pub id: String,
    pub scenario_schema_version: String,
    pub scenario_version: u64,
    pub failure_class: String,
    pub difficulty: String,
    pub false_causality_trap: bool,
    pub janus: BTreeMap<String, Value>,
    pub raw: BTreeMap<String, Value>,
    pub comparison: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadMeasurement {
    pub bytes: usize,
    pub measured_tokens: u32,
}

#[derive(Debug)]
pub enum ComparativeEvalError {
    FixtureCorpusLoad(FixtureCorpusLoadError),
    NoFixturesSelected,
    InvalidScenarioTimeWindow {
        scenario_id: String,
        source: serde_json::Error,
    },
    JanusAccess {
        scenario_id: String,
        source: Box<GetEvidenceBundleError>,
    },
    SerializePayload(serde_json::Error),
    TokenEstimateOverflow {
        bytes: usize,
    },
}

impl Default for EvalBudget {
    fn default() -> Self {
        Self {
            max_items: DEFAULT_MAX_ITEMS,
            max_tokens: DEFAULT_MAX_TOKENS,
        }
    }
}

impl EvalFixtureSelector {
    pub fn matches(&self, case: &FixtureCase) -> bool {
        let entry = &case.registry_entry;

        self.fixture_id.as_ref().is_none_or(|id| entry.id == *id)
            && self
                .capability
                .as_ref()
                .is_none_or(|capability| entry.capabilities.contains(capability))
            && self
                .failure_class
                .as_ref()
                .is_none_or(|failure_class| entry.failure_class == *failure_class)
            && self
                .difficulty
                .as_ref()
                .is_none_or(|difficulty| entry.difficulty == *difficulty)
            && self
                .false_causality_trap
                .is_none_or(|trap| entry.false_causality_trap == trap)
    }
}

impl EvalSubmission {
    pub fn from_serialized_context(
        input: EvalSubmissionInput,
    ) -> Result<Self, ComparativeEvalError> {
        let measurement = measure_serialized_payload(&input.serialized_context)?;

        Ok(Self {
            scenario_id: input.scenario_id,
            access_path: input.access_path,
            budget: input.budget,
            serialized_context: input.serialized_context,
            measured_tokens: measurement.measured_tokens,
            candidate_entities: input.candidate_entities,
            timeline_events: input.timeline_events,
            evidence_refs: input.evidence_refs,
            counter_evidence_refs: input.counter_evidence_refs,
            missing_data_refs: input.missing_data_refs,
        })
    }
}

pub fn metric_definitions() -> Vec<EvalMetricDefinition> {
    vec![
        EvalMetricDefinition {
            metric: EvalMetric::SuspiciousEntityAccuracy,
            role: EvalMetricRole::Required,
        },
        EvalMetricDefinition {
            metric: EvalMetric::FalseCausalityRisk,
            role: EvalMetricRole::Required,
        },
        EvalMetricDefinition {
            metric: EvalMetric::MissingDataAwareness,
            role: EvalMetricRole::Required,
        },
        EvalMetricDefinition {
            metric: EvalMetric::Auditability,
            role: EvalMetricRole::Required,
        },
        EvalMetricDefinition {
            metric: EvalMetric::TokenEfficiency,
            role: EvalMetricRole::Required,
        },
        EvalMetricDefinition {
            metric: EvalMetric::TimelineQuality,
            role: EvalMetricRole::ReportOnly,
        },
    ]
}

pub fn load_comparative_eval_report_with_janus(
    root: impl AsRef<Path>,
    selector: &EvalFixtureSelector,
    budget: EvalBudget,
    repo_sha: impl Into<String>,
) -> Result<ComparativeEvalReport, ComparativeEvalError> {
    let corpus = FixtureCorpus::load(root).map_err(ComparativeEvalError::FixtureCorpusLoad)?;
    build_comparative_eval_report_with_janus(&corpus, selector, budget, repo_sha)
}

pub fn build_comparative_eval_report_with_janus(
    corpus: &FixtureCorpus,
    selector: &EvalFixtureSelector,
    budget: EvalBudget,
    repo_sha: impl Into<String>,
) -> Result<ComparativeEvalReport, ComparativeEvalError> {
    let cases = selected_cases(corpus, selector)?;
    let mut scenarios = Vec::new();
    let mut total_janus_tokens = 0u32;

    for case in &cases {
        let submission = run_janus_eval_submission(case, budget.clone())?;
        total_janus_tokens = total_janus_tokens.saturating_add(submission.measured_tokens);

        let mut report = scenario_report(case);
        report.janus.insert(
            "submission".to_string(),
            serde_json::to_value(&submission).map_err(ComparativeEvalError::SerializePayload)?,
        );
        scenarios.push(report);
    }

    let mut janus_summary = BTreeMap::new();
    janus_summary.insert("submission_count".to_string(), Value::from(cases.len()));
    janus_summary.insert(
        "measured_tokens".to_string(),
        Value::from(total_janus_tokens),
    );

    Ok(ComparativeEvalReport {
        schema_version: COMPARATIVE_EVAL_SCHEMA_VERSION.to_string(),
        repo_sha: repo_sha.into(),
        fixture_registry: EvalFixtureRegistryReport {
            schema_version: corpus.registry.schema_version.clone(),
        },
        budget,
        metrics: metric_definitions(),
        summary: ComparativeEvalSummary {
            fixture_count: cases.len(),
            janus: janus_summary,
            raw: BTreeMap::new(),
            delta: BTreeMap::new(),
            false_causality_traps: BTreeMap::new(),
        },
        scenarios,
    })
}

pub fn build_empty_comparative_eval_report(
    corpus: &FixtureCorpus,
    selector: &EvalFixtureSelector,
    budget: EvalBudget,
    repo_sha: impl Into<String>,
) -> Result<ComparativeEvalReport, ComparativeEvalError> {
    let cases = selected_cases(corpus, selector)?;

    let scenarios = cases.iter().map(|case| scenario_report(case)).collect();

    Ok(ComparativeEvalReport {
        schema_version: COMPARATIVE_EVAL_SCHEMA_VERSION.to_string(),
        repo_sha: repo_sha.into(),
        fixture_registry: EvalFixtureRegistryReport {
            schema_version: corpus.registry.schema_version.clone(),
        },
        budget,
        metrics: metric_definitions(),
        summary: ComparativeEvalSummary {
            fixture_count: cases.len(),
            janus: BTreeMap::new(),
            raw: BTreeMap::new(),
            delta: BTreeMap::new(),
            false_causality_traps: BTreeMap::new(),
        },
        scenarios,
    })
}

pub fn run_janus_eval_submission(
    case: &FixtureCase,
    budget: EvalBudget,
) -> Result<EvalSubmission, ComparativeEvalError> {
    let query = janus_query_for_case(case, &budget)?;
    let bundle =
        get_evidence_bundle(query).map_err(|source| ComparativeEvalError::JanusAccess {
            scenario_id: case.manifest.id.clone(),
            source: Box::new(source),
        })?;

    normalize_janus_bundle(&case.manifest.id, budget, bundle)
}

pub fn measure_serialized_payload<T: Serialize>(
    payload: &T,
) -> Result<PayloadMeasurement, ComparativeEvalError> {
    let bytes = serde_json::to_vec(payload)
        .map_err(ComparativeEvalError::SerializePayload)?
        .len();
    let measured_tokens = u32::try_from(bytes.div_ceil(4))
        .map_err(|_| ComparativeEvalError::TokenEstimateOverflow { bytes })?;

    Ok(PayloadMeasurement {
        bytes,
        measured_tokens,
    })
}

pub fn format_text_report(report: &ComparativeEvalReport) -> String {
    let required_metrics = report
        .metrics
        .iter()
        .filter(|definition| definition.role == EvalMetricRole::Required)
        .count();
    let report_only_metrics = report.metrics.len().saturating_sub(required_metrics);

    let mut output = String::new();
    output.push_str("comparative eval v1\n");
    output.push_str(&format!("schema: {}\n", report.schema_version));
    output.push_str(&format!("repo_sha: {}\n", report.repo_sha));
    output.push_str(&format!(
        "fixtures: {} (registry schema {})\n",
        report.summary.fixture_count, report.fixture_registry.schema_version
    ));
    output.push_str(&format!(
        "budget: max_items={}, max_tokens={}\n",
        report.budget.max_items, report.budget.max_tokens
    ));
    output.push_str(&format!(
        "metrics: {} required, {} report_only\n",
        required_metrics, report_only_metrics
    ));
    output.push_str("scenarios:\n");

    for scenario in &report.scenarios {
        let janus_tokens = scenario
            .janus
            .get("submission")
            .and_then(|submission| submission.get("measured_tokens"))
            .and_then(Value::as_u64)
            .map(|tokens| format!(", janus_tokens={tokens}"))
            .unwrap_or_default();

        output.push_str(&format!(
            "- {} v{} ({}, {}, trap={}{})\n",
            scenario.id,
            scenario.scenario_version,
            scenario.failure_class,
            scenario.difficulty,
            scenario.false_causality_trap,
            janus_tokens
        ));
    }

    output
}

fn selected_cases<'a>(
    corpus: &'a FixtureCorpus,
    selector: &EvalFixtureSelector,
) -> Result<Vec<&'a FixtureCase>, ComparativeEvalError> {
    let cases: Vec<&FixtureCase> = corpus
        .cases
        .iter()
        .filter(|case| selector.matches(case))
        .collect();

    if cases.is_empty() {
        Err(ComparativeEvalError::NoFixturesSelected)
    } else {
        Ok(cases)
    }
}

fn scenario_report(case: &FixtureCase) -> ScenarioEvalReport {
    ScenarioEvalReport {
        id: case.manifest.id.clone(),
        scenario_schema_version: case.manifest.schema_version.clone(),
        scenario_version: case.manifest.version,
        failure_class: case.manifest.failure_class.clone(),
        difficulty: case.manifest.difficulty.clone(),
        false_causality_trap: case.manifest.false_causality_trap,
        janus: BTreeMap::new(),
        raw: BTreeMap::new(),
        comparison: BTreeMap::new(),
    }
}

fn janus_query_for_case(
    case: &FixtureCase,
    budget: &EvalBudget,
) -> Result<EvidenceQuery, ComparativeEvalError> {
    let time_window: TimeWindow = serde_json::from_value(case.manifest.time_window.clone())
        .map_err(|source| ComparativeEvalError::InvalidScenarioTimeWindow {
            scenario_id: case.manifest.id.clone(),
            source,
        })?;

    Ok(EvidenceQuery {
        intent: EvidenceQueryIntent {
            question: Some(case.manifest.question.clone()),
            hypothesis: None,
        },
        time_window,
        budget: EvidenceQueryBudget {
            max_items: budget.max_items,
            max_tokens: budget.max_tokens,
            min_counter_evidence_items: None,
            reserve_tokens_for_raw_refs: None,
        },
        scenario_id: Some(case.manifest.id.clone()),
        entities: Vec::new(),
        require_counter_evidence: false,
        require_raw_refs: true,
        freshness: FreshnessPreference::Any,
        privacy_scope: None,
    })
}

fn normalize_janus_bundle(
    scenario_id: &str,
    budget: EvalBudget,
    bundle: EvidenceBundle,
) -> Result<EvalSubmission, ComparativeEvalError> {
    let candidate_entities = candidate_entities_from_bundle(&bundle);
    let timeline_events = timeline_events_from_bundle(&bundle);
    let evidence_refs = source_refs_for_items(&bundle.items, |_| true);
    let counter_evidence_refs = source_refs_for_items(&bundle.items, is_counter_evidence_item);
    let missing_data_refs = source_refs_for_items(&bundle.items, is_missing_data_item);
    let serialized_context =
        serde_json::to_value(&bundle).map_err(ComparativeEvalError::SerializePayload)?;

    EvalSubmission::from_serialized_context(EvalSubmissionInput {
        scenario_id: scenario_id.to_string(),
        access_path: EvalAccessPath::Janus,
        budget,
        serialized_context,
        candidate_entities,
        timeline_events,
        evidence_refs,
        counter_evidence_refs,
        missing_data_refs,
    })
}

fn candidate_entities_from_bundle(bundle: &EvidenceBundle) -> Vec<EvalCandidateEntity> {
    let mut drafts = BTreeMap::<String, CandidateEntityDraft>::new();

    for (item_index, item) in bundle.items.iter().enumerate() {
        for entity in &item.entities {
            let draft = drafts
                .entry(entity.clone())
                .or_insert_with(|| CandidateEntityDraft {
                    entity: entity.clone(),
                    first_item_index: item_index,
                    score: item.strength.0,
                });
            draft.first_item_index = draft.first_item_index.min(item_index);
            draft.score = draft.score.max(item.strength.0);
        }
    }

    let mut ranked = drafts.into_values().collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        left.first_item_index
            .cmp(&right.first_item_index)
            .then_with(|| right.score.total_cmp(&left.score))
            .then_with(|| left.entity.cmp(&right.entity))
    });

    ranked
        .into_iter()
        .enumerate()
        .map(|(index, draft)| EvalCandidateEntity {
            entity: draft.entity,
            rank: Some((index + 1) as u32),
            score: Some(draft.score),
        })
        .collect()
}

fn timeline_events_from_bundle(bundle: &EvidenceBundle) -> Vec<EvalTimelineEvent> {
    let mut events = bundle
        .items
        .iter()
        .map(|item| EvalTimelineEvent {
            t: item.time_window.start.clone(),
            marker: timeline_marker_for_item(item).to_string(),
            entity: item
                .entities
                .first()
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
            source_ref: item.source_refs.iter().next().map(eval_source_ref),
        })
        .collect::<Vec<_>>();

    events.sort_by(|left, right| {
        left.t
            .cmp(&right.t)
            .then_with(|| left.marker.cmp(&right.marker))
            .then_with(|| left.entity.cmp(&right.entity))
    });

    events
}

fn source_refs_for_items(
    items: &[EvidenceItem],
    predicate: impl Fn(&EvidenceItem) -> bool,
) -> Vec<EvalSourceRef> {
    let mut seen = BTreeSet::<(String, String)>::new();
    let mut refs = Vec::new();

    for item in items.iter().filter(|item| predicate(item)) {
        for source_ref in item.source_refs.iter() {
            let eval_ref = eval_source_ref(source_ref);
            if seen.insert((eval_ref.signal.clone(), eval_ref.r#ref.clone())) {
                refs.push(eval_ref);
            }
        }
    }

    refs
}

fn eval_source_ref(source_ref: &SourceRef) -> EvalSourceRef {
    EvalSourceRef {
        signal: source_signal_name(source_ref.signal).to_string(),
        r#ref: source_ref.r#ref.clone(),
    }
}

fn is_counter_evidence_item(item: &EvidenceItem) -> bool {
    item.kind == EvidenceKind::CounterEvidence
        || matches!(
            item.direction,
            EvidenceDirection::Weakens | EvidenceDirection::Contradicts
        )
}

fn is_missing_data_item(item: &EvidenceItem) -> bool {
    item.kind == EvidenceKind::MissingData
        || !item.missing_data.is_empty()
        || item
            .source_refs
            .iter()
            .any(|source_ref| source_ref.signal == SourceSignal::TelemetryGap)
}

fn timeline_marker_for_item(item: &EvidenceItem) -> &'static str {
    if is_missing_data_item(item) {
        return "data-gap";
    }

    if is_counter_evidence_item(item) {
        return "non-causal-change";
    }

    match item.kind {
        EvidenceKind::ChangeEvent => "change",
        EvidenceKind::DependencyEdge => "propagation",
        EvidenceKind::PreviousIncident => "recovery",
        EvidenceKind::MetricAnomaly
        | EvidenceKind::TraceExemplar
        | EvidenceKind::LogCluster
        | EvidenceKind::ProfileHotspot => "symptom",
        EvidenceKind::CounterEvidence => "non-causal-change",
        EvidenceKind::MissingData => "data-gap",
    }
}

#[derive(Debug)]
struct CandidateEntityDraft {
    entity: String,
    first_item_index: usize,
    score: f64,
}

impl fmt::Display for ComparativeEvalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComparativeEvalError::FixtureCorpusLoad(error) => {
                write!(formatter, "failed to load fixture corpus: {error}")
            }
            ComparativeEvalError::NoFixturesSelected => {
                write!(
                    formatter,
                    "no fixtures matched the comparative eval selector"
                )
            }
            ComparativeEvalError::InvalidScenarioTimeWindow {
                scenario_id,
                source,
            } => write!(
                formatter,
                "invalid scenario time_window for {scenario_id}: {source}"
            ),
            ComparativeEvalError::JanusAccess {
                scenario_id,
                source,
            } => write!(formatter, "Janus access failed for {scenario_id}: {source}"),
            ComparativeEvalError::SerializePayload(error) => {
                write!(formatter, "failed to serialize measured payload: {error}")
            }
            ComparativeEvalError::TokenEstimateOverflow { bytes } => {
                write!(formatter, "token estimate overflows u32 for {bytes} bytes")
            }
        }
    }
}

impl std::error::Error for ComparativeEvalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ComparativeEvalError::FixtureCorpusLoad(error) => Some(error),
            ComparativeEvalError::InvalidScenarioTimeWindow { source, .. } => Some(source),
            ComparativeEvalError::JanusAccess { source, .. } => Some(source.as_ref()),
            ComparativeEvalError::SerializePayload(error) => Some(error),
            ComparativeEvalError::NoFixturesSelected
            | ComparativeEvalError::TokenEstimateOverflow { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{EvidenceBudget, EvidenceFreshness, SourceRefs, UnitInterval};
    use serde_json::json;
    use std::path::Path;

    const TEST_SHA: &str = "test-sha";

    #[test]
    fn empty_report_includes_all_fixture_versions_by_default() {
        let corpus = test_corpus();
        let report = build_empty_comparative_eval_report(
            &corpus,
            &EvalFixtureSelector::default(),
            EvalBudget::default(),
            TEST_SHA,
        )
        .expect("report should build");

        assert_eq!(report.schema_version, COMPARATIVE_EVAL_SCHEMA_VERSION);
        assert_eq!(report.fixture_registry.schema_version, "fixtures/v1");
        assert_eq!(report.summary.fixture_count, corpus.cases.len());
        assert_eq!(report.scenarios.len(), corpus.cases.len());
        assert!(
            report
                .scenarios
                .iter()
                .all(|scenario| scenario.scenario_schema_version == "fixtures/v1")
        );
        assert!(
            report
                .scenarios
                .iter()
                .all(|scenario| scenario.scenario_version >= 1)
        );
    }

    #[test]
    fn selector_filters_by_fixture_failure_class_difficulty_and_trap_flag() {
        let corpus = test_corpus();
        let report = build_empty_comparative_eval_report(
            &corpus,
            &EvalFixtureSelector {
                fixture_id: None,
                capability: None,
                failure_class: Some("coincidental-correlation".to_string()),
                difficulty: Some("hard".to_string()),
                false_causality_trap: Some(true),
            },
            EvalBudget::default(),
            TEST_SHA,
        )
        .expect("report should build");

        assert_eq!(report.summary.fixture_count, 1);
        assert_eq!(report.scenarios[0].id, "coincidental-deploy-trap");
        assert!(report.scenarios[0].false_causality_trap);

        let single_fixture = build_empty_comparative_eval_report(
            &corpus,
            &EvalFixtureSelector {
                fixture_id: Some("deploy-bad-rollout".to_string()),
                ..EvalFixtureSelector::default()
            },
            EvalBudget::default(),
            TEST_SHA,
        )
        .expect("single-fixture report should build");
        assert_eq!(single_fixture.summary.fixture_count, 1);
        assert_eq!(single_fixture.scenarios[0].id, "deploy-bad-rollout");
    }

    #[test]
    fn metric_definitions_split_required_and_report_only_metrics() {
        let definitions = metric_definitions();
        let required: Vec<EvalMetric> = definitions
            .iter()
            .filter(|definition| definition.role == EvalMetricRole::Required)
            .map(|definition| definition.metric)
            .collect();
        let report_only: Vec<EvalMetric> = definitions
            .iter()
            .filter(|definition| definition.role == EvalMetricRole::ReportOnly)
            .map(|definition| definition.metric)
            .collect();

        assert_eq!(
            required,
            vec![
                EvalMetric::SuspiciousEntityAccuracy,
                EvalMetric::FalseCausalityRisk,
                EvalMetric::MissingDataAwareness,
                EvalMetric::Auditability,
                EvalMetric::TokenEfficiency,
            ]
        );
        assert_eq!(report_only, vec![EvalMetric::TimelineQuality]);
    }

    #[test]
    fn shared_payload_measurement_uses_compact_json_bytes() {
        let payload = json!({
            "path": "janus",
            "items": [1, 2, 3]
        });
        let expected_bytes = serde_json::to_vec(&payload)
            .expect("payload should serialize")
            .len();

        let measurement =
            measure_serialized_payload(&payload).expect("measurement should serialize");

        assert_eq!(measurement.bytes, expected_bytes);
        assert_eq!(
            measurement.measured_tokens as usize,
            expected_bytes.div_ceil(4)
        );
    }

    #[test]
    fn report_serializes_to_v1_shape_with_empty_score_maps() {
        let corpus = test_corpus();
        let report = build_empty_comparative_eval_report(
            &corpus,
            &EvalFixtureSelector {
                fixture_id: Some("missing-data-gap".to_string()),
                ..EvalFixtureSelector::default()
            },
            EvalBudget {
                max_items: 4,
                max_tokens: 800,
            },
            TEST_SHA,
        )
        .expect("report should build");

        let value = serde_json::to_value(&report).expect("report should serialize");
        assert_eq!(value["schema_version"], "comparative-eval/v1");
        assert_eq!(value["repo_sha"], TEST_SHA);
        assert_eq!(value["fixture_registry"]["schema_version"], "fixtures/v1");
        assert_eq!(value["budget"]["max_items"], 4);
        assert_eq!(value["budget"]["max_tokens"], 800);
        assert_eq!(value["summary"]["fixture_count"], 1);
        assert_eq!(value["scenarios"][0]["id"], "missing-data-gap");
        assert_eq!(
            value["scenarios"][0]["scenario_schema_version"],
            "fixtures/v1"
        );
        assert_eq!(value["scenarios"][0]["scenario_version"], 1);
        assert!(
            value["scenarios"][0]["janus"]
                .as_object()
                .unwrap()
                .is_empty()
        );
        assert!(value["scenarios"][0]["raw"].as_object().unwrap().is_empty());
        assert!(
            value["scenarios"][0]["comparison"]
                .as_object()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn janus_report_populates_measured_submission_from_compiled_bundle() {
        let corpus = test_corpus();
        let budget = EvalBudget {
            max_items: 6,
            max_tokens: 1_200,
        };
        let report = build_comparative_eval_report_with_janus(
            &corpus,
            &EvalFixtureSelector {
                fixture_id: Some("deploy-bad-rollout".to_string()),
                ..EvalFixtureSelector::default()
            },
            budget.clone(),
            TEST_SHA,
        )
        .expect("Janus report should build");

        let submission = janus_submission(&report.scenarios[0]);
        let measurement = measure_serialized_payload(&submission.serialized_context)
            .expect("serialized context should measure");

        assert_eq!(submission.scenario_id, "deploy-bad-rollout");
        assert_eq!(submission.access_path, EvalAccessPath::Janus);
        assert_eq!(submission.budget, budget);
        assert_eq!(submission.measured_tokens, measurement.measured_tokens);
        assert!(
            submission
                .serialized_context
                .get("items")
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty())
        );
        assert!(!submission.candidate_entities.is_empty());
        assert!(!submission.evidence_refs.is_empty());
        assert!(!submission.timeline_events.is_empty());
        assert!(
            submission
                .timeline_events
                .windows(2)
                .all(|events| events[0].t <= events[1].t)
        );
    }

    #[test]
    fn janus_normalization_surfaces_selected_counter_evidence_refs() {
        let time_window = TimeWindow {
            start: "2026-06-08T15:00:00Z".to_string(),
            end: "2026-06-08T15:05:00Z".to_string(),
        };
        let bundle = EvidenceBundle {
            question: Some("Is the deploy causal?".to_string()),
            hypothesis: None,
            time_window: time_window.clone(),
            budget: EvidenceBudget {
                max_items: 1,
                max_tokens: 500,
                tokens_used: 42,
                items_dropped: 0,
                note: None,
            },
            items: vec![EvidenceItem {
                id: "ev-1".to_string(),
                claim: "The deploy is weakened by stable backend metrics.".to_string(),
                kind: EvidenceKind::CounterEvidence,
                direction: EvidenceDirection::Weakens,
                strength: UnitInterval(0.8),
                time_window,
                entities: vec!["service:search".to_string()],
                source_refs: SourceRefs(vec![SourceRef {
                    signal: SourceSignal::Metric,
                    r#ref: "search.error_rate@service:search".to_string(),
                }]),
                freshness: EvidenceFreshness::Settled,
                missing_data: Vec::new(),
                token_cost: 42,
                privacy_scope: "default".to_string(),
                confidence: BTreeMap::new(),
                note: None,
            }],
        };

        let submission = normalize_janus_bundle(
            "counter-case",
            EvalBudget {
                max_items: 1,
                max_tokens: 500,
            },
            bundle,
        )
        .expect("Janus bundle should normalize");

        assert_eq!(
            submission.counter_evidence_refs,
            vec![EvalSourceRef {
                signal: "metric".to_string(),
                r#ref: "search.error_rate@service:search".to_string(),
            }]
        );
    }

    #[test]
    fn janus_submission_surfaces_selected_missing_data_refs() {
        let corpus = test_corpus();
        let case = case_by_id(&corpus, "missing-data-gap");

        let submission = run_janus_eval_submission(
            case,
            EvalBudget {
                max_items: 6,
                max_tokens: 10_000,
            },
        )
        .expect("Janus submission should build");

        assert!(
            !submission.missing_data_refs.is_empty(),
            "missing-data refs should be normalized when the compiled bundle selects them"
        );
    }

    fn test_corpus() -> FixtureCorpus {
        FixtureCorpus::load(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("fixture corpus should load")
    }

    fn case_by_id<'a>(corpus: &'a FixtureCorpus, id: &str) -> &'a FixtureCase {
        corpus
            .cases
            .iter()
            .find(|case| case.manifest.id == id)
            .unwrap_or_else(|| panic!("missing fixture case {id}"))
    }

    fn janus_submission(scenario: &ScenarioEvalReport) -> EvalSubmission {
        serde_json::from_value(
            scenario
                .janus
                .get("submission")
                .cloned()
                .expect("scenario should include a Janus submission"),
        )
        .expect("Janus submission should deserialize")
    }
}
