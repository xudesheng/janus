use janus::{
    evidence::{evidence_bundle_schema, evidence_item_schema},
    fixtures::{FixtureLoadError, load_bundle_by_scenario_id, load_bundle_from_expected_path},
    query::evidence_query_schema,
};
use serde_json::Value;
use std::{fs, path::PathBuf};

#[test]
fn current_fixture_bundles_deserialize_and_validate() {
    let paths = fixture_expected_paths();
    assert!(!paths.is_empty(), "expected at least one fixture");

    for path in paths {
        let bundle = load_bundle_from_expected_path(&path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        bundle
            .validate()
            .unwrap_or_else(|error| panic!("invalid {}: {error}", path.display()));
    }
}

#[test]
fn can_load_bundle_by_scenario_id() {
    let bundle = load_bundle_by_scenario_id("deploy-bad-rollout").unwrap();

    assert_eq!(
        bundle.question.as_deref(),
        Some("Why did checkout start returning 5xx around 14:05 on 2026-06-01?")
    );
    assert_eq!(bundle.items.len(), 5);
}

#[test]
fn scenario_id_loader_rejects_path_traversal() {
    for scenario_id in ["../deploy-bad-rollout", r"..\deploy-bad-rollout", "", "."] {
        let error = load_bundle_by_scenario_id(scenario_id).unwrap_err();
        assert!(matches!(error, FixtureLoadError::InvalidScenarioId(_)));
    }
}

#[test]
fn can_serialize_a_bundle_back_to_json() {
    let bundle = load_bundle_by_scenario_id("deploy-bad-rollout").unwrap();
    let json = serde_json::to_value(&bundle).unwrap();

    assert_eq!(json["items"][0]["source_refs"][0]["signal"], "change");
    assert!(json["items"][0].get("confidence").is_some());
}

#[test]
fn generated_schemas_match_committed_artifacts() {
    assert_schema_matches(
        "schemas/evidence-ir/evidence-item.schema.json",
        serde_json::to_value(evidence_item_schema()).unwrap(),
    );
    assert_schema_matches(
        "schemas/evidence-ir/evidence-bundle.schema.json",
        serde_json::to_value(evidence_bundle_schema()).unwrap(),
    );
    assert_schema_matches(
        "schemas/evidence-ir/evidence-query.schema.json",
        serde_json::to_value(evidence_query_schema()).unwrap(),
    );
}

#[test]
fn generated_array_schemas_declare_items() {
    let item_schema = serde_json::to_value(evidence_item_schema()).unwrap();
    let bundle_schema = serde_json::to_value(evidence_bundle_schema()).unwrap();
    let query_schema = serde_json::to_value(evidence_query_schema()).unwrap();

    assert_arrays_have_items("$EvidenceItem", &item_schema);
    assert_arrays_have_items("$EvidenceBundle", &bundle_schema);
    assert_arrays_have_items("$EvidenceQuery", &query_schema);
}

#[test]
fn source_refs_schema_requires_at_least_one_ref() {
    let schema = serde_json::to_value(evidence_item_schema()).unwrap();
    let min_items = schema
        .pointer("/definitions/SourceRefs/minItems")
        .and_then(Value::as_u64);

    assert_eq!(min_items, Some(1));
}

fn fixture_expected_paths() -> Vec<PathBuf> {
    let scenarios_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/scenarios");
    let mut paths = Vec::new();

    for entry in fs::read_dir(&scenarios_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path().join("expected.json");

        if path.is_file() {
            paths.push(path);
        }
    }

    paths.sort();
    paths
}

fn assert_schema_matches(path: &str, generated: Value) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
    let committed: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();

    assert_eq!(
        generated,
        committed,
        "schema mismatch for {}",
        path.display()
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
