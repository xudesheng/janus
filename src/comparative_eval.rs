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
    references::{metric_series_ref, source_signal_name},
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
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
    scenario_id: String,
    access_path: EvalAccessPath,
    budget: EvalBudget,
    serialized_context: Value,
    measured_tokens: u32,
    #[serde(default)]
    candidate_entities: Vec<EvalCandidateEntity>,
    #[serde(default)]
    timeline_events: Vec<EvalTimelineEvent>,
    #[serde(default)]
    evidence_refs: Vec<EvalSourceRef>,
    #[serde(default)]
    counter_evidence_refs: Vec<EvalSourceRef>,
    #[serde(default)]
    missing_data_refs: Vec<EvalSourceRef>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawContextEnvelope {
    scenario_id: String,
    question: String,
    time_window: TimeWindow,
    records: Vec<RawContextRecord>,
    dropped_record_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawContextRecord {
    kind: RawContextRecordKind,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    t: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    entities: Vec<String>,
    #[serde(default)]
    source_refs: Vec<EvalSourceRef>,
    summary: String,
    payload: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RawContextRecordKind {
    Change,
    Log,
    Trace,
    MetricDelta,
    TelemetryGap,
}

#[derive(Debug, Clone, PartialEq)]
struct RawCandidateRecord {
    priority: u8,
    score: f64,
    sort_time: String,
    sort_id: String,
    record: RawContextRecord,
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
    BudgetExceeded {
        scenario_id: String,
        access_path: EvalAccessPath,
        measured_tokens: u32,
        max_tokens: u32,
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

    pub fn scenario_id(&self) -> &str {
        &self.scenario_id
    }

    pub fn access_path(&self) -> EvalAccessPath {
        self.access_path
    }

    pub fn budget(&self) -> &EvalBudget {
        &self.budget
    }

    pub fn serialized_context(&self) -> &Value {
        &self.serialized_context
    }

    pub fn measured_tokens(&self) -> u32 {
        self.measured_tokens
    }

    pub fn candidate_entities(&self) -> &[EvalCandidateEntity] {
        &self.candidate_entities
    }

    pub fn timeline_events(&self) -> &[EvalTimelineEvent] {
        &self.timeline_events
    }

    pub fn evidence_refs(&self) -> &[EvalSourceRef] {
        &self.evidence_refs
    }

    pub fn counter_evidence_refs(&self) -> &[EvalSourceRef] {
        &self.counter_evidence_refs
    }

    pub fn missing_data_refs(&self) -> &[EvalSourceRef] {
        &self.missing_data_refs
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

pub fn load_comparative_eval_report(
    root: impl AsRef<Path>,
    selector: &EvalFixtureSelector,
    budget: EvalBudget,
    repo_sha: impl Into<String>,
) -> Result<ComparativeEvalReport, ComparativeEvalError> {
    let corpus = FixtureCorpus::load(root).map_err(ComparativeEvalError::FixtureCorpusLoad)?;
    build_comparative_eval_report(&corpus, selector, budget, repo_sha)
}

pub fn build_comparative_eval_report(
    corpus: &FixtureCorpus,
    selector: &EvalFixtureSelector,
    budget: EvalBudget,
    repo_sha: impl Into<String>,
) -> Result<ComparativeEvalReport, ComparativeEvalError> {
    let cases = selected_cases(corpus, selector)?;
    let mut scenarios = Vec::new();
    let mut total_janus_tokens = 0u32;
    let mut total_raw_tokens = 0u32;

    for case in &cases {
        let janus_submission = run_janus_eval_submission(case, budget.clone())?;
        total_janus_tokens = total_janus_tokens.saturating_add(janus_submission.measured_tokens());

        let raw_submission = run_raw_eval_submission(case, budget.clone())?;
        total_raw_tokens = total_raw_tokens.saturating_add(raw_submission.measured_tokens());

        let mut report = scenario_report(case);
        insert_submission(&mut report.janus, &janus_submission)?;
        insert_submission(&mut report.raw, &raw_submission)?;
        scenarios.push(report);
    }

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
            janus: access_summary(cases.len(), total_janus_tokens),
            raw: access_summary(cases.len(), total_raw_tokens),
            delta: BTreeMap::new(),
            false_causality_traps: BTreeMap::new(),
        },
        scenarios,
    })
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
        total_janus_tokens = total_janus_tokens.saturating_add(submission.measured_tokens());

        let mut report = scenario_report(case);
        insert_submission(&mut report.janus, &submission)?;
        scenarios.push(report);
    }

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
            janus: access_summary(cases.len(), total_janus_tokens),
            raw: BTreeMap::new(),
            delta: BTreeMap::new(),
            false_causality_traps: BTreeMap::new(),
        },
        scenarios,
    })
}

