use crate::fixture_validation::{FixtureCase, FixtureCorpus, FixtureCorpusLoadError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, fmt, path::Path};

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
    SerializePayload(serde_json::Error),
    TokenEstimateOverflow { bytes: usize },
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

pub fn load_empty_comparative_eval_report(
    root: impl AsRef<Path>,
    selector: &EvalFixtureSelector,
    budget: EvalBudget,
    repo_sha: impl Into<String>,
) -> Result<ComparativeEvalReport, ComparativeEvalError> {
    let corpus = FixtureCorpus::load(root).map_err(ComparativeEvalError::FixtureCorpusLoad)?;
    build_empty_comparative_eval_report(&corpus, selector, budget, repo_sha)
}

pub fn build_empty_comparative_eval_report(
    corpus: &FixtureCorpus,
    selector: &EvalFixtureSelector,
    budget: EvalBudget,
    repo_sha: impl Into<String>,
) -> Result<ComparativeEvalReport, ComparativeEvalError> {
    let cases: Vec<&FixtureCase> = corpus
        .cases
        .iter()
        .filter(|case| selector.matches(case))
        .collect();

    if cases.is_empty() {
        return Err(ComparativeEvalError::NoFixturesSelected);
    }

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
        output.push_str(&format!(
            "- {} v{} ({}, {}, trap={})\n",
            scenario.id,
            scenario.scenario_version,
            scenario.failure_class,
            scenario.difficulty,
            scenario.false_causality_trap
        ));
    }

    output
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
            ComparativeEvalError::SerializePayload(error) => Some(error),
            ComparativeEvalError::NoFixturesSelected
            | ComparativeEvalError::TokenEstimateOverflow { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn test_corpus() -> FixtureCorpus {
        FixtureCorpus::load(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("fixture corpus should load")
    }
}
