use janus::{
    evidence::{EvidenceDirection, EvidenceKind, TimeWindow},
    fixtures::load_bundle_by_scenario_id,
    query::{
        EvidenceQuery, EvidenceQueryBudget, EvidenceQueryIntent, FreshnessPreference,
        GetEvidenceBundleError, evidence_query_schema, get_evidence_bundle,
    },
};
use serde_json::Value;

#[test]
fn returns_deploy_bad_rollout_gold_bundle() {
    let query = deploy_bad_rollout_query();
    let bundle = get_evidence_bundle(query).unwrap();
    let expected = load_bundle_by_scenario_id("deploy-bad-rollout").unwrap();

    assert_eq!(bundle, expected);
}

#[test]
fn preserves_counter_evidence_for_coincidental_deploy_trap() {
    let mut query = coincidental_deploy_trap_query();
    query.require_counter_evidence = true;
    query.budget.min_counter_evidence_items = Some(2);

    let bundle = get_evidence_bundle(query).unwrap();
    let counter_ids: Vec<&str> = bundle
        .items
        .iter()
        .filter(|item| {
            item.kind == EvidenceKind::CounterEvidence
                || matches!(
                    item.direction,
                    EvidenceDirection::Weakens | EvidenceDirection::Contradicts
                )
        })
        .map(|item| item.id.as_str())
        .collect();

    assert_eq!(counter_ids, vec!["ev-3", "ev-4"]);
}

#[test]
fn rejects_query_without_question_or_hypothesis() {
    let mut query = deploy_bad_rollout_query();
    query.intent.question = None;
    query.intent.hypothesis = None;

    let error = get_evidence_bundle(query).unwrap_err();
    let errors = invalid_query_errors(error);

    assert!(
        errors
            .iter()
            .any(|error| error.path == "EvidenceQuery.intent")
    );
}

#[test]
fn rejects_missing_or_unsafe_scenario_id() {
    let mut missing = deploy_bad_rollout_query();
    missing.scenario_id = None;

    let error = get_evidence_bundle(missing).unwrap_err();
    let errors = invalid_query_errors(error);
    assert!(
        errors
            .iter()
            .any(|error| error.path == "EvidenceQuery.scenario_id")
    );

    for scenario_id in ["../deploy-bad-rollout", r"..\deploy-bad-rollout", "", "."] {
        let mut query = deploy_bad_rollout_query();
        query.scenario_id = Some(scenario_id.to_string());

        let error = get_evidence_bundle(query).unwrap_err();
        let errors = invalid_query_errors(error);

        assert!(
            errors
                .iter()
                .any(|error| error.path == "EvidenceQuery.scenario_id"),
            "expected invalid scenario id error for {scenario_id:?}"
        );
    }
}

#[test]
fn rejects_budget_that_cannot_fit_gold_bundle() {
    let mut token_query = deploy_bad_rollout_query();
    token_query.budget.max_tokens = 249;

    let error = get_evidence_bundle(token_query).unwrap_err();
    assert!(matches!(
        error,
        GetEvidenceBundleError::UnsupportedBudget {
            requested_max_tokens: 249,
            required_tokens: 250,
            ..
        }
    ));

    let mut item_query = deploy_bad_rollout_query();
    item_query.budget.max_items = 4;

    let error = get_evidence_bundle(item_query).unwrap_err();
    assert!(matches!(
        error,
        GetEvidenceBundleError::UnsupportedBudget {
            requested_max_items: 4,
            required_items: 5,
            ..
        }
    ));
}

#[test]
fn serializes_returned_bundle_to_json() {
    let bundle = get_evidence_bundle(deploy_bad_rollout_query()).unwrap();
    let json = serde_json::to_value(&bundle).unwrap();

    assert_eq!(json["items"][0]["source_refs"][0]["signal"], "change");
    assert_eq!(json["budget"]["tokens_used"], 250);
}

#[test]
fn query_schema_requires_core_fields_not_fixture_selector() {
    let schema = serde_json::to_value(evidence_query_schema()).unwrap();
    let required = schema
        .pointer("/required")
        .and_then(Value::as_array)
        .unwrap();

    assert!(required.iter().any(|value| value == "intent"));
    assert!(required.iter().any(|value| value == "time_window"));
    assert!(required.iter().any(|value| value == "budget"));
    assert!(!required.iter().any(|value| value == "scenario_id"));
}

fn deploy_bad_rollout_query() -> EvidenceQuery {
    EvidenceQuery {
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
            max_tokens: 250,
            min_counter_evidence_items: None,
            reserve_tokens_for_raw_refs: None,
        },
        scenario_id: Some("deploy-bad-rollout".to_string()),
        entities: Vec::new(),
        require_counter_evidence: false,
        require_raw_refs: true,
        freshness: FreshnessPreference::Any,
        privacy_scope: None,
    }
}

fn coincidental_deploy_trap_query() -> EvidenceQuery {
    EvidenceQuery {
        intent: EvidenceQueryIntent {
            question: Some(
                "Why did search start failing around 15:03 on 2026-06-08, right after the search-ui deploy?"
                    .to_string(),
            ),
            hypothesis: None,
        },
        time_window: TimeWindow {
            start: "2026-06-08T14:55:00Z".to_string(),
            end: "2026-06-08T15:20:00Z".to_string(),
        },
        budget: EvidenceQueryBudget {
            max_items: 5,
            max_tokens: 380,
            min_counter_evidence_items: None,
            reserve_tokens_for_raw_refs: None,
        },
        scenario_id: Some("coincidental-deploy-trap".to_string()),
        entities: Vec::new(),
        require_counter_evidence: false,
        require_raw_refs: true,
        freshness: FreshnessPreference::Any,
        privacy_scope: None,
    }
}

fn invalid_query_errors(error: GetEvidenceBundleError) -> Vec<janus::query::QueryValidationError> {
    match error {
        GetEvidenceBundleError::InvalidQuery(errors) => errors.errors().to_vec(),
        other => panic!("expected invalid query, got {other:?}"),
    }
}