#[cfg(test)]
fn build_empty_comparative_eval_report(
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
    let aliases = resource_entity_aliases(&case.input);
    let bundle =
        get_evidence_bundle(query).map_err(|source| ComparativeEvalError::JanusAccess {
            scenario_id: case.manifest.id.clone(),
            source: Box::new(source),
        })?;

    normalize_janus_bundle(&case.manifest.id, budget, bundle, &aliases)
}

pub fn run_raw_eval_submission(
    case: &FixtureCase,
    budget: EvalBudget,
) -> Result<EvalSubmission, ComparativeEvalError> {
    let time_window: TimeWindow = serde_json::from_value(case.manifest.time_window.clone())
        .map_err(|source| ComparativeEvalError::InvalidScenarioTimeWindow {
            scenario_id: case.manifest.id.clone(),
            source,
        })?;
    let aliases = resource_entity_aliases(&case.input);
    let candidates = raw_candidate_records(case, &time_window, &aliases);
    let envelope = select_raw_context(case, time_window, &budget, candidates)?;
    normalize_raw_context(&case.manifest.id, budget, envelope)
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
        let raw_tokens = scenario
            .raw
            .get("submission")
            .and_then(|submission| submission.get("measured_tokens"))
            .and_then(Value::as_u64)
            .map(|tokens| format!(", raw_tokens={tokens}"))
            .unwrap_or_default();

        output.push_str(&format!(
            "- {} v{} ({}, {}, trap={}{}{})\n",
            scenario.id,
            scenario.scenario_version,
            scenario.failure_class,
            scenario.difficulty,
            scenario.false_causality_trap,
            janus_tokens,
            raw_tokens
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

fn insert_submission(
    target: &mut BTreeMap<String, Value>,
    submission: &EvalSubmission,
) -> Result<(), ComparativeEvalError> {
    target.insert(
        "submission".to_string(),
        serde_json::to_value(submission).map_err(ComparativeEvalError::SerializePayload)?,
    );
    Ok(())
}

fn access_summary(submission_count: usize, measured_tokens: u32) -> BTreeMap<String, Value> {
    let mut summary = BTreeMap::new();
    summary.insert(
        "submission_count".to_string(),
        Value::from(submission_count),
    );
    summary.insert("measured_tokens".to_string(), Value::from(measured_tokens));
    summary
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
    aliases: &BTreeMap<String, String>,
) -> Result<EvalSubmission, ComparativeEvalError> {
    let candidate_entities = candidate_entities_from_bundle(&bundle, aliases);
    let timeline_events = timeline_events_from_bundle(&bundle, aliases);
    let evidence_refs = source_refs_for_items(&bundle.items, |_| true);
    let counter_evidence_refs = source_refs_for_items(&bundle.items, is_counter_evidence_item);
    let missing_data_refs = source_refs_for_items(&bundle.items, is_missing_data_item);
    let serialized_context =
        serde_json::to_value(&bundle).map_err(ComparativeEvalError::SerializePayload)?;

    let submission = EvalSubmission::from_serialized_context(EvalSubmissionInput {
        scenario_id: scenario_id.to_string(),
        access_path: EvalAccessPath::Janus,
        budget,
        serialized_context,
        candidate_entities,
        timeline_events,
        evidence_refs,
        counter_evidence_refs,
        missing_data_refs,
    })?;
    ensure_submission_budget(&submission)?;
    Ok(submission)
}

fn normalize_raw_context(
    scenario_id: &str,
    budget: EvalBudget,
    envelope: RawContextEnvelope,
) -> Result<EvalSubmission, ComparativeEvalError> {
    let candidate_entities = candidate_entities_from_raw_records(&envelope.records);
    let timeline_events = timeline_events_from_raw_records(&envelope.records);
    let evidence_refs = source_refs_from_raw_records(&envelope.records, |_| true);
    let counter_evidence_refs = Vec::new();
    let missing_data_refs = source_refs_from_raw_records(&envelope.records, |record| {
        record.kind == RawContextRecordKind::TelemetryGap
            || record
                .source_refs
                .iter()
                .any(|source_ref| source_ref.signal == "telemetry_gap")
    });
    let serialized_context =
        serde_json::to_value(&envelope).map_err(ComparativeEvalError::SerializePayload)?;

    let submission = EvalSubmission::from_serialized_context(EvalSubmissionInput {
        scenario_id: scenario_id.to_string(),
        access_path: EvalAccessPath::Raw,
        budget,
        serialized_context,
        candidate_entities,
        timeline_events,
        evidence_refs,
        counter_evidence_refs,
        missing_data_refs,
    })?;
    ensure_submission_budget(&submission)?;
    Ok(submission)
}

fn raw_candidate_records(
    case: &FixtureCase,
    time_window: &TimeWindow,
    aliases: &BTreeMap<String, String>,
) -> Vec<RawCandidateRecord> {
    let mut candidates = Vec::new();
    push_raw_change_candidates(case, time_window, aliases, &mut candidates);
    push_raw_gap_candidates(case, time_window, aliases, &mut candidates);
    push_raw_log_candidates(case, time_window, aliases, &mut candidates);
    push_raw_trace_candidates(case, time_window, aliases, &mut candidates);
    push_raw_metric_candidates(case, time_window, aliases, &mut candidates);

    candidates.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| right.score.total_cmp(&left.score))
            .then_with(|| left.sort_time.cmp(&right.sort_time))
            .then_with(|| left.sort_id.cmp(&right.sort_id))
    });

    candidates
}

