use janus::{
    evidence::TimeWindow,
    mcp::{
        GetEvidenceBundleToolInput, ToolErrorCode, call_get_evidence_bundle,
        get_evidence_bundle_input_schema, get_evidence_bundle_output_schema,
        get_evidence_bundle_tool_definition, tool_error_from_get_evidence_bundle,
    },
    query::{
        EvidenceQueryBudget, EvidenceQueryIntent, FreshnessPreference, GetEvidenceBundleError,
    },
};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

#[test]
fn generated_mcp_schemas_match_committed_artifacts() {
    assert_schema_matches(
        "schemas/mcp/get-evidence-bundle.input.schema.json",
        serde_json::to_value(get_evidence_bundle_input_schema()).unwrap(),
    );
    assert_schema_matches(
        "schemas/mcp/get-evidence-bundle.output.schema.json",
        serde_json::to_value(get_evidence_bundle_output_schema()).unwrap(),
    );
}

#[test]
fn mcp_input_schema_requires_scenario_id() {
    let schema = serde_json::to_value(get_evidence_bundle_input_schema()).unwrap();
    let required = schema
        .pointer("/required")
        .and_then(Value::as_array)
        .expect("input schema should have root required list");

    for field in ["scenario_id", "intent", "time_window", "budget"] {
        assert!(
            required.iter().any(|value| value == field),
            "input schema should require {field}"
        );
    }
}

#[test]
fn generated_mcp_array_schemas_declare_items() {
    let input_schema = serde_json::to_value(get_evidence_bundle_input_schema()).unwrap();
    let output_schema = serde_json::to_value(get_evidence_bundle_output_schema()).unwrap();

    assert_arrays_have_items("$GetEvidenceBundleToolInput", &input_schema);
    assert_arrays_have_items("$GetEvidenceBundleToolOutput", &output_schema);
}

#[test]
fn committed_mcp_schemas_compile_under_draft7_validator() {
    for path in [
        "schemas/mcp/get-evidence-bundle.input.schema.json",
        "schemas/mcp/get-evidence-bundle.output.schema.json",
    ] {
        let schema = read_json(path);
        jsonschema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .build(&schema)
            .unwrap_or_else(|error| panic!("{path} did not compile as draft-07: {error}"));
    }
}

#[test]
fn mcp_schemas_validate_valid_tool_input_and_output() {
    let input_schema = read_json("schemas/mcp/get-evidence-bundle.input.schema.json");
    let output_schema = read_json("schemas/mcp/get-evidence-bundle.output.schema.json");
    let input_validator = draft7_validator(&input_schema);
    let output_validator = draft7_validator(&output_schema);
    let input = deploy_bad_rollout_tool_arguments();

    input_validator
        .validate(&input)
        .unwrap_or_else(|error| panic!("valid MCP input failed validation: {error}"));

    let output = call_get_evidence_bundle(input).unwrap();
    let output_json = serde_json::to_value(output).unwrap();

    output_validator
        .validate(&output_json)
        .unwrap_or_else(|error| panic!("valid MCP output failed validation: {error}"));
}

#[test]
fn tool_definition_advertises_get_evidence_bundle_contract() {
    let definition = get_evidence_bundle_tool_definition();

    assert_eq!(definition.name, "get_evidence_bundle");
    assert_eq!(
        definition.input_schema,
        serde_json::to_value(get_evidence_bundle_input_schema()).unwrap()
    );
    assert_eq!(
        definition.output_schema,
        serde_json::to_value(get_evidence_bundle_output_schema()).unwrap()
    );
}

#[test]
fn tool_input_converts_to_internal_query_with_fixture_selector() {
    let input = deploy_bad_rollout_tool_input();
    let query: janus::query::EvidenceQuery = input.into();

    assert_eq!(query.scenario_id.as_deref(), Some("deploy-bad-rollout"));
    assert!(query.require_raw_refs);
    assert_eq!(query.freshness, FreshnessPreference::Any);
}

#[test]
fn tool_handler_returns_bundle_envelope() {
    let output = call_get_evidence_bundle(deploy_bad_rollout_tool_arguments()).unwrap();

    assert_eq!(
        output.bundle.question.as_deref(),
        Some("Why did checkout start returning 5xx around 14:05 on 2026-06-01?")
    );
    assert!(!output.bundle.items.is_empty());
}

#[test]
fn missing_required_tool_field_is_invalid_request() {
    let mut arguments = deploy_bad_rollout_tool_arguments();
    arguments.as_object_mut().unwrap().remove("scenario_id");

    let error = call_get_evidence_bundle(arguments).unwrap_err();

    assert_eq!(error.code, ToolErrorCode::InvalidRequest);
}

