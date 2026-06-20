use crate::evidence::{
    EvidenceBundle, EvidenceDirection, EvidenceKind, SourceSignal, ValidationErrors,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs, io,
    path::{Component, Path, PathBuf},
};

const FIXTURE_SCHEMA_VERSION: &str = "fixtures/v1";
const DIFFICULTIES: &[&str] = &["baseline", "hard"];
const INPUT_KEYS: &[&str] = &[
    "resources",
    "traces",
    "metrics",
    "logs",
    "changes",
    "prior_incidents",
    "telemetry_gaps",
];
const EXPECTED_KEYS: &[&str] = &[
    "entities",
    "relationships",
    "anomaly_windows",
    "log_patterns",
    "evidence_bundle",
    "timeline",
    "suspected_causes",
    "next_checks",
    "entity_context",
    "related_anomalies",
    "window_comparison",
];
const RELATIONSHIP_TYPES: &[&str] = &[
    "calls",
    "depends-on",
    "runs-on",
    "owns",
    "deployed-as",
    "emits",
    "retries",
    "fans-out-to",
    "reads-from",
    "writes-to",
    "shares-resource-with",
];
const TIMELINE_MARKERS: &[&str] = &[
    "change",
    "symptom",
    "propagation",
    "recovery",
    "trigger",
    "amplification",
    "non-causal-change",
    "data-gap",
];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FixtureRegistry {
    pub schema_version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub failure_classes: Vec<String>,
    #[serde(default)]
    pub fixtures: Vec<FixtureRegistryEntry>,
    #[serde(default)]
    pub proposed: Vec<ProposedFixture>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FixtureRegistryEntry {
    pub id: String,
    pub path: String,
    pub failure_class: String,
    pub difficulty: String,
    pub false_causality_trap: bool,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProposedFixture {
    pub id: String,
    pub failure_class: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ScenarioManifest {
    pub id: String,
    pub title: String,
    pub version: u64,
    pub schema_version: String,
    pub failure_class: String,
    pub difficulty: String,
    pub false_causality_trap: bool,
    pub summary: String,
    pub question: String,
    pub time_window: Value,
    pub ground_truth: Value,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub expected: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FixtureCase {
    pub registry_entry: FixtureRegistryEntry,
    pub directory: PathBuf,
    pub manifest: ScenarioManifest,
    pub input: Value,
    pub expected: Value,
}

#[derive(Debug, Clone)]
pub struct FixtureCorpus {
    pub root: PathBuf,
    pub fixtures_root: PathBuf,
    pub registry: FixtureRegistry,
    pub cases: Vec<FixtureCase>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FixtureSelector {
    pub fixture_id: Option<String>,
    pub capability: Option<String>,
    pub failure_class: Option<String>,
    pub difficulty: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ValidationIssue {
    pub severity: IssueSeverity,
    pub fixture_id: Option<String>,
    pub file_path: PathBuf,
    pub json_path: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CoverageReport {
    pub fixture_count: usize,
    pub proposed_count: usize,
    pub by_failure_class: BTreeMap<String, usize>,
    pub by_capability: BTreeMap<String, usize>,
    pub by_difficulty: BTreeMap<String, usize>,
    pub false_causality_trap_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct FixtureValidationReport {
    pub issues: Vec<ValidationIssue>,
    pub coverage: CoverageReport,
}

#[derive(Debug)]
pub struct FixtureCorpusLoadError {
    pub report: FixtureValidationReport,
}

impl FixtureCorpus {
    pub fn load(root: impl AsRef<Path>) -> Result<Self, FixtureCorpusLoadError> {
        let raw = read_corpus(root.as_ref(), &FixtureSelector::default());
        let read_errors = raw
            .issues
            .iter()
            .any(|issue| issue.severity == IssueSeverity::Error);

        let Some(registry) = raw.registry else {
            return Err(FixtureCorpusLoadError {
                report: FixtureValidationReport {
                    issues: raw.issues,
                    coverage: CoverageReport::default(),
                },
            });
        };

        if read_errors {
            return Err(FixtureCorpusLoadError {
                report: FixtureValidationReport {
                    coverage: build_coverage(&registry, &raw.cases),
                    issues: raw.issues,
                },
            });
        }

        Ok(Self {
            root: raw.root,
            fixtures_root: raw.fixtures_root,
            registry,
            cases: raw.cases,
        })
    }

    pub fn select(&self, selector: &FixtureSelector) -> Vec<&FixtureCase> {
        self.cases
            .iter()
            .filter(|case| selector.matches(&case.registry_entry))
            .collect()
    }
}

impl FixtureSelector {
    pub fn matches(&self, entry: &FixtureRegistryEntry) -> bool {
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
    }
}

impl FixtureValidationReport {
    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == IssueSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == IssueSeverity::Warning)
            .count()
    }

    pub fn is_success(&self) -> bool {
        self.error_count() == 0
    }
}

pub fn validate_fixture_corpus(root: impl AsRef<Path>) -> FixtureValidationReport {
    validate_fixture_corpus_with_selector(root, &FixtureSelector::default())
}

pub fn validate_fixture_corpus_with_selector(
    root: impl AsRef<Path>,
    selector: &FixtureSelector,
) -> FixtureValidationReport {
    let mut raw = read_corpus(root.as_ref(), selector);

    if let Some(registry) = &raw.registry {
        validate_registry(registry, &raw.fixtures_root, &mut raw.issues);

        let selected_cases: Vec<&FixtureCase> = raw
            .cases
            .iter()
            .filter(|case| selector.matches(&case.registry_entry))
            .collect();

        for case in selected_cases {
            validate_case(registry, case, &mut raw.issues);
        }
    }

    let coverage = raw
        .registry
        .as_ref()
        .map(|registry| build_coverage(registry, &raw.cases))
        .unwrap_or_default();

    FixtureValidationReport {
        issues: raw.issues,
        coverage,
    }
}

fn validate_registry(
    registry: &FixtureRegistry,
    fixtures_root: &Path,
    issues: &mut Vec<ValidationIssue>,
) {
    let registry_path = fixtures_root.join("registry.json");
    let canonical_capabilities: BTreeSet<&str> =
        registry.capabilities.iter().map(String::as_str).collect();
    let canonical_failure_classes: BTreeSet<&str> = registry
        .failure_classes
        .iter()
        .map(String::as_str)
        .collect();

    if registry.schema_version != FIXTURE_SCHEMA_VERSION {
        push_error(
            issues,
            None,
            &registry_path,
            "$.schema_version",
            format!("must be {FIXTURE_SCHEMA_VERSION}"),
        );
    }

    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();

    for (index, entry) in registry.fixtures.iter().enumerate() {
        let path = registry_path_for_fixture(fixtures_root, entry);
        let fixture_id = Some(entry.id.clone());
        let json_path = format!("$.fixtures[{index}]");

        if !ids.insert(entry.id.as_str()) {
            push_error(
                issues,
                fixture_id.clone(),
                &registry_path,
                format!("{json_path}.id"),
                format!("duplicate fixture id `{}`", entry.id),
            );
        }

        if !paths.insert(entry.path.as_str()) {
            push_error(
                issues,
                fixture_id.clone(),
                &registry_path,
                format!("{json_path}.path"),
                format!("duplicate fixture path `{}`", entry.path),
            );
        }

        if !is_safe_fixture_path(&entry.path) {
            push_error(
                issues,
                fixture_id.clone(),
                &registry_path,
                format!("{json_path}.path"),
                "must stay under fixtures/scenarios",
            );
        }

        if path.file_name().and_then(|name| name.to_str()) != Some(entry.id.as_str()) {
            push_error(
                issues,
                fixture_id.clone(),
                &registry_path,
                format!("{json_path}.path"),
                "path basename must equal fixture id",
            );
        }

        if is_safe_fixture_path(&entry.path) && !path.is_dir() {
            push_error(
                issues,
                fixture_id.clone(),
                &registry_path,
                format!("{json_path}.path"),
                format!(
                    "registered fixture directory does not exist: {}",
                    path.display()
                ),
            );
        }

        if !canonical_failure_classes.contains(entry.failure_class.as_str()) {
            push_error(
                issues,
                fixture_id.clone(),
                &registry_path,
                format!("{json_path}.failure_class"),
                format!("unknown failure class `{}`", entry.failure_class),
            );
        }

        if !DIFFICULTIES.contains(&entry.difficulty.as_str()) {
            push_error(
                issues,
                fixture_id.clone(),
                &registry_path,
                format!("{json_path}.difficulty"),
                "must be baseline or hard",
            );
        }

        for (capability_index, capability) in entry.capabilities.iter().enumerate() {
            if !canonical_capabilities.contains(capability.as_str()) {
                push_error(
                    issues,
                    fixture_id.clone(),
                    &registry_path,
                    format!("{json_path}.capabilities[{capability_index}]"),
                    format!("unknown capability `{capability}`"),
                );
            }
        }
    }
}

fn validate_case(
    registry: &FixtureRegistry,
    case: &FixtureCase,
    issues: &mut Vec<ValidationIssue>,
) {
    let fixture_id = Some(case.registry_entry.id.clone());
    let scenario_path = case.directory.join("scenario.json");
    let input_path = case.directory.join("input.json");
    let expected_path = case.directory.join("expected.json");
    let canonical_capabilities: BTreeSet<&str> =
        registry.capabilities.iter().map(String::as_str).collect();

    validate_manifest(&canonical_capabilities, case, issues);
    validate_top_level_keys(case, &input_path, &expected_path, issues);

    let bundle = validate_evidence_bundle(case, &expected_path, issues);
    validate_derived_artifacts(case, &expected_path, issues);

    let reference_index = ReferenceIndex::build(&case.input, &case.expected);
    validate_source_references(case, &reference_index, &input_path, &expected_path, issues);
    validate_capability_witnesses(case, bundle.as_ref(), &expected_path, issues);
    validate_uncertainty_checks(case, bundle.as_ref(), &expected_path, issues);

    if case.manifest.id != case.registry_entry.id {
        push_error(
            issues,
            fixture_id.clone(),
            &scenario_path,
            "$.id",
            "scenario id must match registry id",
        );
    }
}

fn validate_manifest(
    canonical_capabilities: &BTreeSet<&str>,
    case: &FixtureCase,
    issues: &mut Vec<ValidationIssue>,
) {
    let fixture_id = Some(case.registry_entry.id.clone());
    let scenario_path = case.directory.join("scenario.json");
    let manifest = &case.manifest;
    let entry = &case.registry_entry;

    if manifest.schema_version != FIXTURE_SCHEMA_VERSION {
        push_error(
            issues,
            fixture_id.clone(),
            &scenario_path,
            "$.schema_version",
            format!("must be {FIXTURE_SCHEMA_VERSION}"),
        );
    }

    if manifest.version == 0 {
        push_error(
            issues,
            fixture_id.clone(),
            &scenario_path,
            "$.version",
            "must be a positive integer",
        );
    }

    if manifest.id != entry.id {
        push_error(
            issues,
            fixture_id.clone(),
            &scenario_path,
            "$.id",
            "must match registry id",
        );
    }

    if case.directory.file_name().and_then(|name| name.to_str()) != Some(entry.id.as_str()) {
        push_error(
            issues,
            fixture_id.clone(),
            &scenario_path,
            "$.id",
            "must match fixture directory basename",
        );
    }

    compare_field(
        issues,
        &fixture_id,
        &scenario_path,
        "$.failure_class",
        &manifest.failure_class,
        &entry.failure_class,
    );
    compare_field(
        issues,
        &fixture_id,
        &scenario_path,
        "$.difficulty",
        &manifest.difficulty,
        &entry.difficulty,
    );
    compare_field(
        issues,
        &fixture_id,
        &scenario_path,
        "$.title",
        &manifest.title,
        &entry.title,
    );

    if manifest.false_causality_trap != entry.false_causality_trap {
        push_error(
            issues,
            fixture_id.clone(),
            &scenario_path,
            "$.false_causality_trap",
            "must match registry entry",
        );
    }

    if sorted_strings(&manifest.capabilities) != sorted_strings(&entry.capabilities) {
        push_error(
            issues,
            fixture_id.clone(),
            &scenario_path,
            "$.capabilities",
            "must match registry entry capabilities",
        );
    }

    for (index, capability) in manifest.capabilities.iter().enumerate() {
        if !canonical_capabilities.contains(capability.as_str()) {
            push_error(
                issues,
                fixture_id.clone(),
                &scenario_path,
                format!("$.capabilities[{index}]"),
                format!("unknown capability `{capability}`"),
            );
        }
    }

    if manifest.summary.trim().is_empty() {
        push_error(
            issues,
            fixture_id.clone(),
            &scenario_path,
            "$.summary",
            "must be non-empty",
        );
    }

    if manifest.question.trim().is_empty() {
        push_error(
            issues,
            fixture_id.clone(),
            &scenario_path,
            "$.question",
            "must be non-empty",
        );
    }

    if !manifest.time_window.is_object() {
        push_error(
            issues,
            fixture_id.clone(),
            &scenario_path,
            "$.time_window",
            "must be an object",
        );
    }

    if !manifest.ground_truth.is_object() {
        push_error(
            issues,
            fixture_id,
            &scenario_path,
            "$.ground_truth",
            "must be an object",
        );
    }
}

fn validate_top_level_keys(
    case: &FixtureCase,
    input_path: &Path,
    expected_path: &Path,
    issues: &mut Vec<ValidationIssue>,
) {
    let fixture_id = Some(case.registry_entry.id.clone());
    let input_keys = non_helper_keys(&case.input);
    let expected_keys = non_helper_keys(&case.expected);

    validate_known_keys(
        issues,
        &fixture_id,
        input_path,
        &input_keys,
        INPUT_KEYS,
        "input",
    );
    validate_known_keys(
        issues,
        &fixture_id,
        expected_path,
        &expected_keys,
        EXPECTED_KEYS,
        "expected",
    );

    compare_declared_keys(
        issues,
        &fixture_id,
        &case.directory.join("scenario.json"),
        "$.inputs",
        &case.manifest.inputs,
        &input_keys,
    );
    compare_declared_keys(
        issues,
        &fixture_id,
        &case.directory.join("scenario.json"),
        "$.expected",
        &case.manifest.expected,
        &expected_keys,
    );
}

fn validate_evidence_bundle(
    case: &FixtureCase,
    expected_path: &Path,
    issues: &mut Vec<ValidationIssue>,
) -> Option<EvidenceBundle> {
    let fixture_id = Some(case.registry_entry.id.clone());

    if !case
        .manifest
        .expected
        .iter()
        .any(|key| key == "evidence_bundle")
    {
        return None;
    }

    let Some(value) = case.expected.get("evidence_bundle") else {
        push_error(
            issues,
            fixture_id,
            expected_path,
            "$.evidence_bundle",
            "declared in scenario.expected but missing from expected.json",
        );
        return None;
    };

    let bundle = match serde_json::from_value::<EvidenceBundle>(value.clone()) {
        Ok(bundle) => bundle,
        Err(error) => {
            push_error(
                issues,
                fixture_id,
                expected_path,
                "$.evidence_bundle",
                format!("invalid EvidenceBundle JSON: {error}"),
            );
            return None;
        }
    };

    if let Err(errors) = bundle.validate() {
        push_evidence_errors(
            issues,
            fixture_id,
            expected_path,
            "$.evidence_bundle",
            &errors,
        );
    }

    Some(bundle)
}

fn validate_derived_artifacts(
    case: &FixtureCase,
    expected_path: &Path,
    issues: &mut Vec<ValidationIssue>,
) {
    let fixture_id = Some(case.registry_entry.id.clone());
    let relationship_types: BTreeSet<&str> = RELATIONSHIP_TYPES.iter().copied().collect();
    let timeline_markers: BTreeSet<&str> = TIMELINE_MARKERS.iter().copied().collect();
    let metric_names_by_entity = metric_names_by_entity(&case.input);

    if let Some(relationships) = array_at(&case.expected, "relationships") {
        for (index, relationship) in relationships.iter().enumerate() {
            match relationship.get("type").and_then(Value::as_str) {
                Some(kind) if relationship_types.contains(kind) => {}
                Some(kind) => push_error(
                    issues,
                    fixture_id.clone(),
                    expected_path,
                    format!("$.relationships[{index}].type"),
                    format!("unknown relationship type `{kind}`"),
                ),
                None => push_error(
                    issues,
                    fixture_id.clone(),
                    expected_path,
                    format!("$.relationships[{index}].type"),
                    "must be a string",
                ),
            }
        }
    }

    if let Some(timeline) = array_at(&case.expected, "timeline") {
        for (index, marker) in timeline
            .iter()
            .map(|item| item.get("marker").and_then(Value::as_str))
            .enumerate()
        {
            match marker {
                Some(marker) if timeline_markers.contains(marker) => {}
                Some(marker) => push_error(
                    issues,
                    fixture_id.clone(),
                    expected_path,
                    format!("$.timeline[{index}].marker"),
                    format!("unknown timeline marker `{marker}`"),
                ),
                None => push_error(
                    issues,
                    fixture_id.clone(),
                    expected_path,
                    format!("$.timeline[{index}].marker"),
                    "must be a string",
                ),
            }
        }
    }

    if let Some(anomaly_windows) = array_at(&case.expected, "anomaly_windows") {
        for (index, anomaly) in anomaly_windows.iter().enumerate() {
            let signal = anomaly.get("signal").and_then(Value::as_str);
            let entity = anomaly.get("entity").and_then(Value::as_str);

            match signal {
                Some(signal) if !signal.trim().is_empty() => {
                    if let Some(entity) = entity
                        && let Some(metric_names) = metric_names_by_entity.get(entity)
                        && !metric_names.contains(signal)
                    {
                        push_error(
                            issues,
                            fixture_id.clone(),
                            expected_path,
                            format!("$.anomaly_windows[{index}].signal"),
                            format!("does not match an input metric name for entity `{entity}`"),
                        );
                    }
                }
                _ => push_error(
                    issues,
                    fixture_id.clone(),
                    expected_path,
                    format!("$.anomaly_windows[{index}].signal"),
                    "must be a non-empty string",
                ),
            }
        }
    }
}

fn validate_source_references(
    case: &FixtureCase,
    index: &ReferenceIndex,
    input_path: &Path,
    expected_path: &Path,
    issues: &mut Vec<ValidationIssue>,
) {
    let fixture_id = Some(case.registry_entry.id.clone());

    if let Some(value) = case.expected.get("evidence_bundle")
        && let Ok(bundle) = serde_json::from_value::<EvidenceBundle>(value.clone())
    {
        for (item_index, item) in bundle.items.iter().enumerate() {
            for (source_index, source_ref) in item.source_refs.iter().enumerate() {
                validate_evidence_source_ref(
                    issues,
                    &fixture_id,
                    expected_path,
                    &format!("$.evidence_bundle.items[{item_index}].source_refs[{source_index}]"),
                    source_ref.signal,
                    &source_ref.r#ref,
                    index,
                );
            }
        }
    }

    validate_timeline_refs(case, index, expected_path, issues);
    validate_relationship_evidence_refs(case, index, expected_path, issues);
    validate_log_pattern_refs(case, index, expected_path, issues);
    validate_suspected_cause_refs(case, index, expected_path, issues);
    validate_related_anomaly_refs(case, index, expected_path, issues);
    validate_entity_context_refs(case, index, expected_path, issues);
    validate_metric_gap_refs(case, index, input_path, issues);
    validate_telemetry_gap_refs(case, index, input_path, issues);
}

fn validate_timeline_refs(
    case: &FixtureCase,
    index: &ReferenceIndex,
    expected_path: &Path,
    issues: &mut Vec<ValidationIssue>,
) {
    if let Some(timeline) = array_at(&case.expected, "timeline") {
        for (item_index, item) in timeline.iter().enumerate() {
            validate_scalar_ref_value(
                issues,
                &Some(case.registry_entry.id.clone()),
                expected_path,
                &format!("$.timeline[{item_index}].source_ref"),
                item.get("source_ref"),
                index,
                None,
            );
        }
    }
}

fn validate_relationship_evidence_refs(
    case: &FixtureCase,
    index: &ReferenceIndex,
    expected_path: &Path,
    issues: &mut Vec<ValidationIssue>,
) {
    if let Some(relationships) = array_at(&case.expected, "relationships") {
        for (relationship_index, relationship) in relationships.iter().enumerate() {
            if let Some(evidence) = relationship.get("evidence").and_then(Value::as_array) {
                for (evidence_index, value) in evidence.iter().enumerate() {
                    validate_scalar_ref_value(
                        issues,
                        &Some(case.registry_entry.id.clone()),
                        expected_path,
                        &format!(
                            "$.relationships[{relationship_index}].evidence[{evidence_index}]"
                        ),
                        Some(value),
                        index,
                        None,
                    );
                }
            }
        }
    }
}

fn validate_log_pattern_refs(
    case: &FixtureCase,
    index: &ReferenceIndex,
    expected_path: &Path,
    issues: &mut Vec<ValidationIssue>,
) {
    if let Some(log_patterns) = array_at(&case.expected, "log_patterns") {
        for (pattern_index, pattern) in log_patterns.iter().enumerate() {
            if let Some(exemplars) = pattern.get("exemplars").and_then(Value::as_array) {
                for (exemplar_index, value) in exemplars.iter().enumerate() {
                    validate_scalar_ref_value(
                        issues,
                        &Some(case.registry_entry.id.clone()),
                        expected_path,
                        &format!("$.log_patterns[{pattern_index}].exemplars[{exemplar_index}]"),
                        Some(value),
                        index,
                        Some(&[RefCategory::Log]),
                    );
                }
            }
        }
    }
}

fn validate_suspected_cause_refs(
    case: &FixtureCase,
    index: &ReferenceIndex,
    expected_path: &Path,
    issues: &mut Vec<ValidationIssue>,
) {
    if let Some(causes) = array_at(&case.expected, "suspected_causes") {
        for (cause_index, cause) in causes.iter().enumerate() {
            for key in ["supporting", "counter"] {
                if let Some(values) = cause.get(key).and_then(Value::as_array) {
                    for (value_index, value) in values.iter().enumerate() {
                        validate_scalar_ref_value(
                            issues,
                            &Some(case.registry_entry.id.clone()),
                            expected_path,
                            &format!("$.suspected_causes[{cause_index}].{key}[{value_index}]"),
                            Some(value),
                            index,
                            Some(&[RefCategory::EvidenceItem]),
                        );
                    }
                }
            }
        }
    }
}

fn validate_related_anomaly_refs(
    case: &FixtureCase,
    index: &ReferenceIndex,
    expected_path: &Path,
    issues: &mut Vec<ValidationIssue>,
) {
    let Some(related_anomalies) = case.expected.get("related_anomalies") else {
        return;
    };

    validate_scalar_ref_value(
        issues,
        &Some(case.registry_entry.id.clone()),
        expected_path,
        "$.related_anomalies.seed",
        related_anomalies.get("seed"),
        index,
        Some(&[RefCategory::AnomalyWindow]),
    );

    if let Some(related) = related_anomalies.get("related").and_then(Value::as_array) {
        for (related_index, related_item) in related.iter().enumerate() {
            if related_item.get("window").is_some() {
                validate_scalar_ref_value(
                    issues,
                    &Some(case.registry_entry.id.clone()),
                    expected_path,
                    &format!("$.related_anomalies.related[{related_index}].window"),
                    related_item.get("window"),
                    index,
                    Some(&[RefCategory::AnomalyWindow]),
                );
            } else if related_item.get("prior_incident").is_some() {
                validate_scalar_ref_value(
                    issues,
                    &Some(case.registry_entry.id.clone()),
                    expected_path,
                    &format!("$.related_anomalies.related[{related_index}].prior_incident"),
                    related_item.get("prior_incident"),
                    index,
                    Some(&[RefCategory::PriorIncident]),
                );
            } else {
                push_error(
                    issues,
                    Some(case.registry_entry.id.clone()),
                    expected_path,
                    format!("$.related_anomalies.related[{related_index}]"),
                    "must include a window or prior_incident ref",
                );
            }
        }
    }
}

fn validate_entity_context_refs(
    case: &FixtureCase,
    index: &ReferenceIndex,
    expected_path: &Path,
    issues: &mut Vec<ValidationIssue>,
) {
    let Some(entity_context) = case.expected.get("entity_context") else {
        return;
    };

    validate_scalar_ref_value(
        issues,
        &Some(case.registry_entry.id.clone()),
        expected_path,
        "$.entity_context.entity",
        entity_context.get("entity"),
        index,
        Some(&[RefCategory::Entity]),
    );

    for key in ["recent_changes", "siblings_same_name", "related_incidents"] {
        if let Some(values) = entity_context.get(key).and_then(Value::as_array) {
            let expected = match key {
                "recent_changes" => Some(&[RefCategory::Change][..]),
                "siblings_same_name" => Some(&[RefCategory::Entity][..]),
                "related_incidents" => Some(&[RefCategory::PriorIncident][..]),
                _ => None,
            };

            for (value_index, value) in values.iter().enumerate() {
                validate_scalar_ref_value(
                    issues,
                    &Some(case.registry_entry.id.clone()),
                    expected_path,
                    &format!("$.entity_context.{key}[{value_index}]"),
                    Some(value),
                    index,
                    expected,
                );
            }
        }
    }
}

fn validate_metric_gap_refs(
    case: &FixtureCase,
    index: &ReferenceIndex,
    input_path: &Path,
    issues: &mut Vec<ValidationIssue>,
) {
    if let Some(metrics) = array_at(&case.input, "metrics") {
        for (metric_index, metric) in metrics.iter().enumerate() {
            if let Some(gap) = metric.get("_gap") {
                validate_scalar_ref_value(
                    issues,
                    &Some(case.registry_entry.id.clone()),
                    input_path,
                    &format!("$.metrics[{metric_index}]._gap.ref"),
                    gap.get("ref"),
                    index,
                    Some(&[RefCategory::TelemetryGap]),
                );
            }
        }
    }
}

fn validate_telemetry_gap_refs(
    case: &FixtureCase,
    index: &ReferenceIndex,
    input_path: &Path,
    issues: &mut Vec<ValidationIssue>,
) {
    if let Some(gaps) = array_at(&case.input, "telemetry_gaps") {
        for (gap_index, gap) in gaps.iter().enumerate() {
            if gap.get("cause").is_some() {
                validate_scalar_ref_value(
                    issues,
                    &Some(case.registry_entry.id.clone()),
                    input_path,
                    &format!("$.telemetry_gaps[{gap_index}].cause"),
                    gap.get("cause"),
                    index,
                    None,
                );
            }
        }
    }
}

fn validate_capability_witnesses(
    case: &FixtureCase,
    bundle: Option<&EvidenceBundle>,
    expected_path: &Path,
    issues: &mut Vec<ValidationIssue>,
) {
    for capability in &case.manifest.capabilities {
        let ok = match capability.as_str() {
            "entity-resolution" => non_empty_key(&case.expected, "entities"),
            "relationship-building" => non_empty_key(&case.expected, "relationships"),
            "change-ingestion" => non_empty_key(&case.input, "changes"),
            "anomaly-windows" => non_empty_key(&case.expected, "anomaly_windows"),
            "log-pattern-clustering" => non_empty_key(&case.expected, "log_patterns"),
            "evidence-ir" | "get_evidence_bundle" => bundle.is_some_and(|bundle| {
                !bundle.items.is_empty()
                    && bundle.budget.max_items > 0
                    && bundle.budget.max_tokens > 0
            }),
            "build_timeline" => non_empty_key(&case.expected, "timeline"),
            "find_related_anomalies" => non_empty_key(&case.expected, "related_anomalies"),
            "compare_windows" => non_empty_key(&case.expected, "window_comparison"),
            "rank_suspected_causes" => non_empty_key(&case.expected, "suspected_causes"),
            "expand_entity_context" => non_empty_key(&case.expected, "entity_context"),
            "suggest_next_checks" => non_empty_key(&case.expected, "next_checks"),
            "false-causality-guard" => has_counter_evidence_path(bundle, &case.expected),
            "token-budget-retrieval" => case
                .expected
                .pointer("/evidence_bundle/budget")
                .is_some_and(|budget| {
                    budget.get("max_items").is_some() && budget.get("max_tokens").is_some()
                }),
            _ => true,
        };

        if !ok {
            push_error(
                issues,
                Some(case.registry_entry.id.clone()),
                expected_path,
                "$",
                format!("declared capability `{capability}` lacks a non-empty structural witness"),
            );
        }
    }
}

fn validate_uncertainty_checks(
    case: &FixtureCase,
    bundle: Option<&EvidenceBundle>,
    expected_path: &Path,
    issues: &mut Vec<ValidationIssue>,
) {
    let fixture_id = Some(case.registry_entry.id.clone());
    let has_counter = has_counter_evidence_path(bundle, &case.expected);

    if case.manifest.false_causality_trap && !has_counter {
        push_error(
            issues,
            fixture_id.clone(),
            expected_path,
            "$",
            "false-causality trap requires counter-evidence",
        );
    }

    let not_the_cause = not_the_cause_entities(&case.manifest.ground_truth);
    if !not_the_cause.is_empty() && !has_counter {
        push_error(
            issues,
            fixture_id.clone(),
            expected_path,
            "$",
            "ground_truth.not_the_cause requires a counter-evidence path",
        );
    }

    if let Some(first_ranked_entity) = first_ranked_suspected_cause(&case.expected)
        && not_the_cause.contains(first_ranked_entity)
    {
        push_error(
            issues,
            fixture_id.clone(),
            expected_path,
            "$.suspected_causes",
            format!("explicitly innocent suspect `{first_ranked_entity}` must not rank first"),
        );
    }

    let has_missing_data_channel = bundle.is_some_and(|bundle| {
        bundle
            .items
            .iter()
            .any(|item| item.kind == EvidenceKind::MissingData || !item.missing_data.is_empty())
    });

    if (case.manifest.failure_class == "missing-data" || case.input.get("telemetry_gaps").is_some())
        && !has_missing_data_channel
    {
        push_error(
            issues,
            fixture_id,
            expected_path,
            "$.evidence_bundle",
            "missing-data scenarios require missing_data evidence or item-level missing_data",
        );
    }
}

fn validate_evidence_source_ref(
    issues: &mut Vec<ValidationIssue>,
    fixture_id: &Option<String>,
    path: &Path,
    json_path: &str,
    signal: SourceSignal,
    raw_ref: &str,
    index: &ReferenceIndex,
) {
    if signal == SourceSignal::External {
        push_error(
            issues,
            fixture_id.clone(),
            path,
            json_path,
            "external source refs are not allowed in self-contained fixtures",
        );
        return;
    }

    let Some(categories) = index.resolve(raw_ref) else {
        push_error(
            issues,
            fixture_id.clone(),
            path,
            json_path,
            format!("unresolved source ref `{raw_ref}`"),
        );
        return;
    };

    let expected = categories_for_signal(signal);
    if !categories
        .iter()
        .any(|category| expected.contains(category))
    {
        push_warning(
            issues,
            fixture_id.clone(),
            path,
            json_path,
            format!(
                "source signal `{}` points at {} ref `{raw_ref}`",
                source_signal_name(signal),
                display_categories(categories)
            ),
        );
    }
}

fn validate_scalar_ref_value(
    issues: &mut Vec<ValidationIssue>,
    fixture_id: &Option<String>,
    path: &Path,
    json_path: &str,
    value: Option<&Value>,
    index: &ReferenceIndex,
    expected_categories: Option<&[RefCategory]>,
) {
    let Some(raw_ref) = value.and_then(Value::as_str) else {
        push_error(
            issues,
            fixture_id.clone(),
            path,
            json_path,
            "must be a string ref",
        );
        return;
    };

    if raw_ref.trim().is_empty() {
        push_error(
            issues,
            fixture_id.clone(),
            path,
            json_path,
            "must be a non-empty ref",
        );
        return;
    }

    let Some(categories) = index.resolve(raw_ref) else {
        push_error(
            issues,
            fixture_id.clone(),
            path,
            json_path,
            format!("unresolved ref `{raw_ref}`"),
        );
        return;
    };

    if let Some(expected_categories) = expected_categories
        && !categories
            .iter()
            .any(|category| expected_categories.contains(category))
    {
        push_error(
            issues,
            fixture_id.clone(),
            path,
            json_path,
            format!(
                "ref `{raw_ref}` resolves as {}, expected {}",
                display_categories(categories),
                display_categories(expected_categories)
            ),
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RefCategory {
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
struct ReferenceIndex {
    refs: BTreeMap<String, BTreeSet<RefCategory>>,
}

impl ReferenceIndex {
    fn build(input: &Value, expected: &Value) -> Self {
        let mut index = Self::default();
        index.add_input_refs(input);
        index.add_expected_refs(expected);
        index
    }

    fn resolve(&self, raw_ref: &str) -> Option<&BTreeSet<RefCategory>> {
        if let Some(categories) = self.refs.get(raw_ref) {
            return Some(categories);
        }

        raw_ref
            .strip_prefix("trace:")
            .and_then(|stripped| self.refs.get(stripped))
    }

    fn add(&mut self, raw_ref: impl Into<String>, category: RefCategory) {
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
                                self.add(format!("{trace_id}/{span_id}"), RefCategory::Span);
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
                    self.add(format!("{name}@{entity}"), RefCategory::Metric);
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

struct RawCorpus {
    root: PathBuf,
    fixtures_root: PathBuf,
    registry: Option<FixtureRegistry>,
    cases: Vec<FixtureCase>,
    issues: Vec<ValidationIssue>,
}

fn read_corpus(root: &Path, selector: &FixtureSelector) -> RawCorpus {
    let root = root.to_path_buf();
    let fixtures_root = root.join("fixtures");
    let registry_path = fixtures_root.join("registry.json");
    let mut issues = Vec::new();

    let registry = match read_json::<FixtureRegistry>(&registry_path) {
        Ok(registry) => Some(registry),
        Err(error) => {
            push_read_error(&mut issues, None, &registry_path, "$", error);
            None
        }
    };

    let cases = registry
        .as_ref()
        .map(|registry| read_cases(registry, &fixtures_root, selector, &mut issues))
        .unwrap_or_default();

    RawCorpus {
        root,
        fixtures_root,
        registry,
        cases,
        issues,
    }
}

fn read_cases(
    registry: &FixtureRegistry,
    fixtures_root: &Path,
    selector: &FixtureSelector,
    issues: &mut Vec<ValidationIssue>,
) -> Vec<FixtureCase> {
    let mut cases = Vec::new();

    for entry in &registry.fixtures {
        if !selector.matches(entry) {
            continue;
        }

        let directory = registry_path_for_fixture(fixtures_root, entry);
        let scenario_path = directory.join("scenario.json");
        let input_path = directory.join("input.json");
        let expected_path = directory.join("expected.json");
        let fixture_id = Some(entry.id.clone());

        let manifest = match read_json::<ScenarioManifest>(&scenario_path) {
            Ok(manifest) => manifest,
            Err(error) => {
                push_read_error(issues, fixture_id.clone(), &scenario_path, "$", error);
                continue;
            }
        };
        let input = match read_json::<Value>(&input_path) {
            Ok(input) => input,
            Err(error) => {
                push_read_error(issues, fixture_id.clone(), &input_path, "$", error);
                continue;
            }
        };
        let expected = match read_json::<Value>(&expected_path) {
            Ok(expected) => expected,
            Err(error) => {
                push_read_error(issues, fixture_id, &expected_path, "$", error);
                continue;
            }
        };

        cases.push(FixtureCase {
            registry_entry: entry.clone(),
            directory,
            manifest,
            input,
            expected,
        });
    }

    cases
}

fn build_coverage(registry: &FixtureRegistry, cases: &[FixtureCase]) -> CoverageReport {
    let mut coverage = CoverageReport {
        fixture_count: cases.len(),
        proposed_count: registry.proposed.len(),
        ..CoverageReport::default()
    };

    for case in cases {
        *coverage
            .by_failure_class
            .entry(case.registry_entry.failure_class.clone())
            .or_default() += 1;
        *coverage
            .by_difficulty
            .entry(case.registry_entry.difficulty.clone())
            .or_default() += 1;

        if case.registry_entry.false_causality_trap {
            coverage.false_causality_trap_count += 1;
        }

        for capability in &case.registry_entry.capabilities {
            *coverage
                .by_capability
                .entry(capability.clone())
                .or_default() += 1;
        }
    }

    coverage
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, JsonReadError> {
    let body = fs::read_to_string(path).map_err(JsonReadError::Read)?;
    serde_json::from_str(&body).map_err(JsonReadError::Parse)
}

#[derive(Debug)]
enum JsonReadError {
    Read(io::Error),
    Parse(serde_json::Error),
}

fn registry_path_for_fixture(fixtures_root: &Path, entry: &FixtureRegistryEntry) -> PathBuf {
    fixtures_root.join(&entry.path)
}

fn is_safe_fixture_path(path: &str) -> bool {
    let mut components = Path::new(path).components();

    match components.next() {
        Some(Component::Normal(component)) if component == "scenarios" => {}
        _ => return false,
    }

    components.all(|component| matches!(component, Component::Normal(_)))
}

fn compare_field(
    issues: &mut Vec<ValidationIssue>,
    fixture_id: &Option<String>,
    path: &Path,
    json_path: &str,
    actual: &str,
    expected: &str,
) {
    if actual != expected {
        push_error(
            issues,
            fixture_id.clone(),
            path,
            json_path,
            format!("must match registry value `{expected}`"),
        );
    }
}

fn validate_known_keys(
    issues: &mut Vec<ValidationIssue>,
    fixture_id: &Option<String>,
    path: &Path,
    keys: &BTreeSet<String>,
    known_keys: &[&str],
    label: &str,
) {
    let known: BTreeSet<&str> = known_keys.iter().copied().collect();

    for key in keys {
        if !known.contains(key.as_str()) {
            push_error(
                issues,
                fixture_id.clone(),
                path,
                format!("$.{key}"),
                format!("unknown {label} top-level key `{key}`"),
            );
        }
    }
}

fn compare_declared_keys(
    issues: &mut Vec<ValidationIssue>,
    fixture_id: &Option<String>,
    path: &Path,
    json_path: &str,
    declared: &[String],
    actual: &BTreeSet<String>,
) {
    let declared = sorted_strings(declared);
    let actual = actual.iter().cloned().collect::<Vec<_>>();

    if declared != actual {
        push_error(
            issues,
            fixture_id.clone(),
            path,
            json_path,
            format!("declared keys {declared:?} must match actual non-helper keys {actual:?}"),
        );
    }
}

fn non_helper_keys(value: &Value) -> BTreeSet<String> {
    value
        .as_object()
        .map(|object| {
            object
                .keys()
                .filter(|key| !key.starts_with('_'))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

fn non_empty_key(value: &Value, key: &str) -> bool {
    value.get(key).is_some_and(non_empty_value)
}

fn non_empty_value(value: &Value) -> bool {
    match value {
        Value::Array(values) => !values.is_empty(),
        Value::Object(object) => object.keys().any(|key| !key.starts_with('_')),
        Value::Null => false,
        Value::String(value) => !value.trim().is_empty(),
        _ => true,
    }
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

fn metric_names_by_entity(input: &Value) -> BTreeMap<String, BTreeSet<String>> {
    let mut names = BTreeMap::<String, BTreeSet<String>>::new();

    if let Some(metrics) = array_at(input, "metrics") {
        for metric in metrics {
            if let (Some(entity), Some(name)) = (
                metric.get("entity").and_then(Value::as_str),
                metric.get("name").and_then(Value::as_str),
            ) {
                names
                    .entry(entity.to_string())
                    .or_default()
                    .insert(name.to_string());
            }
        }
    }

    names
}

fn has_counter_evidence_path(bundle: Option<&EvidenceBundle>, expected: &Value) -> bool {
    bundle.is_some_and(|bundle| {
        bundle.items.iter().any(|item| {
            item.kind == EvidenceKind::CounterEvidence
                || matches!(
                    item.direction,
                    EvidenceDirection::Weakens | EvidenceDirection::Contradicts
                )
        })
    }) || array_at(expected, "suspected_causes").is_some_and(|causes| {
        causes.iter().any(|cause| {
            cause
                .get("counter")
                .and_then(Value::as_array)
                .is_some_and(|counter| !counter.is_empty())
        })
    })
}

fn not_the_cause_entities(ground_truth: &Value) -> BTreeSet<&str> {
    ground_truth
        .get("not_the_cause")
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

fn first_ranked_suspected_cause(expected: &Value) -> Option<&str> {
    array_at(expected, "suspected_causes")?
        .iter()
        .filter_map(|cause| {
            let rank = cause.get("rank")?.as_u64()?;
            let entity = cause.get("entity")?.as_str()?;
            Some((rank, entity))
        })
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, entity)| entity)
}

fn categories_for_signal(signal: SourceSignal) -> &'static [RefCategory] {
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

fn source_signal_name(signal: SourceSignal) -> &'static str {
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

fn display_categories<'a>(categories: impl IntoIterator<Item = &'a RefCategory>) -> String {
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

fn sorted_strings(values: &[String]) -> Vec<String> {
    let mut values = values.to_vec();
    values.sort();
    values
}

fn push_read_error(
    issues: &mut Vec<ValidationIssue>,
    fixture_id: Option<String>,
    path: &Path,
    json_path: impl Into<String>,
    error: JsonReadError,
) {
    let message = match error {
        JsonReadError::Read(error) => format!("failed to read file: {error}"),
        JsonReadError::Parse(error) => format!("failed to parse JSON: {error}"),
    };

    push_error(issues, fixture_id, path, json_path, message);
}

fn push_evidence_errors(
    issues: &mut Vec<ValidationIssue>,
    fixture_id: Option<String>,
    path: &Path,
    prefix: &str,
    errors: &ValidationErrors,
) {
    for error in errors.errors() {
        push_error(
            issues,
            fixture_id.clone(),
            path,
            format!("{prefix}.{}", error.path),
            error.message.clone(),
        );
    }
}

fn push_error(
    issues: &mut Vec<ValidationIssue>,
    fixture_id: Option<String>,
    file_path: &Path,
    json_path: impl Into<String>,
    message: impl Into<String>,
) {
    issues.push(ValidationIssue {
        severity: IssueSeverity::Error,
        fixture_id,
        file_path: file_path.to_path_buf(),
        json_path: json_path.into(),
        message: message.into(),
    });
}

fn push_warning(
    issues: &mut Vec<ValidationIssue>,
    fixture_id: Option<String>,
    file_path: &Path,
    json_path: impl Into<String>,
    message: impl Into<String>,
) {
    issues.push(ValidationIssue {
        severity: IssueSeverity::Warning,
        fixture_id,
        file_path: file_path.to_path_buf(),
        json_path: json_path.into(),
        message: message.into(),
    });
}

impl fmt::Display for FixtureValidationReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            formatter,
            "fixture validation: {} error(s), {} warning(s)",
            self.error_count(),
            self.warning_count()
        )?;

        if !self.issues.is_empty() {
            writeln!(formatter)?;
            writeln!(formatter, "Issues:")?;
            for issue in &self.issues {
                writeln!(
                    formatter,
                    "- {:?}: {} {}: {}",
                    issue.severity,
                    issue.file_path.display(),
                    issue.json_path,
                    issue.message
                )?;
            }
        }

        writeln!(formatter)?;
        writeln!(formatter, "Coverage:")?;
        writeln!(formatter, "- fixtures: {}", self.coverage.fixture_count)?;
        writeln!(formatter, "- proposed: {}", self.coverage.proposed_count)?;
        writeln!(
            formatter,
            "- false-causality traps: {}",
            self.coverage.false_causality_trap_count
        )?;
        write_counts(
            formatter,
            "failure classes",
            &self.coverage.by_failure_class,
        )?;
        write_counts(formatter, "capabilities", &self.coverage.by_capability)?;
        write_counts(formatter, "difficulties", &self.coverage.by_difficulty)?;

        Ok(())
    }
}

impl fmt::Display for FixtureCorpusLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "failed to load fixture corpus: {}", self.report)
    }
}

impl std::error::Error for FixtureCorpusLoadError {}

fn write_counts(
    formatter: &mut fmt::Formatter<'_>,
    label: &str,
    counts: &BTreeMap<String, usize>,
) -> fmt::Result {
    writeln!(formatter, "- {label}:")?;
    for (key, count) in counts {
        writeln!(formatter, "  - {key}: {count}")?;
    }
    Ok(())
}