fn push_raw_change_candidates(
    case: &FixtureCase,
    time_window: &TimeWindow,
    aliases: &BTreeMap<String, String>,
    candidates: &mut Vec<RawCandidateRecord>,
) {
    let Some(changes) = array_at(&case.input, "changes") else {
        return;
    };

    for (index, change) in changes.iter().enumerate() {
        let Some(t) = str_field(change, "t") else {
            continue;
        };
        if !timestamp_in_window(t, time_window) {
            continue;
        }

        let id = str_field(change, "id")
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("raw-change-{index}"));
        let entities = canonical_entities(entities_from_raw_value(change), aliases);
        let summary = format!(
            "{} change for {}: {}",
            str_field(change, "kind").unwrap_or("unknown"),
            display_entities(&entities),
            str_field(change, "summary").unwrap_or("")
        );
        let record = RawContextRecord {
            kind: RawContextRecordKind::Change,
            id: id.clone(),
            t: Some(t.to_string()),
            entities,
            source_refs: vec![EvalSourceRef {
                signal: "change".to_string(),
                r#ref: id.clone(),
            }],
            summary,
            payload: change.clone(),
        };
        candidates.push(RawCandidateRecord {
            priority: 0,
            score: 1.0,
            sort_time: t.to_string(),
            sort_id: id,
            record,
        });
    }
}

fn push_raw_gap_candidates(
    case: &FixtureCase,
    time_window: &TimeWindow,
    aliases: &BTreeMap<String, String>,
    candidates: &mut Vec<RawCandidateRecord>,
) {
    let mut seen_gap_refs = BTreeSet::new();

    if let Some(gaps) = array_at(&case.input, "telemetry_gaps") {
        for (index, gap) in gaps.iter().enumerate() {
            let Some(start) = str_field(gap, "start") else {
                continue;
            };
            let Some(end) = str_field(gap, "end") else {
                continue;
            };
            if !window_overlaps(start, end, time_window) {
                continue;
            }

            let id = str_field(gap, "id")
                .map(ToString::to_string)
                .unwrap_or_else(|| format!("raw-gap-{index}"));
            seen_gap_refs.insert(id.clone());
            let entities = canonical_entities(entities_from_raw_value(gap), aliases);
            let record = RawContextRecord {
                kind: RawContextRecordKind::TelemetryGap,
                id: id.clone(),
                t: Some(start.to_string()),
                entities,
                source_refs: vec![EvalSourceRef {
                    signal: "telemetry_gap".to_string(),
                    r#ref: id.clone(),
                }],
                summary: str_field(gap, "note")
                    .unwrap_or("telemetry gap overlaps the incident window")
                    .to_string(),
                payload: gap.clone(),
            };
            candidates.push(RawCandidateRecord {
                priority: 1,
                score: 1.0,
                sort_time: start.to_string(),
                sort_id: id,
                record,
            });
        }
    }

    let Some(metrics) = array_at(&case.input, "metrics") else {
        return;
    };

    for (index, metric) in metrics.iter().enumerate() {
        let Some(gap) = metric.get("_gap") else {
            continue;
        };
        let Some(start) = str_field(gap, "start") else {
            continue;
        };
        let Some(end) = str_field(gap, "end") else {
            continue;
        };
        if !window_overlaps(start, end, time_window) {
            continue;
        }

        let Some(gap_ref) = str_field(gap, "ref") else {
            continue;
        };
        if seen_gap_refs.contains(gap_ref) {
            continue;
        }

        let metric_name = str_field(metric, "name").unwrap_or("metric");
        let entity = str_field(metric, "entity").unwrap_or("unknown");
        let canonical = canonical_entity(entity, aliases);
        let id = format!("raw-metric-gap-{index}-{gap_ref}");
        let record = RawContextRecord {
            kind: RawContextRecordKind::TelemetryGap,
            id: id.clone(),
            t: Some(start.to_string()),
            entities: vec![canonical.clone()],
            source_refs: vec![EvalSourceRef {
                signal: "telemetry_gap".to_string(),
                r#ref: gap_ref.to_string(),
            }],
            summary: format!("{metric_name} has a telemetry gap for {canonical}"),
            payload: json_object([
                ("metric", Value::String(metric_name.to_string())),
                ("entity", Value::String(canonical)),
                ("gap", gap.clone()),
            ]),
        };
        candidates.push(RawCandidateRecord {
            priority: 1,
            score: 0.95,
            sort_time: start.to_string(),
            sort_id: id,
            record,
        });
    }
}

