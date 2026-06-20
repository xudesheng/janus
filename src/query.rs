use crate::{
    evidence::{EvidenceBundle, EvidenceDirection, EvidenceKind, TimeWindow, ValidationErrors},
    fixtures::{FixtureLoadError, load_bundle_by_scenario_id},
};
use schemars::{
    JsonSchema,
    schema::{InstanceType, NumberValidation, RootSchema, Schema, SchemaObject, SingleOrVec},
    schema_for,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt, fs,
    path::{Component, Path},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceQuery {
    pub intent: EvidenceQueryIntent,
    pub time_window: TimeWindow,
    pub budget: EvidenceQueryBudget,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<String>,
    #[serde(default)]
    pub require_counter_evidence: bool,
    #[serde(default = "default_require_raw_refs")]
    pub require_raw_refs: bool,
    #[serde(default)]
    pub freshness: FreshnessPreference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_scope: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceQueryIntent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hypothesis: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceQueryBudget {
    #[schemars(schema_with = "positive_u32_schema")]
    pub max_items: u32,
    #[schemars(schema_with = "positive_u32_schema")]
    pub max_tokens: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "optional_positive_u32_schema")]
    pub min_counter_evidence_items: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "optional_positive_u32_schema")]
    pub reserve_tokens_for_raw_refs: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum FreshnessPreference {
    #[default]
    Any,
    Settled,
    Changing,
}

#[derive(Debug)]
pub enum GetEvidenceBundleError {
    InvalidQuery(QueryValidationErrors),
    FixtureLoad(FixtureLoadError),
    InvalidFixtureBundle(ValidationErrors),
    UnsupportedBudget {
        requested_max_items: u32,
        required_items: usize,
        requested_max_tokens: u32,
        required_tokens: u32,
    },
    UnsatisfiedRequirement {
        requirement: &'static str,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryValidationError {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryValidationErrors {
    errors: Vec<QueryValidationError>,
}

pub fn get_evidence_bundle(query: EvidenceQuery) -> Result<EvidenceBundle, GetEvidenceBundleError> {
    query
        .validate()
        .map_err(GetEvidenceBundleError::InvalidQuery)?;

    let scenario_id = query
        .scenario_id
        .as_deref()
        .expect("validated fixture-backed queries always have scenario_id");

    let bundle = load_bundle_by_scenario_id(scenario_id).map_err(map_fixture_load_error)?;
    bundle
        .validate()
        .map_err(GetEvidenceBundleError::InvalidFixtureBundle)?;

    ensure_budget_fits(&query, &bundle)?;
    ensure_required_raw_refs(&query, &bundle)?;
    ensure_required_counter_evidence(&query, &bundle)?;

    Ok(bundle)
}

pub fn evidence_query_schema() -> RootSchema {
    schema_for!(EvidenceQuery)
}

pub fn write_schema_files(schema_dir: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let schema_dir = schema_dir.as_ref();
    fs::create_dir_all(schema_dir)?;

    write_schema_file(
        &schema_dir.join("evidence-query.schema.json"),
        &evidence_query_schema(),
    )?;

    Ok(())
}

impl EvidenceQuery {
    pub fn validate(&self) -> Result<(), QueryValidationErrors> {
        let mut errors = Vec::new();

        validate_intent(&self.intent, &mut errors);
        validate_time_window(&self.time_window, "EvidenceQuery.time_window", &mut errors);
        validate_budget(&self.budget, &mut errors);
        validate_scenario_id(self.scenario_id.as_deref(), &mut errors);

        for (index, entity) in self.entities.iter().enumerate() {
            if entity.trim().is_empty() {
                push_error(
                    &mut errors,
                    &format!("EvidenceQuery.entities[{index}]"),
                    "must be non-empty",
                );
            }
        }

        if self
            .privacy_scope
            .as_ref()
            .is_some_and(|scope| scope.trim().is_empty())
        {
            push_error(
                &mut errors,
                "EvidenceQuery.privacy_scope",
                "must be non-empty when present",
            );
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(QueryValidationErrors::new(errors))
        }
    }
}

impl QueryValidationErrors {
    fn new(errors: Vec<QueryValidationError>) -> Self {
        Self { errors }
    }

