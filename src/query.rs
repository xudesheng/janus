use crate::{
    evidence::{
        EvidenceBundle, EvidenceDirection, EvidenceKind, SourceRef, TimeWindow, ValidationErrors,
    },
    fixture_validation::{FixtureCase, FixtureCorpus, FixtureCorpusLoadError, FixtureSelector},
    hot_context_store::{HotContextStore, HotStoreError, SourceQuery, SourceResolution},
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
    FixtureCorpusLoad(FixtureCorpusLoadError),
    FixtureCaseNotFound {
        scenario_id: String,
    },
    MissingFixtureBundle {
        scenario_id: String,
    },
    FixtureBundleParse {
        scenario_id: String,
        source: serde_json::Error,
    },
    InvalidFixtureBundle(ValidationErrors),
    HotStore(HotStoreError),
    SourceLookup {
        item_id: String,
        source_ref: SourceRef,
        message: String,
    },
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

    let case = load_fixture_case_by_scenario_id(scenario_id)?;
    let bundle = load_bundle_from_case(&case)?;
    bundle
        .validate()
        .map_err(GetEvidenceBundleError::InvalidFixtureBundle)?;

    ensure_budget_fits(&query, &bundle)?;
    ensure_required_raw_refs(&query, &bundle)?;
    ensure_required_counter_evidence(&query, &bundle)?;

    let store =
        HotContextStore::load_fixture_case(&case).map_err(GetEvidenceBundleError::HotStore)?;
    ensure_source_refs_resolve(&store, &bundle)?;
    ensure_query_context_selects(&store, &query)?;

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
            GetEvidenceBundleError::FixtureCorpusLoad(error) => {
                write!(formatter, "fixture corpus load error: {error}")
            }
            GetEvidenceBundleError::FixtureCaseNotFound { scenario_id } => {
                write!(formatter, "fixture case not found: {scenario_id}")
            }
            GetEvidenceBundleError::MissingFixtureBundle { scenario_id } => {
                write!(
                    formatter,
                    "missing evidence_bundle in fixture case: {scenario_id}"
                )
            }
            GetEvidenceBundleError::FixtureBundleParse {
                scenario_id,
                source,
            } => write!(
                formatter,
                "invalid evidence_bundle JSON in fixture case {scenario_id}: {source}"
            ),
            GetEvidenceBundleError::InvalidFixtureBundle(errors) => {
                write!(formatter, "invalid fixture bundle: {errors}")
            }
            GetEvidenceBundleError::HotStore(error) => {
                write!(formatter, "hot context store error: {error}")
            }
            GetEvidenceBundleError::SourceLookup {
                item_id,
                source_ref,
                message,
            } => write!(
                formatter,
                "source lookup failed for evidence item {item_id} ref {:?}: {message}",
                source_ref
            ),
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
            GetEvidenceBundleError::FixtureCorpusLoad(error) => Some(error),
            GetEvidenceBundleError::FixtureBundleParse { source, .. } => Some(source),
            GetEvidenceBundleError::InvalidFixtureBundle(errors) => Some(errors),
            GetEvidenceBundleError::HotStore(error) => Some(error),
            GetEvidenceBundleError::FixtureCaseNotFound { .. }
            | GetEvidenceBundleError::MissingFixtureBundle { .. }
            | GetEvidenceBundleError::SourceLookup { .. }
            | GetEvidenceBundleError::UnsupportedBudget { .. }
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

fn ensure_source_refs_resolve(
    store: &HotContextStore,
    bundle: &EvidenceBundle,
) -> Result<(), GetEvidenceBundleError> {
    for item in &bundle.items {
        for source_ref in item.source_refs.iter() {
            match store.resolve_source_ref(source_ref) {
                SourceResolution::Found(_) => {}
                resolution => {
                    return Err(GetEvidenceBundleError::SourceLookup {
                        item_id: item.id.clone(),
                        source_ref: source_ref.clone(),
                        message: describe_resolution(resolution),
                    });
                }
            }
        }
    }

    Ok(())
}

fn ensure_query_context_selects(
    store: &HotContextStore,
    query: &EvidenceQuery,
) -> Result<(), GetEvidenceBundleError> {
    let time_matches = store.select(SourceQuery {
        time_window: Some(query.time_window.clone()),
        ..SourceQuery::default()
    });

    if time_matches.is_empty() {
        return Err(GetEvidenceBundleError::UnsatisfiedRequirement {
            requirement: "hot_context_time_window",
            message: "query time window matched no hot-store records".to_string(),
        });
    }

    if !query.entities.is_empty() {
        let entity_matches = store.select(SourceQuery {
            entities: query.entities.clone(),
            ..SourceQuery::default()
        });

        if entity_matches.is_empty() {
            return Err(GetEvidenceBundleError::UnsatisfiedRequirement {
                requirement: "hot_context_entities",
                message: format!(
                    "query entities matched no hot-store records: {:?}",
                    query.entities
                ),
            });
        }

        let combined_matches = store.select(SourceQuery {
            time_window: Some(query.time_window.clone()),
            entities: query.entities.clone(),
            ..SourceQuery::default()
        });

        if combined_matches.is_empty() {
            return Err(GetEvidenceBundleError::UnsatisfiedRequirement {
                requirement: "hot_context_time_window_entities",
                message: format!(
                    "query time window and entities matched no shared hot-store records: {:?}",
                    query.entities
                ),
            });
        }
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

fn load_fixture_case_by_scenario_id(
    scenario_id: &str,
) -> Result<FixtureCase, GetEvidenceBundleError> {
    let corpus =
        FixtureCorpus::load(fixture_root()).map_err(GetEvidenceBundleError::FixtureCorpusLoad)?;
    let selector = FixtureSelector {
        fixture_id: Some(scenario_id.to_string()),
        ..FixtureSelector::default()
    };

    corpus
        .select(&selector)
        .into_iter()
        .next()
        .cloned()
        .ok_or_else(|| GetEvidenceBundleError::FixtureCaseNotFound {
            scenario_id: scenario_id.to_string(),
        })
}

fn fixture_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn load_bundle_from_case(case: &FixtureCase) -> Result<EvidenceBundle, GetEvidenceBundleError> {
    let scenario_id = case.registry_entry.id.clone();
    let value = case.expected.get("evidence_bundle").ok_or_else(|| {
        GetEvidenceBundleError::MissingFixtureBundle {
            scenario_id: scenario_id.clone(),
        }
    })?;

    serde_json::from_value(value.clone()).map_err(|source| {
        GetEvidenceBundleError::FixtureBundleParse {
            scenario_id,
            source,
        }
    })
}

fn describe_resolution(resolution: SourceResolution<'_>) -> String {
    match resolution {
        SourceResolution::Found(_) => "found".to_string(),
        SourceResolution::Missing { raw_ref } => format!("missing ref `{raw_ref}`"),
        SourceResolution::Unsupported { raw_ref, signal } => {
            format!("unsupported source signal {:?} for ref `{raw_ref}`", signal)
        }
        SourceResolution::Ambiguous {
            raw_ref,
            candidates,
        } => format!(
            "ambiguous ref `{raw_ref}` resolved to {}",
            describe_candidates(&candidates)
        ),
        SourceResolution::SignalMismatch {
            raw_ref,
            signal,
            candidates,
        } => format!(
            "source signal {:?} mismatched ref `{raw_ref}` candidates {}",
            signal,
            describe_candidates(&candidates)
        ),
    }
}

fn describe_candidates(candidates: &[&crate::hot_context_store::StoredRecord]) -> String {
    candidates
        .iter()
        .map(|record| format!("{}:{}", record.kind, record.key))
        .collect::<Vec<_>>()
        .join(", ")
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