fn push_raw_log_candidates(
    case: &FixtureCase,
    time_window: &TimeWindow,
    aliases: &BTreeMap<String, String>,
    candidates: &mut Vec<RawCandidateRecord>,
) {
    let Some(logs) = array_at(&case.input, "logs") else {
        return;
    };

    for (index, log) in logs.iter().enumerate() {
        let Some(t) = str_field(log, "t") else {
            continue;
        };
        if !timestamp_in_window(t, time_window) {
            continue;
        }

        let severity = str_field(log, "severity").unwrap_or("");
        let severity_score = match severity {
            "ERROR" => 1.0,
            "WARN" | "WARNING" => 0.65,
            _ => continue,
        };
        let id = str_field(log, "id")
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("raw-log-{index}"));
        let entities = canonical_entities(entities_from_raw_value(log), aliases);
        let body = str_field(log, "body").unwrap_or("");
        let record = RawContextRecord {
            kind: RawContextRecordKind::Log,
            id: id.clone(),
            t: Some(t.to_string()),
            entities,
            source_refs: vec![EvalSourceRef {
                signal: "log".to_string(),
                r#ref: id.clone(),
            }],
            summary: format!("{severity} log: {body}"),
            payload: log.clone(),
        };
        candidates.push(RawCandidateRecord {
            priority: if severity == "ERROR" { 2 } else { 4 },
            score: severity_score,
            sort_time: t.to_string(),
            sort_id: id,
            record,
        });
    }
}

fn push_raw_trace_candidates(
    case: &FixtureCase,
    time_window: &TimeWindow,
    aliases: &BTreeMap<String, String>,
    candidates: &mut Vec<RawCandidateRecord>,
) {
    let Some(traces) = array_at(&case.input, "traces") else {
        return;
    };

    for (index, trace) in traces.iter().enumerate() {
        let Some(trace_id) = str_field(trace, "trace_id") else {
            continue;
        };
        let Some(spans) = array_at(trace, "spans") else {
            continue;
        };

        let mut failed_spans = Vec::new();
        let mut source_refs = Vec::new();
        let mut entities = Vec::new();
        let mut starts = Vec::new();

        for span in spans {
            let Some(start) = str_field(span, "start") else {
                continue;
            };
            if !timestamp_in_window(start, time_window) {
                continue;
            }
            starts.push(start.to_string());

            if str_field(span, "status") == Some("ERROR") {
                failed_spans.push(compact_span_payload(span, aliases));
            }

            if let Some(span_id) = str_field(span, "span_id") {
                source_refs.push(EvalSourceRef {
                    signal: "trace".to_string(),
                    r#ref: format!("{trace_id}/{span_id}"),
                });
            }
            entities.extend(canonical_entities(entities_from_span(span), aliases));
        }

        if failed_spans.is_empty() {
            continue;
        }

        starts.sort();
        let t = starts
            .first()
            .cloned()
            .unwrap_or_else(|| time_window.start.clone());
        let id = format!("raw-trace-{index}-{trace_id}");
        let entities = dedupe_strings(entities);
        let record = RawContextRecord {
            kind: RawContextRecordKind::Trace,
            id: id.clone(),
            t: Some(t.clone()),
            entities: entities.clone(),
            source_refs: dedupe_eval_refs(source_refs),
            summary: format!(
                "failed trace {trace_id} touches {} with {} failed span(s)",
                display_entities(&entities),
                failed_spans.len()
            ),
            payload: json_object([
                ("trace_id", Value::String(trace_id.to_string())),
                (
                    "exemplar_of",
                    trace
                        .get("exemplar_of")
                        .cloned()
                        .unwrap_or_else(|| Value::String("unknown".to_string())),
                ),
                ("failed_spans", Value::Array(failed_spans)),
            ]),
        };
        candidates.push(RawCandidateRecord {
            priority: 3,
            score: 1.0,
            sort_time: t,
            sort_id: id,
            record,
        });
    }
}

