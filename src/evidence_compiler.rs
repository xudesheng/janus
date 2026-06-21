use crate::{
    derived_context::DerivedContext,
    evidence::{
        EvidenceBundle, EvidenceDirection, EvidenceFreshness, EvidenceItem, EvidenceKind,
        SourceRefs, TimeWindow, UnitInterval,
    },
    fixture_validation::FixtureCase,
    hot_context_store::HotContextStore,
    query::EvidenceQuery,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

const NUMERIC_TOLERANCE: f64 = 0.05;

#[derive(Debug, Clone, Copy)]
pub struct EvidenceCompilerInput<'a> {
    pub query: &'a EvidenceQuery,
    pub store: &'a HotContextStore,
    pub derived: &'a DerivedContext,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceCompilation {
    pub bundle: EvidenceBundle,
    pub suspected_causes: Vec<SuspectedCause>,
    pub next_checks: Vec<NextCheck>,
    pub report: EvidenceCompilationReport,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvidenceCompilationReport {
    pub generated_items: usize,
    pub selected_items: usize,
    pub dropped_items: Vec<DroppedEvidenceItem>,
    pub requirement_failures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DroppedEvidenceItem {
    pub id: String,
    pub reason: String,
    pub token_cost: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuspectedCause {
    pub rank: u32,
    pub entity: String,
    pub hypothesis: String,
    pub score: UnitInterval,
    #[serde(default)]
    pub reasons: Vec<String>,
    #[serde(default)]
    pub supporting: Vec<String>,
    #[serde(default)]
    pub counter: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trap_note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NextCheck {
    pub action: String,
    pub rationale: String,
    pub expected_signal: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EvidenceCompilationComparison {
    pub bundle_mismatches: Vec<EvidenceCompilationFieldMismatch>,
    pub item_order_mismatches: Vec<EvidenceItemOrderMismatch>,
    pub item_mismatches: Vec<EvidenceCompilationFieldMismatch>,
    pub missing_suspected_causes: Vec<u32>,
    pub extra_suspected_causes: Vec<u32>,
    pub suspected_cause_mismatches: Vec<EvidenceCompilationFieldMismatch>,
    pub missing_next_checks: Vec<usize>,
    pub extra_next_checks: Vec<usize>,
    pub next_check_mismatches: Vec<EvidenceCompilationFieldMismatch>,
    pub text_differences: Vec<EvidenceCompilationFieldMismatch>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceCompilationFieldMismatch {
    pub artifact: String,
    pub field: String,
    pub expected: Value,
    pub actual: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceItemOrderMismatch {
    pub index: usize,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

#[derive(Debug)]
pub enum EvidenceCompilationGoldError {
    MissingArtifact {
        fixture_id: String,
        artifact: &'static str,
    },
    ParseArtifact {
        fixture_id: String,
        artifact: &'static str,
        source: serde_json::Error,
    },
}

#[derive(Debug)]
pub enum EvidenceCompileError {
    TokenEstimate {
        item_id: String,
        source: serde_json::Error,
    },
    TokenCostOverflow {
        item_id: String,
        bytes: usize,
    },
}

#[derive(Serialize)]
struct EvidenceItemTokenPayload<'a> {
    id: &'a str,
    claim: &'a str,
    kind: EvidenceKind,
    direction: EvidenceDirection,
    strength: UnitInterval,
    time_window: &'a TimeWindow,
    entities: &'a Vec<String>,
    source_refs: &'a SourceRefs,
    freshness: EvidenceFreshness,
    missing_data: &'a Vec<String>,
    privacy_scope: &'a str,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    confidence: &'a BTreeMap<String, UnitInterval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: &'a Option<String>,
}

impl EvidenceCompilation {
    pub fn from_bundle(
        bundle: EvidenceBundle,
        suspected_causes: Vec<SuspectedCause>,
        next_checks: Vec<NextCheck>,
    ) -> Self {
        let selected_items = bundle.items.len();

        Self {
            bundle,
            suspected_causes,
            next_checks,
            report: EvidenceCompilationReport {
                generated_items: selected_items,
                selected_items,
                dropped_items: Vec::new(),
                requirement_failures: Vec::new(),
            },
        }
    }
}

impl EvidenceCompilationComparison {
    pub fn has_expected_mismatches(&self) -> bool {
        !self.bundle_mismatches.is_empty()
            || !self.item_order_mismatches.is_empty()
            || !self.item_mismatches.is_empty()
            || !self.missing_suspected_causes.is_empty()
            || !self.suspected_cause_mismatches.is_empty()
            || !self.missing_next_checks.is_empty()
            || !self.next_check_mismatches.is_empty()
    }
}

pub fn load_expected_compilation(
    case: &FixtureCase,
) -> Result<EvidenceCompilation, EvidenceCompilationGoldError> {
    let bundle = parse_required_artifact(case, "evidence_bundle")?;
    let suspected_causes = parse_optional_artifact(case, "suspected_causes")?.unwrap_or_default();
    let next_checks = parse_optional_artifact(case, "next_checks")?.unwrap_or_default();

    Ok(EvidenceCompilation::from_bundle(
        bundle,
        suspected_causes,
        next_checks,
    ))
}

pub fn compare_compiled_evidence_for_case(
    case: &FixtureCase,
    actual: &EvidenceCompilation,
) -> Result<EvidenceCompilationComparison, EvidenceCompilationGoldError> {
    let expected = load_expected_compilation(case)?;

    Ok(compare_compiled_evidence(&expected, actual))
}

pub fn compare_compiled_evidence(
    expected: &EvidenceCompilation,
    actual: &EvidenceCompilation,
) -> EvidenceCompilationComparison {
    let mut comparison = EvidenceCompilationComparison::default();

    compare_bundle(&expected.bundle, &actual.bundle, &mut comparison);
    compare_suspected_causes(expected, actual, &mut comparison);
    compare_next_checks(expected, actual, &mut comparison);

    comparison
}

pub fn canonical_evidence_item_payload_json_bytes(
    item: &EvidenceItem,
) -> Result<Vec<u8>, EvidenceCompileError> {
    let payload = EvidenceItemTokenPayload {
        id: item.id.as_str(),
        claim: item.claim.as_str(),
        kind: item.kind,
        direction: item.direction,
        strength: item.strength,
        time_window: &item.time_window,
        entities: &item.entities,
        source_refs: &item.source_refs,
        freshness: item.freshness,
        missing_data: &item.missing_data,
        privacy_scope: item.privacy_scope.as_str(),
        confidence: &item.confidence,
        note: &item.note,
    };

    serde_json::to_vec(&payload).map_err(|source| EvidenceCompileError::TokenEstimate {
        item_id: item.id.clone(),
        source,
    })
}

pub fn estimate_evidence_item_tokens(item: &EvidenceItem) -> Result<u32, EvidenceCompileError> {
    let bytes = canonical_evidence_item_payload_json_bytes(item)?.len();
    let tokens = bytes.div_ceil(4);

    u32::try_from(tokens).map_err(|_| EvidenceCompileError::TokenCostOverflow {
        item_id: item.id.clone(),
        bytes,
    })
}

pub fn apply_compiler_token_estimates(
    bundle: &mut EvidenceBundle,
) -> Result<(), EvidenceCompileError> {
    let mut tokens_used = 0u32;

    for item in &mut bundle.items {
        let token_cost = estimate_evidence_item_tokens(item)?;
        item.token_cost = token_cost;
        tokens_used = tokens_used.saturating_add(token_cost);
    }

    bundle.budget.tokens_used = tokens_used;

    Ok(())
}

fn parse_required_artifact<T>(
    case: &FixtureCase,
    artifact: &'static str,
) -> Result<T, EvidenceCompilationGoldError>
where
    T: for<'de> Deserialize<'de>,
{
    let Some(value) = case.expected.get(artifact) else {
        return Err(EvidenceCompilationGoldError::MissingArtifact {
            fixture_id: case.registry_entry.id.clone(),
            artifact,
        });
    };

    serde_json::from_value(value.clone()).map_err(|source| {
        EvidenceCompilationGoldError::ParseArtifact {
            fixture_id: case.registry_entry.id.clone(),
            artifact,
            source,
        }
    })
}

fn parse_optional_artifact<T>(
    case: &FixtureCase,
    artifact: &'static str,
) -> Result<Option<T>, EvidenceCompilationGoldError>
where
    T: for<'de> Deserialize<'de>,
{
    let Some(value) = case.expected.get(artifact) else {
        return Ok(None);
    };

    serde_json::from_value(value.clone())
        .map(Some)
        .map_err(|source| EvidenceCompilationGoldError::ParseArtifact {
            fixture_id: case.registry_entry.id.clone(),
            artifact,
            source,
        })
}

fn compare_bundle(
    expected: &EvidenceBundle,
    actual: &EvidenceBundle,
    comparison: &mut EvidenceCompilationComparison,
) {
    compare_exact(
        &mut comparison.bundle_mismatches,
        "bundle",
        "question",
        &expected.question,
        &actual.question,
    );
    compare_exact(
        &mut comparison.bundle_mismatches,
        "bundle",
        "hypothesis",
        &expected.hypothesis,
        &actual.hypothesis,
    );
    compare_exact(
        &mut comparison.bundle_mismatches,
        "bundle",
        "time_window",
        &expected.time_window,
        &actual.time_window,
    );
    compare_exact(
        &mut comparison.bundle_mismatches,
        "bundle",
        "budget.max_items",
        &expected.budget.max_items,
        &actual.budget.max_items,
    );
    compare_exact(
        &mut comparison.bundle_mismatches,
        "bundle",
        "budget.max_tokens",
        &expected.budget.max_tokens,
        &actual.budget.max_tokens,
    );
    compare_exact(
        &mut comparison.bundle_mismatches,
        "bundle",
        "budget.tokens_used",
        &expected.budget.tokens_used,
        &actual.budget.tokens_used,
    );
    compare_exact(
        &mut comparison.bundle_mismatches,
        "bundle",
        "budget.items_dropped",
        &expected.budget.items_dropped,
        &actual.budget.items_dropped,
    );

    compare_items(expected, actual, comparison);
}

fn compare_items(
    expected: &EvidenceBundle,
    actual: &EvidenceBundle,
    comparison: &mut EvidenceCompilationComparison,
) {
    let max_len = expected.items.len().max(actual.items.len());

    for index in 0..max_len {
        match (expected.items.get(index), actual.items.get(index)) {
            (Some(expected), Some(actual)) => {
                if expected.id != actual.id {
                    comparison
                        .item_order_mismatches
                        .push(EvidenceItemOrderMismatch {
                            index,
                            expected: Some(expected.id.clone()),
                            actual: Some(actual.id.clone()),
                        });
                }

                compare_item(expected, actual, comparison);
            }
            (Some(expected), None) => {
                comparison
                    .item_order_mismatches
                    .push(EvidenceItemOrderMismatch {
                        index,
                        expected: Some(expected.id.clone()),
                        actual: None,
                    });
            }
            (None, Some(actual)) => {
                comparison
                    .item_order_mismatches
                    .push(EvidenceItemOrderMismatch {
                        index,
                        expected: None,
                        actual: Some(actual.id.clone()),
                    });
            }
            (None, None) => {}
        }
    }
}

fn compare_item(
    expected: &EvidenceItem,
    actual: &EvidenceItem,
    comparison: &mut EvidenceCompilationComparison,
) {
    let artifact = format!("item:{}", expected.id);

    compare_exact(
        &mut comparison.item_mismatches,
        &artifact,
        "id",
        &expected.id,
        &actual.id,
    );
    compare_exact(
        &mut comparison.item_mismatches,
        &artifact,
        "kind",
        &expected.kind,
        &actual.kind,
    );
    compare_exact(
        &mut comparison.item_mismatches,
        &artifact,
        "direction",
        &expected.direction,
        &actual.direction,
    );
    compare_unit_interval(
        &mut comparison.item_mismatches,
        &artifact,
        "strength",
        expected.strength,
        actual.strength,
    );
    compare_exact(
        &mut comparison.item_mismatches,
        &artifact,
        "time_window",
        &expected.time_window,
        &actual.time_window,
    );
    compare_string_sets(
        &mut comparison.item_mismatches,
        &artifact,
        "entities",
        &expected.entities,
        &actual.entities,
    );
    compare_exact(
        &mut comparison.item_mismatches,
        &artifact,
        "source_refs",
        &expected.source_refs.0,
        &actual.source_refs.0,
    );
    compare_exact(
        &mut comparison.item_mismatches,
        &artifact,
        "freshness",
        &expected.freshness,
        &actual.freshness,
    );
    compare_string_sets(
        &mut comparison.item_mismatches,
        &artifact,
        "missing_data",
        &expected.missing_data,
        &actual.missing_data,
    );
    compare_exact(
        &mut comparison.item_mismatches,
        &artifact,
        "token_cost",
        &expected.token_cost,
        &actual.token_cost,
    );
    compare_exact(
        &mut comparison.item_mismatches,
        &artifact,
        "privacy_scope",
        &expected.privacy_scope,
        &actual.privacy_scope,
    );
    compare_confidence_maps(
        &mut comparison.item_mismatches,
        &artifact,
        &expected.confidence,
        &actual.confidence,
    );
    compare_text_structural(
        &mut comparison.item_mismatches,
        &mut comparison.text_differences,
        &artifact,
        "claim",
        expected.claim.as_str(),
        actual.claim.as_str(),
    );
    compare_optional_text_structural(
        &mut comparison.item_mismatches,
        &mut comparison.text_differences,
        &artifact,
        "note",
        expected.note.as_deref(),
        actual.note.as_deref(),
    );
}

fn compare_suspected_causes(
    expected: &EvidenceCompilation,
    actual: &EvidenceCompilation,
    comparison: &mut EvidenceCompilationComparison,
) {
    let actual_by_rank = actual
        .suspected_causes
        .iter()
        .map(|cause| (cause.rank, cause))
        .collect::<BTreeMap<_, _>>();
    let expected_ranks = expected
        .suspected_causes
        .iter()
        .map(|cause| cause.rank)
        .collect::<BTreeSet<_>>();

    for expected in &expected.suspected_causes {
        let Some(actual) = actual_by_rank.get(&expected.rank).copied() else {
            comparison.missing_suspected_causes.push(expected.rank);
            continue;
        };

        compare_suspected_cause(expected, actual, comparison);
    }

    for actual in &actual.suspected_causes {
        if !expected_ranks.contains(&actual.rank) {
            comparison.extra_suspected_causes.push(actual.rank);
        }
    }
}

fn compare_suspected_cause(
    expected: &SuspectedCause,
    actual: &SuspectedCause,
    comparison: &mut EvidenceCompilationComparison,
) {
    let artifact = format!("suspected_cause:{}", expected.rank);

    compare_exact(
        &mut comparison.suspected_cause_mismatches,
        &artifact,
        "rank",
        &expected.rank,
        &actual.rank,
    );
    compare_exact(
        &mut comparison.suspected_cause_mismatches,
        &artifact,
        "entity",
        &expected.entity,
        &actual.entity,
    );
    compare_unit_interval(
        &mut comparison.suspected_cause_mismatches,
        &artifact,
        "score",
        expected.score,
        actual.score,
    );
    compare_string_sets(
        &mut comparison.suspected_cause_mismatches,
        &artifact,
        "reasons",
        &expected.reasons,
        &actual.reasons,
    );
    compare_string_sets(
        &mut comparison.suspected_cause_mismatches,
        &artifact,
        "supporting",
        &expected.supporting,
        &actual.supporting,
    );
    compare_string_sets(
        &mut comparison.suspected_cause_mismatches,
        &artifact,
        "counter",
        &expected.counter,
        &actual.counter,
    );
    compare_text_structural(
        &mut comparison.suspected_cause_mismatches,
        &mut comparison.text_differences,
        &artifact,
        "hypothesis",
        expected.hypothesis.as_str(),
        actual.hypothesis.as_str(),
    );
    compare_optional_text_structural(
        &mut comparison.suspected_cause_mismatches,
        &mut comparison.text_differences,
        &artifact,
        "note",
        expected.note.as_deref(),
        actual.note.as_deref(),
    );
    compare_optional_text_structural(
        &mut comparison.suspected_cause_mismatches,
        &mut comparison.text_differences,
        &artifact,
        "trap_note",
        expected.trap_note.as_deref(),
        actual.trap_note.as_deref(),
    );
}

fn compare_next_checks(
    expected: &EvidenceCompilation,
    actual: &EvidenceCompilation,
    comparison: &mut EvidenceCompilationComparison,
) {
    let max_len = expected.next_checks.len().max(actual.next_checks.len());

    for index in 0..max_len {
        match (
            expected.next_checks.get(index),
            actual.next_checks.get(index),
        ) {
            (Some(expected), Some(actual)) => {
                let artifact = format!("next_check:{}", index + 1);
                compare_text_structural(
                    &mut comparison.next_check_mismatches,
                    &mut comparison.text_differences,
                    &artifact,
                    "action",
                    expected.action.as_str(),
                    actual.action.as_str(),
                );
                compare_text_structural(
                    &mut comparison.next_check_mismatches,
                    &mut comparison.text_differences,
                    &artifact,
                    "rationale",
                    expected.rationale.as_str(),
                    actual.rationale.as_str(),
                );
                compare_text_structural(
                    &mut comparison.next_check_mismatches,
                    &mut comparison.text_differences,
                    &artifact,
                    "expected_signal",
                    expected.expected_signal.as_str(),
                    actual.expected_signal.as_str(),
                );
            }
            (Some(_), None) => comparison.missing_next_checks.push(index),
            (None, Some(_)) => comparison.extra_next_checks.push(index),
            (None, None) => {}
        }
    }
}

fn compare_exact<T>(
    mismatches: &mut Vec<EvidenceCompilationFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: &T,
    actual: &T,
) where
    T: Serialize + PartialEq,
{
    if expected != actual {
        mismatches.push(EvidenceCompilationFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: serde_json::to_value(expected).unwrap_or(Value::Null),
            actual: Some(serde_json::to_value(actual).unwrap_or(Value::Null)),
        });
    }
}

fn compare_unit_interval(
    mismatches: &mut Vec<EvidenceCompilationFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: UnitInterval,
    actual: UnitInterval,
) {
    if !within_numeric_tolerance(expected.0, actual.0) {
        mismatches.push(EvidenceCompilationFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: Value::from(expected.0),
            actual: Some(Value::from(actual.0)),
        });
    }
}

fn compare_confidence_maps(
    mismatches: &mut Vec<EvidenceCompilationFieldMismatch>,
    artifact: &str,
    expected: &BTreeMap<String, UnitInterval>,
    actual: &BTreeMap<String, UnitInterval>,
) {
    let keys = expected
        .keys()
        .chain(actual.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    for key in keys {
        match (expected.get(&key), actual.get(&key)) {
            (Some(expected), Some(actual)) => {
                compare_unit_interval(
                    mismatches,
                    artifact,
                    &format!("confidence.{key}"),
                    *expected,
                    *actual,
                );
            }
            (Some(expected), None) => mismatches.push(EvidenceCompilationFieldMismatch {
                artifact: artifact.to_string(),
                field: format!("confidence.{key}"),
                expected: Value::from(expected.0),
                actual: None,
            }),
            (None, Some(actual)) => mismatches.push(EvidenceCompilationFieldMismatch {
                artifact: artifact.to_string(),
                field: format!("confidence.{key}"),
                expected: Value::Null,
                actual: Some(Value::from(actual.0)),
            }),
            (None, None) => {}
        }
    }
}

fn compare_string_sets(
    mismatches: &mut Vec<EvidenceCompilationFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: &[String],
    actual: &[String],
) {
    let expected = string_set(expected);
    let actual = string_set(actual);

    if expected != actual {
        mismatches.push(EvidenceCompilationFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: serde_json::to_value(expected).unwrap_or(Value::Null),
            actual: Some(serde_json::to_value(actual).unwrap_or(Value::Null)),
        });
    }
}

fn compare_text_structural(
    mismatches: &mut Vec<EvidenceCompilationFieldMismatch>,
    differences: &mut Vec<EvidenceCompilationFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: &str,
    actual: &str,
) {
    if expected.trim().is_empty() {
        return;
    }

    if actual.trim().is_empty() {
        mismatches.push(EvidenceCompilationFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: Value::String("non-empty text".to_string()),
            actual: Some(Value::String(actual.to_string())),
        });
    } else if expected != actual {
        differences.push(EvidenceCompilationFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: Value::String(expected.to_string()),
            actual: Some(Value::String(actual.to_string())),
        });
    }
}

fn compare_optional_text_structural(
    mismatches: &mut Vec<EvidenceCompilationFieldMismatch>,
    differences: &mut Vec<EvidenceCompilationFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: Option<&str>,
    actual: Option<&str>,
) {
    match (expected, actual) {
        (Some(expected), Some(actual)) => {
            compare_text_structural(mismatches, differences, artifact, field, expected, actual);
        }
        (Some(expected), None) if !expected.trim().is_empty() => {
            mismatches.push(EvidenceCompilationFieldMismatch {
                artifact: artifact.to_string(),
                field: field.to_string(),
                expected: Value::String("non-empty text".to_string()),
                actual: None,
            });
        }
        _ => {}
    }
}

fn string_set(values: &[String]) -> BTreeSet<String> {
    values.iter().cloned().collect()
}

fn within_numeric_tolerance(expected: f64, actual: f64) -> bool {
    (expected - actual).abs() <= NUMERIC_TOLERANCE + 1e-9
}

impl fmt::Display for EvidenceCompilationGoldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvidenceCompilationGoldError::MissingArtifact {
                fixture_id,
                artifact,
            } => write!(
                formatter,
                "fixture `{fixture_id}` is missing expected artifact `{artifact}`"
            ),
            EvidenceCompilationGoldError::ParseArtifact {
                fixture_id,
                artifact,
                source,
            } => write!(
                formatter,
                "fixture `{fixture_id}` has invalid expected artifact `{artifact}`: {source}"
            ),
        }
    }
}

impl std::error::Error for EvidenceCompilationGoldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EvidenceCompilationGoldError::ParseArtifact { source, .. } => Some(source),
            EvidenceCompilationGoldError::MissingArtifact { .. } => None,
        }
    }
}

impl fmt::Display for EvidenceCompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvidenceCompileError::TokenEstimate { item_id, source } => {
                write!(
                    formatter,
                    "failed to estimate token cost for {item_id}: {source}"
                )
            }
            EvidenceCompileError::TokenCostOverflow { item_id, bytes } => write!(
                formatter,
                "estimated token cost for {item_id} overflows u32 from {bytes} bytes"
            ),
        }
    }
}

impl std::error::Error for EvidenceCompileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EvidenceCompileError::TokenEstimate { source, .. } => Some(source),
            EvidenceCompileError::TokenCostOverflow { .. } => None,
        }
    }
}
