use crate::{
    evidence::{EvidenceBundle, TimeWindow},
    query::{
        EvidenceQuery, EvidenceQueryBudget, EvidenceQueryIntent, FreshnessPreference,
        GetEvidenceBundleError, REQUIREMENT_COUNTER_EVIDENCE, REQUIREMENT_HOT_CONTEXT_ENTITIES,
        REQUIREMENT_HOT_CONTEXT_TIME_WINDOW, REQUIREMENT_HOT_CONTEXT_TIME_WINDOW_ENTITIES,
        REQUIREMENT_RAW_REFS, default_require_raw_refs, get_evidence_bundle,
    },
};
use schemars::{JsonSchema, schema::RootSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt, fs, path::Path};

pub const GET_EVIDENCE_BUNDLE_TOOL_NAME: &str = "get_evidence_bundle";
pub const MCP_PROTOCOL_VERSION: &str = "2025-11-25";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetEvidenceBundleToolInput {
    pub scenario_id: String,
    pub intent: EvidenceQueryIntent,
    pub time_window: TimeWindow,
    pub budget: EvidenceQueryBudget,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetEvidenceBundleToolOutput {
    pub bundle: EvidenceBundle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolError {
    pub code: ToolErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolErrorCode {
    InvalidRequest,
    FixtureNotFound,
    ContextUnavailable,
    BudgetUnsatisfied,
    RequirementUnsatisfied,
    SourceRefUnresolved,
    InternalError,
}

pub fn get_evidence_bundle_input_schema() -> RootSchema {
    schema_for!(GetEvidenceBundleToolInput)
}

pub fn get_evidence_bundle_output_schema() -> RootSchema {
    schema_for!(GetEvidenceBundleToolOutput)
}

pub fn get_evidence_bundle_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: GET_EVIDENCE_BUNDLE_TOOL_NAME.to_string(),
        description: "Return a bounded, source-backed Evidence IR bundle for a question or hypothesis under a token and item budget.".to_string(),
        input_schema: serde_json::to_value(get_evidence_bundle_input_schema())
            .expect("schema serialization should not fail"),
        output_schema: serde_json::to_value(get_evidence_bundle_output_schema())
            .expect("schema serialization should not fail"),
    }
}

pub fn call_get_evidence_bundle(
    arguments: Value,
) -> Result<GetEvidenceBundleToolOutput, ToolError> {
    let input: GetEvidenceBundleToolInput =
        serde_json::from_value(arguments).map_err(invalid_arguments_error)?;
    let bundle = get_evidence_bundle(input.into()).map_err(tool_error_from_get_evidence_bundle)?;

    Ok(GetEvidenceBundleToolOutput { bundle })
}

pub fn tool_error_from_get_evidence_bundle(error: GetEvidenceBundleError) -> ToolError {
    match error {
        GetEvidenceBundleError::InvalidQuery(errors) => {
            let first = errors.errors().first();
            ToolError {
                code: ToolErrorCode::InvalidRequest,
                message: format!("invalid request: {errors}"),
                path: first.map(|error| error.path.clone()),
                requirement: None,
            }
        }
        GetEvidenceBundleError::FixtureCaseNotFound { scenario_id } => ToolError {
            code: ToolErrorCode::FixtureNotFound,
            message: format!("fixture scenario `{scenario_id}` was not found"),
            path: Some("scenario_id".to_string()),
            requirement: None,
        },
        GetEvidenceBundleError::UnsupportedBudget {
            requested_max_items,
            required_items,
            requested_max_tokens,
            required_tokens,
        } => ToolError {
            code: ToolErrorCode::BudgetUnsatisfied,
            message: format!(
                "requested budget cannot fit compiled evidence: max_items={requested_max_items}, required_items={required_items}, max_tokens={requested_max_tokens}, required_tokens={required_tokens}"
            ),
            path: Some("budget".to_string()),
            requirement: None,
        },
        GetEvidenceBundleError::UnsatisfiedRequirement {
            requirement,
            message,
        } if is_context_requirement(requirement) => ToolError {
            code: ToolErrorCode::ContextUnavailable,
            message,
            path: None,
            requirement: None,
        },
        GetEvidenceBundleError::UnsatisfiedRequirement {
            requirement,
            message,
        } => ToolError {
            code: ToolErrorCode::RequirementUnsatisfied,
            message,
            path: None,
            requirement: Some(requirement.to_string()),
        },
        GetEvidenceBundleError::SourceLookup { message, .. } => ToolError {
            code: ToolErrorCode::SourceRefUnresolved,
            message,
            path: Some("bundle.items.source_refs".to_string()),
            requirement: None,
        },
        GetEvidenceBundleError::FixtureCorpusLoad(_)
        | GetEvidenceBundleError::MissingFixtureBundle { .. }
        | GetEvidenceBundleError::FixtureBundleParse { .. }
        | GetEvidenceBundleError::FixtureReplay(_)
        | GetEvidenceBundleError::InvalidFixtureBundle(_)
        | GetEvidenceBundleError::EvidenceCompile(_)
        | GetEvidenceBundleError::HotStore(_) => ToolError {
            code: ToolErrorCode::InternalError,
            message: "internal Janus error while handling get_evidence_bundle".to_string(),
            path: None,
            requirement: None,
        },
    }
}

pub fn write_schema_files(schema_dir: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let schema_dir = schema_dir.as_ref();
    fs::create_dir_all(schema_dir)?;

    write_schema_file(
        &schema_dir.join("get-evidence-bundle.input.schema.json"),
        &get_evidence_bundle_input_schema(),
    )?;
    write_schema_file(
        &schema_dir.join("get-evidence-bundle.output.schema.json"),
        &get_evidence_bundle_output_schema(),
    )?;

    Ok(())
}

impl From<GetEvidenceBundleToolInput> for EvidenceQuery {
    fn from(input: GetEvidenceBundleToolInput) -> Self {
        Self {
            intent: input.intent,
            time_window: input.time_window,
            budget: input.budget,
            scenario_id: Some(input.scenario_id),
            entities: input.entities,
            require_counter_evidence: input.require_counter_evidence,
            require_raw_refs: input.require_raw_refs,
            freshness: input.freshness,
            privacy_scope: input.privacy_scope,
        }
    }
}

impl fmt::Display for ToolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ToolError {}

impl fmt::Display for ToolErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            ToolErrorCode::InvalidRequest => "invalid_request",
            ToolErrorCode::FixtureNotFound => "fixture_not_found",
            ToolErrorCode::ContextUnavailable => "context_unavailable",
            ToolErrorCode::BudgetUnsatisfied => "budget_unsatisfied",
            ToolErrorCode::RequirementUnsatisfied => "requirement_unsatisfied",
            ToolErrorCode::SourceRefUnresolved => "source_ref_unresolved",
            ToolErrorCode::InternalError => "internal_error",
        };
        formatter.write_str(code)
    }
}

fn invalid_arguments_error(error: serde_json::Error) -> ToolError {
    ToolError {
        code: ToolErrorCode::InvalidRequest,
        message: format!("invalid tool arguments: {error}"),
        path: None,
        requirement: None,
    }
}

fn is_context_requirement(requirement: &str) -> bool {
    matches!(
        requirement,
        REQUIREMENT_HOT_CONTEXT_TIME_WINDOW
            | REQUIREMENT_HOT_CONTEXT_ENTITIES
            | REQUIREMENT_HOT_CONTEXT_TIME_WINDOW_ENTITIES
    )
}

pub fn is_public_evidence_requirement(requirement: &str) -> bool {
    matches!(
        requirement,
        REQUIREMENT_COUNTER_EVIDENCE | REQUIREMENT_RAW_REFS
    )
}

fn write_schema_file(path: &Path, schema: &RootSchema) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(schema)?;
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}