fn push_raw_metric_candidates(
    case: &FixtureCase,
    time_window: &TimeWindow,
    aliases: &BTreeMap<String, String>,
    candidates: &mut Vec<RawCandidateRecord>,
) {
    let Some(metrics) = array_at(&case.input, "metrics") else {
        return;
    };

    for (index, metric) in metrics.iter().enumerate() {
        let Some(name) = str_field(metric, "name") else {
            continue;
        };
        let Some(entity) = str_field(metric, "entity") else {
            continue;
        };
        let Some(points) = array_at(metric, "points") else {
            continue;
        };

        let mut selected_points = points
            .iter()
            .filter(|point| {
                str_field(point, "t")
                    .is_some_and(|timestamp| timestamp_in_window(timestamp, time_window))
            })
            .cloned()
            .collect::<Vec<_>>();
        if selected_points.len() < 2 {
            selected_points = points.clone();
        }
        selected_points.sort_by(|left, right| {
            str_field(left, "t")
                .unwrap_or("")
                .cmp(str_field(right, "t").unwrap_or(""))
        });

        let Some((first_t, first_v)) = point_time_value(selected_points.first()) else {
            continue;
        };
        let Some((last_t, last_v)) = point_time_value(selected_points.last()) else {
            continue;
        };
        let delta = (last_v - first_v).abs();
        if delta <= 0.0 {
            continue;
        }
        let relative_delta = delta / first_v.abs().max(0.001);
        let score = relative_delta.max(delta);
        let canonical = canonical_entity(entity, aliases);
        let metric_ref = metric_series_ref(name, entity);
        let id = format!("raw-metric-{index}-{metric_ref}");
        let record = RawContextRecord {
            kind: RawContextRecordKind::MetricDelta,
            id: id.clone(),
            t: Some(last_t.to_string()),
            entities: vec![canonical.clone()],
            source_refs: vec![EvalSourceRef {
                signal: "metric".to_string(),
                r#ref: metric_ref,
            }],
            summary: format!("{name} for {canonical} changed from {first_v:.3} to {last_v:.3}"),
            payload: json_object([
                ("name", Value::String(name.to_string())),
                ("entity", Value::String(canonical)),
                (
                    "unit",
                    metric
                        .get("unit")
                        .cloned()
                        .unwrap_or_else(|| Value::String("unknown".to_string())),
                ),
                (
                    "first",
                    json_object([
                        ("t", Value::String(first_t.to_string())),
                        ("v", Value::from(first_v)),
                    ]),
                ),
                (
                    "last",
                    json_object([
                        ("t", Value::String(last_t.to_string())),
                        ("v", Value::from(last_v)),
                    ]),
                ),
                ("absolute_delta", Value::from(delta)),
                ("relative_delta", Value::from(relative_delta)),
            ]),
        };
        candidates.push(RawCandidateRecord {
            priority: 5,
            score,
            sort_time: last_t.to_string(),
            sort_id: id,
            record,
        });
    }
}

fn select_raw_context(
    case: &FixtureCase,
    time_window: TimeWindow,
    budget: &EvalBudget,
    candidates: Vec<RawCandidateRecord>,
) -> Result<RawContextEnvelope, ComparativeEvalError> {
    let total_candidates = candidates.len();
    let mut selected = Vec::new();

    for candidate in candidates {
        if selected.len() >= budget.max_items as usize {
            continue;
        }

        let mut trial_records = selected.clone();
        trial_records.push(candidate.record);
        let trial = raw_context_envelope(
            case,
            time_window.clone(),
            trial_records.clone(),
            total_candidates.saturating_sub(trial_records.len()),
        );
        let trial_value =
            serde_json::to_value(&trial).map_err(ComparativeEvalError::SerializePayload)?;
        let measurement = measure_serialized_payload(&trial_value)?;
        if measurement.measured_tokens <= budget.max_tokens {
            selected = trial_records;
        }
    }

    let dropped_record_count = total_candidates.saturating_sub(selected.len());
    let envelope = raw_context_envelope(case, time_window, selected, dropped_record_count);
    let value = serde_json::to_value(&envelope).map_err(ComparativeEvalError::SerializePayload)?;
    let measurement = measure_serialized_payload(&value)?;
    if measurement.measured_tokens > budget.max_tokens {
        return Err(ComparativeEvalError::BudgetExceeded {
            scenario_id: case.manifest.id.clone(),
            access_path: EvalAccessPath::Raw,
            measured_tokens: measurement.measured_tokens,
            max_tokens: budget.max_tokens,
        });
    }

    Ok(envelope)
}

fn raw_context_envelope(
    case: &FixtureCase,
    time_window: TimeWindow,
    records: Vec<RawContextRecord>,
    dropped_record_count: usize,
) -> RawContextEnvelope {
    RawContextEnvelope {
        scenario_id: case.manifest.id.clone(),
        question: case.manifest.question.clone(),
        time_window,
        records,
        dropped_record_count,
    }
}

fn ensure_submission_budget(submission: &EvalSubmission) -> Result<(), ComparativeEvalError> {
    if submission.measured_tokens() > submission.budget().max_tokens {
        return Err(ComparativeEvalError::BudgetExceeded {
            scenario_id: submission.scenario_id().to_string(),
            access_path: submission.access_path(),
            measured_tokens: submission.measured_tokens(),
            max_tokens: submission.budget().max_tokens,
        });
    }

    Ok(())
}