    pub fn errors(&self) -> &[QueryValidationError] {
        &self.errors
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
}

impl fmt::Display for QueryValidationErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, error) in self.errors.iter().enumerate() {
            if index > 0 {
                write!(formatter, "; ")?;
            }
            write!(formatter, "{}: {}", error.path, error.message)?;
        }
        Ok(())
    }
}

impl std::error::Error for QueryValidationErrors {}

impl fmt::Display for GetEvidenceBundleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GetEvidenceBundleError::InvalidQuery(errors) => {
                write!(formatter, "invalid query: {errors}")
            }
            GetEvidenceBundleError::FixtureLoad(error) => {
                write!(formatter, "fixture load error: {error}")
            }
            GetEvidenceBundleError::InvalidFixtureBundle(errors) => {
                write!(formatter, "invalid fixture bundle: {errors}")
            }
            GetEvidenceBundleError::UnsupportedBudget {
                requested_max_items,
                required_items,
                requested_max_tokens,
                required_tokens,
            } => write!(
                formatter,
                "unsupported budget for fixture stub: requested max_items={requested_max_items}, \
                 required items={required_items}, requested max_tokens={requested_max_tokens}, \
                 required tokens={required_tokens}"
            ),
            GetEvidenceBundleError::UnsatisfiedRequirement {
                requirement,
                message,
            } => write!(
                formatter,
                "unsatisfied fixture requirement {requirement}: {message}"
            ),
        }
    }
}

impl std::error::Error for GetEvidenceBundleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GetEvidenceBundleError::InvalidQuery(errors) => Some(errors),
            GetEvidenceBundleError::FixtureLoad(error) => Some(error),
            GetEvidenceBundleError::InvalidFixtureBundle(errors) => Some(errors),
            GetEvidenceBundleError::UnsupportedBudget { .. }
            | GetEvidenceBundleError::UnsatisfiedRequirement { .. } => None,
        }
    }
}

fn default_require_raw_refs() -> bool {
    true
}

fn ensure_budget_fits(
    query: &EvidenceQuery,
    bundle: &EvidenceBundle,
) -> Result<(), GetEvidenceBundleError> {
    let required_items = bundle.items.len();
    let required_tokens = bundle.budget.tokens_used;

    if query.budget.max_items < required_items as u32 || query.budget.max_tokens < required_tokens {
        return Err(GetEvidenceBundleError::UnsupportedBudget {
            requested_max_items: query.budget.max_items,
            required_items,
            requested_max_tokens: query.budget.max_tokens,
            required_tokens,
        });
    }

    Ok(())
}

fn ensure_required_raw_refs(
    query: &EvidenceQuery,
    bundle: &EvidenceBundle,
) -> Result<(), GetEvidenceBundleError> {
    if !query.require_raw_refs {
        return Ok(());
    }

    if bundle.items.iter().any(|item| item.source_refs.is_empty()) {
        return Err(GetEvidenceBundleError::UnsatisfiedRequirement {
            requirement: "raw_refs",
            message: "require_raw_refs was true but at least one item has no source refs"
                .to_string(),
        });
    }

    Ok(())
}

fn ensure_required_counter_evidence(
    query: &EvidenceQuery,
    bundle: &EvidenceBundle,
) -> Result<(), GetEvidenceBundleError> {
    let required_count = if query.require_counter_evidence {
        query.budget.min_counter_evidence_items.unwrap_or(1).max(1)
    } else {
        query.budget.min_counter_evidence_items.unwrap_or(0)
    };

    if required_count == 0 {
        return Ok(());
    }

    let actual_count = bundle
        .items
        .iter()
        .filter(|item| is_counter_evidence_item(item.kind, item.direction))
        .count();

    if actual_count < required_count as usize {
        return Err(GetEvidenceBundleError::UnsatisfiedRequirement {
            requirement: "counter_evidence",
            message: format!(
                "required at least {required_count} counter-evidence items, found {actual_count}"
            ),
        });
    }

    Ok(())
}

fn is_counter_evidence_item(kind: EvidenceKind, direction: EvidenceDirection) -> bool {
    kind == EvidenceKind::CounterEvidence
        || matches!(
            direction,
            EvidenceDirection::Weakens | EvidenceDirection::Contradicts
        )
}