#[test]
fn unknown_fixture_selector_maps_to_fixture_not_found() {
    let mut arguments = deploy_bad_rollout_tool_arguments();
    arguments["scenario_id"] = json!("not-a-fixture");

    let error = call_get_evidence_bundle(arguments).unwrap_err();

    assert_eq!(error.code, ToolErrorCode::FixtureNotFound);
    assert_eq!(error.path.as_deref(), Some("scenario_id"));
}

#[test]
fn counter_evidence_requirement_maps_to_tool_requirement_error() {
    let mut arguments = coincidental_deploy_trap_tool_arguments();
    arguments["require_counter_evidence"] = json!(true);
    arguments["budget"]["min_counter_evidence_items"] = json!(99);

    let error = call_get_evidence_bundle(arguments).unwrap_err();

    assert_eq!(error.code, ToolErrorCode::RequirementUnsatisfied);
    assert_eq!(error.requirement.as_deref(), Some("counter_evidence"));
}

#[test]
fn raw_refs_requirement_maps_to_tool_requirement_error() {
    let error =
        tool_error_from_get_evidence_bundle(GetEvidenceBundleError::UnsatisfiedRequirement {
            requirement: "raw_refs",
            message: "missing source refs".to_string(),
        });

    assert_eq!(error.code, ToolErrorCode::RequirementUnsatisfied);
    assert_eq!(error.requirement.as_deref(), Some("raw_refs"));
}

#[test]
fn hot_context_requirement_maps_to_context_unavailable() {
    let mut arguments = deploy_bad_rollout_tool_arguments();
    arguments["entities"] = json!(["service:not-present"]);

    let error = call_get_evidence_bundle(arguments).unwrap_err();

    assert_eq!(error.code, ToolErrorCode::ContextUnavailable);
    assert_eq!(error.requirement.as_deref(), Some("hot_context_entities"));
}

fn deploy_bad_rollout_tool_input() -> GetEvidenceBundleToolInput {
    GetEvidenceBundleToolInput {
        scenario_id: "deploy-bad-rollout".to_string(),
        intent: EvidenceQueryIntent {
            question: Some(
                "Why did checkout start returning 5xx around 14:05 on 2026-06-01?".to_string(),
            ),
            hypothesis: None,
        },
        time_window: TimeWindow {
            start: "2026-06-01T14:00:00Z".to_string(),
            end: "2026-06-01T14:15:00Z".to_string(),
        },
        budget: EvidenceQueryBudget {
            max_items: 5,
            max_tokens: 586,
            min_counter_evidence_items: None,
            reserve_tokens_for_raw_refs: None,
        },
        entities: Vec::new(),
        require_counter_evidence: false,
        require_raw_refs: true,
        freshness: FreshnessPreference::Any,
        privacy_scope: None,
    }
}

fn deploy_bad_rollout_tool_arguments() -> Value {
    serde_json::to_value(deploy_bad_rollout_tool_input()).unwrap()
}

fn coincidental_deploy_trap_tool_arguments() -> Value {
    json!({
        "scenario_id": "coincidental-deploy-trap",
        "intent": {
            "question": "Why did search start failing around 15:03 on 2026-06-08, right after the search-ui deploy?"
        },
        "time_window": {
            "start": "2026-06-08T14:55:00Z",
            "end": "2026-06-08T15:20:00Z"
        },
        "budget": {
            "max_items": 5,
            "max_tokens": 642
        }
    })
}

fn assert_schema_matches(path: &str, generated: Value) {
    let committed = read_json(path);

    assert_eq!(
        generated,
        committed,
        "schema mismatch for {}",
        repo_path(path).display()
    );
}

fn assert_arrays_have_items(path: &str, value: &Value) {
    match value {
        Value::Object(object) => {
            if object.get("type").is_some_and(is_array_type) {
                assert!(
                    object.contains_key("items"),
                    "array schema at {path} is missing items"
                );
            }

            for (key, child) in object {
                assert_arrays_have_items(&format!("{path}.{key}"), child);
            }
        }
        Value::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                assert_arrays_have_items(&format!("{path}[{index}]"), child);
            }
        }
        _ => {}
    }
}

fn is_array_type(value: &Value) -> bool {
    match value {
        Value::String(value) => value == "array",
        Value::Array(values) => values.iter().any(|value| value == "array"),
        _ => false,
    }
}

fn draft7_validator(schema: &Value) -> jsonschema::Validator {
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .build(schema)
        .expect("schema should compile as draft-07")
}

fn read_json(path: &str) -> Value {
    serde_json::from_str(&fs::read_to_string(repo_path(path)).unwrap()).unwrap()
}

fn repo_path(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)
}