fn candidate_entities_from_raw_records(records: &[RawContextRecord]) -> Vec<EvalCandidateEntity> {
    let mut drafts = BTreeMap::<String, CandidateEntityDraft>::new();

    for (record_index, record) in records.iter().enumerate() {
        for entity in &record.entities {
            let score = raw_record_entity_score(record.kind);
            let draft = drafts
                .entry(entity.clone())
                .or_insert_with(|| CandidateEntityDraft {
                    entity: entity.clone(),
                    first_item_index: record_index,
                    score,
                });
            draft.first_item_index = draft.first_item_index.min(record_index);
            draft.score = draft.score.max(score);
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

fn timeline_events_from_raw_records(records: &[RawContextRecord]) -> Vec<EvalTimelineEvent> {
    let mut events = records
        .iter()
        .filter_map(|record| {
            record.t.as_ref().map(|t| EvalTimelineEvent {
                t: t.clone(),
                marker: raw_timeline_marker(record.kind).to_string(),
                entity: record
                    .entities
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string()),
                source_ref: record.source_refs.first().cloned(),
            })
        })
        .collect::<Vec<_>>();

    // Fixture timestamps are normalized UTC strings, so lexical ordering is stable here.
    events.sort_by(|left, right| {
        left.t
            .cmp(&right.t)
            .then_with(|| left.marker.cmp(&right.marker))
            .then_with(|| left.entity.cmp(&right.entity))
    });

    events
}

fn source_refs_from_raw_records(
    records: &[RawContextRecord],
    predicate: impl Fn(&RawContextRecord) -> bool,
) -> Vec<EvalSourceRef> {
    let mut refs = Vec::new();
    for record in records.iter().filter(|record| predicate(record)) {
        refs.extend(record.source_refs.iter().cloned());
    }
    dedupe_eval_refs(refs)
}

fn raw_record_entity_score(kind: RawContextRecordKind) -> f64 {
    match kind {
        RawContextRecordKind::Change => 1.0,
        RawContextRecordKind::TelemetryGap => 0.95,
        RawContextRecordKind::Log => 0.9,
        RawContextRecordKind::Trace => 0.85,
        RawContextRecordKind::MetricDelta => 0.8,
    }
}

fn raw_timeline_marker(kind: RawContextRecordKind) -> &'static str {
    match kind {
        RawContextRecordKind::Change => "change",
        RawContextRecordKind::TelemetryGap => "data-gap",
        RawContextRecordKind::Log
        | RawContextRecordKind::Trace
        | RawContextRecordKind::MetricDelta => "symptom",
    }
}

fn resource_entity_aliases(input: &Value) -> BTreeMap<String, String> {
    let mut aliases = BTreeMap::new();

    if let Some(resources) = array_at(input, "resources") {
        for resource in resources {
            let Some(resource_id) = str_field(resource, "id") else {
                continue;
            };
            let Some(attributes) = resource.get("attributes").and_then(Value::as_object) else {
                continue;
            };
            let Some(service_name) = attributes.get("service.name").and_then(Value::as_str) else {
                continue;
            };

            let entity = match attributes.get("db.system").and_then(Value::as_str) {
                Some("redis") => format!("infra:{service_name}"),
                Some(_) => format!("db:{service_name}"),
                None => format!("service:{service_name}"),
            };
            aliases.insert(resource_id.to_string(), entity);
        }
    }

    aliases
}

fn canonical_entities(entities: Vec<String>, aliases: &BTreeMap<String, String>) -> Vec<String> {
    dedupe_strings(
        entities
            .iter()
            .map(|entity| canonical_entity(entity, aliases))
            .collect(),
    )
}

fn canonical_entity(entity: &str, aliases: &BTreeMap<String, String>) -> String {
    aliases
        .get(entity)
        .cloned()
        .unwrap_or_else(|| entity.to_string())
}

fn entities_from_raw_value(value: &Value) -> Vec<String> {
    let mut entities = Vec::new();
    push_str_field(&mut entities, value, "entity");
    push_str_field(&mut entities, value, "resource");
    push_str_array_field(&mut entities, value, "entities");
    push_str_array_field(&mut entities, value, "affected_entities");
    dedupe_strings(entities)
}

fn entities_from_span(span: &Value) -> Vec<String> {
    let mut entities = entities_from_raw_value(span);
    if let Some(attributes) = span.get("attributes").and_then(Value::as_object) {
        if let Some(peer) = attributes.get("peer.service").and_then(Value::as_str) {
            entities.push(format!("service:{peer}"));
        }
        if let Some(db_system) = attributes.get("db.system").and_then(Value::as_str) {
            if let Some(resource) = str_field(span, "resource") {
                entities.push(resource.to_string());
            } else {
                entities.push(format!("db:{db_system}"));
            }
        }
    }
    dedupe_strings(entities)
}

fn compact_span_payload(span: &Value, aliases: &BTreeMap<String, String>) -> Value {
    let mut payload = Map::new();

    for key in [
        "span_id",
        "parent_id",
        "name",
        "kind",
        "start",
        "end",
        "status",
    ] {
        if let Some(value) = span.get(key) {
            payload.insert(key.to_string(), value.clone());
        }
    }
    if let Some(resource) = str_field(span, "resource") {
        payload.insert(
            "entity".to_string(),
            Value::String(canonical_entity(resource, aliases)),
        );
    }
    if let Some(attributes) = span.get("attributes") {
        payload.insert("attributes".to_string(), attributes.clone());
    }

    Value::Object(payload)
}

fn point_time_value(point: Option<&Value>) -> Option<(&str, f64)> {
    let point = point?;
    let t = str_field(point, "t")?;
    let v = point.get("v")?.as_f64()?;
    Some((t, v))
}

fn timestamp_in_window(timestamp: &str, window: &TimeWindow) -> bool {
    window.start.as_str() <= timestamp && timestamp <= window.end.as_str()
}

fn window_overlaps(start: &str, end: &str, window: &TimeWindow) -> bool {
    start <= window.end.as_str() && window.start.as_str() <= end
}

fn array_at<'a>(value: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
    value.get(key).and_then(Value::as_array)
}