fn validate_intent(intent: &EvidenceQueryIntent, errors: &mut Vec<QueryValidationError>) {
    let question_missing = intent
        .question
        .as_ref()
        .is_none_or(|question| question.trim().is_empty());
    let hypothesis_missing = intent
        .hypothesis
        .as_ref()
        .is_none_or(|hypothesis| hypothesis.trim().is_empty());

    if question_missing && hypothesis_missing {
        push_error(
            errors,
            "EvidenceQuery.intent",
            "at least one of question or hypothesis must be present",
        );
    }
}

fn validate_budget(budget: &EvidenceQueryBudget, errors: &mut Vec<QueryValidationError>) {
    if budget.max_items == 0 {
        push_error(
            errors,
            "EvidenceQuery.budget.max_items",
            "must be greater than 0",
        );
    }

    if budget.max_tokens == 0 {
        push_error(
            errors,
            "EvidenceQuery.budget.max_tokens",
            "must be greater than 0",
        );
    }

    if budget.min_counter_evidence_items == Some(0) {
        push_error(
            errors,
            "EvidenceQuery.budget.min_counter_evidence_items",
            "must be greater than 0 when present",
        );
    }

    if budget.reserve_tokens_for_raw_refs == Some(0) {
        push_error(
            errors,
            "EvidenceQuery.budget.reserve_tokens_for_raw_refs",
            "must be greater than 0 when present",
        );
    }
}

fn validate_scenario_id(scenario_id: Option<&str>, errors: &mut Vec<QueryValidationError>) {
    let Some(scenario_id) = scenario_id else {
        push_error(
            errors,
            "EvidenceQuery.scenario_id",
            "is required by the fixture-backed stub",
        );
        return;
    };

    if !is_safe_scenario_id(scenario_id) {
        push_error(
            errors,
            "EvidenceQuery.scenario_id",
            "must be a safe fixture scenario id",
        );
    }
}

fn is_safe_scenario_id(scenario_id: &str) -> bool {
    !scenario_id.is_empty()
        && !scenario_id.contains('/')
        && !scenario_id.contains('\\')
        && scenario_id != "."
        && scenario_id != ".."
        && Path::new(scenario_id)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn validate_time_window(
    time_window: &TimeWindow,
    path: &str,
    errors: &mut Vec<QueryValidationError>,
) {
    if time_window.start.trim().is_empty() {
        push_error(errors, &format!("{path}.start"), "must be non-empty");
    }

    if time_window.end.trim().is_empty() {
        push_error(errors, &format!("{path}.end"), "must be non-empty");
    }
}

fn push_error(errors: &mut Vec<QueryValidationError>, path: &str, message: &str) {
    errors.push(QueryValidationError {
        path: path.to_string(),
        message: message.to_string(),
    });
}

fn map_fixture_load_error(error: FixtureLoadError) -> GetEvidenceBundleError {
    match error {
        FixtureLoadError::InvalidScenarioId(scenario_id) => {
            GetEvidenceBundleError::InvalidQuery(QueryValidationErrors::new(vec![
                QueryValidationError {
                    path: "EvidenceQuery.scenario_id".to_string(),
                    message: format!("must be a safe fixture scenario id: {scenario_id}"),
                },
            ]))
        }
        other => GetEvidenceBundleError::FixtureLoad(other),
    }
}

fn write_schema_file(path: &Path, schema: &RootSchema) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(schema)?;
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

fn positive_u32_schema(_generator: &mut schemars::r#gen::SchemaGenerator) -> Schema {
    Schema::Object(SchemaObject {
        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Integer))),
        format: Some("uint32".to_string()),
        number: Some(Box::new(NumberValidation {
            minimum: Some(1.0),
            ..Default::default()
        })),
        ..Default::default()
    })
}

fn optional_positive_u32_schema(_generator: &mut schemars::r#gen::SchemaGenerator) -> Schema {
    Schema::Object(SchemaObject {
        instance_type: Some(SingleOrVec::Vec(vec![
            InstanceType::Integer,
            InstanceType::Null,
        ])),
        format: Some("uint32".to_string()),
        number: Some(Box::new(NumberValidation {
            minimum: Some(1.0),
            ..Default::default()
        })),
        ..Default::default()
    })
}