fn str_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn push_str_field(entities: &mut Vec<String>, value: &Value, field: &str) {
    if let Some(entity) = str_field(value, field)
        && !entity.trim().is_empty()
    {
        entities.push(entity.to_string());
    }
}

fn push_str_array_field(entities: &mut Vec<String>, value: &Value, field: &str) {
    if let Some(values) = value.get(field).and_then(Value::as_array) {
        for value in values {
            if let Some(entity) = value.as_str()
                && !entity.trim().is_empty()
            {
                entities.push(entity.to_string());
            }
        }
    }
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();
    for value in values {
        if !deduped.contains(&value) {
            deduped.push(value);
        }
    }
    deduped
}

fn dedupe_eval_refs(values: Vec<EvalSourceRef>) -> Vec<EvalSourceRef> {
    let mut seen = BTreeSet::<(String, String)>::new();
    let mut deduped = Vec::new();

    for value in values {
        if seen.insert((value.signal.clone(), value.r#ref.clone())) {
            deduped.push(value);
        }
    }

    deduped
}

fn display_entities(entities: &[String]) -> String {
    if entities.is_empty() {
        "unknown".to_string()
    } else {
        entities.join(", ")
    }
}

fn json_object<const N: usize>(entries: [(&str, Value); N]) -> Value {
    let mut object = Map::new();
    for (key, value) in entries {
        object.insert(key.to_string(), value);
    }
    Value::Object(object)
}

fn candidate_entities_from_bundle(
    bundle: &EvidenceBundle,
    aliases: &BTreeMap<String, String>,
) -> Vec<EvalCandidateEntity> {
    let mut drafts = BTreeMap::<String, CandidateEntityDraft>::new();

    for (item_index, item) in bundle.items.iter().enumerate() {
        for entity in &item.entities {
            let entity = canonical_entity(entity, aliases);
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

fn timeline_events_from_bundle(
    bundle: &EvidenceBundle,
    aliases: &BTreeMap<String, String>,
) -> Vec<EvalTimelineEvent> {
    let mut events = bundle
        .items
        .iter()
        .map(|item| EvalTimelineEvent {
            t: item.time_window.start.clone(),
            marker: timeline_marker_for_item(item).to_string(),
            entity: item
                .entities
                .first()
                .map(|entity| canonical_entity(entity, aliases))
                .unwrap_or_else(|| "unknown".to_string()),
            source_ref: item.source_refs.iter().next().map(eval_source_ref),
        })
        .collect::<Vec<_>>();

    // Fixture timestamps are normalized UTC strings, so lexical ordering is stable here.
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
            ComparativeEvalError::BudgetExceeded {
                scenario_id,
                access_path,
                measured_tokens,
                max_tokens,
            } => write!(
                formatter,
                "{access_path:?} access exceeded eval budget for {scenario_id}: measured_tokens={measured_tokens}, max_tokens={max_tokens}"
            ),
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
            | ComparativeEvalError::BudgetExceeded { .. }
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
        let measurement = measure_serialized_payload(submission.serialized_context())
            .expect("serialized context should measure");

        assert_eq!(submission.scenario_id(), "deploy-bad-rollout");
        assert_eq!(submission.access_path(), EvalAccessPath::Janus);
        assert_eq!(submission.budget(), &budget);
        assert_eq!(submission.measured_tokens(), measurement.measured_tokens);
        assert!(
            submission
                .serialized_context()
                .get("items")
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty())
        );
        assert!(!submission.candidate_entities().is_empty());
        assert!(!submission.evidence_refs().is_empty());
        assert!(!submission.timeline_events().is_empty());
        assert!(
            submission
                .timeline_events()
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
            &BTreeMap::new(),
        )
        .expect("Janus bundle should normalize");

        assert_eq!(
            submission.counter_evidence_refs(),
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
            !submission.missing_data_refs().is_empty(),
            "missing-data refs should be normalized when the compiled bundle selects them"
        );
    }

    #[test]
    fn janus_candidate_entities_dedupe_direct_resource_aliases() {
        let mut aliases = BTreeMap::new();
        aliases.insert(
            "res:redis-cache".to_string(),
            "infra:redis-cache".to_string(),
        );
        let time_window = TimeWindow {
            start: "2026-06-08T15:00:00Z".to_string(),
            end: "2026-06-08T15:05:00Z".to_string(),
        };
        let bundle = EvidenceBundle {
            question: Some("What is failing?".to_string()),
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
                claim: "redis-cache is unhealthy".to_string(),
                kind: EvidenceKind::MetricAnomaly,
                direction: EvidenceDirection::Supports,
                strength: UnitInterval(0.8),
                time_window,
                entities: vec![
                    "infra:redis-cache".to_string(),
                    "res:redis-cache".to_string(),
                ],
                source_refs: SourceRefs(vec![SourceRef {
                    signal: SourceSignal::Metric,
                    r#ref: "cache.hit_ratio@infra:redis-cache".to_string(),
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
            "alias-case",
            EvalBudget {
                max_items: 1,
                max_tokens: 500,
            },
            bundle,
            &aliases,
        )
        .expect("Janus bundle should normalize");

        assert_eq!(submission.candidate_entities().len(), 1);
        assert_eq!(
            submission.candidate_entities()[0].entity,
            "infra:redis-cache"
        );
    }

    #[test]
    fn full_report_populates_janus_and_raw_submissions() {
        let corpus = test_corpus();
        let report = build_comparative_eval_report(
            &corpus,
            &EvalFixtureSelector {
                fixture_id: Some("deploy-bad-rollout".to_string()),
                ..EvalFixtureSelector::default()
            },
            EvalBudget::default(),
            TEST_SHA,
        )
        .expect("full comparative report should build");

        assert_eq!(report.summary.fixture_count, 1);
        assert_eq!(report.summary.janus["submission_count"], 1);
        assert_eq!(report.summary.raw["submission_count"], 1);

        let scenario = &report.scenarios[0];
        assert_eq!(
            janus_submission(scenario).access_path(),
            EvalAccessPath::Janus
        );
        assert_eq!(raw_submission(scenario).access_path(), EvalAccessPath::Raw);
    }

    #[test]
    fn raw_submission_is_deterministic_budgeted_and_source_backed() {
        let corpus = test_corpus();
        let case = case_by_id(&corpus, "coincidental-deploy-trap");
        let budget = EvalBudget {
            max_items: 4,
            max_tokens: 900,
        };

        let first =
            run_raw_eval_submission(case, budget.clone()).expect("raw submission should build");
        let second = run_raw_eval_submission(case, budget.clone())
            .expect("raw submission should be deterministic");
        let envelope = raw_envelope(&first);

        assert_eq!(first, second);
        assert_eq!(first.access_path(), EvalAccessPath::Raw);
        assert_eq!(first.budget(), &budget);
        assert!(first.measured_tokens() <= budget.max_tokens);
        assert!(envelope.records.len() <= budget.max_items as usize);
        assert!(!first.candidate_entities().is_empty());
        assert!(!first.evidence_refs().is_empty());
        assert!(first.evidence_refs().iter().all(|source_ref| {
            matches!(
                source_ref.signal.as_str(),
                "change" | "log" | "trace" | "metric" | "telemetry_gap"
            )
        }));
    }

    #[test]
    fn raw_submission_ignores_expected_and_ground_truth() {
        let corpus = test_corpus();
        let case = case_by_id(&corpus, "deploy-bad-rollout");
        let mut poisoned = case.clone();
        poisoned.manifest.ground_truth = json!({
            "primary_cause_entity": "service:poison",
            "not_the_cause": ["service:checkout"]
        });
        poisoned.expected = json!({
            "evidence_bundle": {
                "items": [
                    {
                        "id": "ev-poison",
                        "entities": ["service:poison"]
                    }
                ]
            }
        });

        let original = run_raw_eval_submission(case, EvalBudget::default())
            .expect("raw submission should build");
        let poisoned = run_raw_eval_submission(&poisoned, EvalBudget::default())
            .expect("poisoned oracle fields should not affect raw submission");

        assert_eq!(original.serialized_context(), poisoned.serialized_context());
        assert_eq!(original.candidate_entities(), poisoned.candidate_entities());
    }

    #[test]
    fn raw_submission_surfaces_telemetry_gaps_as_missing_data_refs() {
        let corpus = test_corpus();
        let case = case_by_id(&corpus, "missing-data-gap");

        let submission = run_raw_eval_submission(
            case,
            EvalBudget {
                max_items: 6,
                max_tokens: 1_200,
            },
        )
        .expect("raw submission should build");

        assert!(
            !submission.missing_data_refs().is_empty(),
            "raw telemetry gaps should be normalized as missing-data refs"
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

    fn raw_submission(scenario: &ScenarioEvalReport) -> EvalSubmission {
        serde_json::from_value(
            scenario
                .raw
                .get("submission")
                .cloned()
                .expect("scenario should include a raw submission"),
        )
        .expect("raw submission should deserialize")
    }

    fn raw_envelope(submission: &EvalSubmission) -> RawContextEnvelope {
        serde_json::from_value(submission.serialized_context().clone())
            .expect("raw context should deserialize")
    }
}
