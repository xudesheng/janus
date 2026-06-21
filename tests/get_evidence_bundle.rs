use janus::{
    evidence::{EvidenceBundle, EvidenceDirection, EvidenceKind, TimeWindow},
    fixture_validation::{FixtureCase, FixtureCorpus},
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
    token_query.budget.max_tokens = 585;

    let error = get_evidence_bundle(token_query).unwrap_err();
    assert!(matches!(
        error,
        GetEvidenceBundleError::UnsupportedBudget {
            requested_max_tokens: 585,
            required_tokens: 586,
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
fn rejects_unsatisfied_counter_evidence_requirement() {
    let mut query = coincidental_deploy_trap_query();
    query.require_counter_evidence = true;
    query.budget.min_counter_evidence_items = Some(3);

    let error = get_evidence_bundle(query).unwrap_err();
    assert!(matches!(
        error,
        GetEvidenceBundleError::UnsatisfiedRequirement {
            requirement: "counter_evidence",
            ..
        }
    ));
}

#[test]
fn serializes_returned_bundle_to_json() {
    let bundle = get_evidence_bundle(deploy_bad_rollout_query()).unwrap();
    let json = serde_json::to_value(&bundle).unwrap();

    assert_eq!(json["items"][0]["source_refs"][0]["signal"], "change");
    assert_eq!(json["budget"]["tokens_used"], 586);
}

#[test]
fn store_aware_path_resolves_source_refs_for_all_current_fixtures() {
    let corpus = FixtureCorpus::load(repo_root()).unwrap();

    for case in &corpus.cases {
        let bundle = get_evidence_bundle(query_for_case(case)).unwrap_or_else(|error| {
            panic!(
                "get_evidence_bundle failed for {}: {error}",
                case.registry_entry.id
            )
        });
        let expected: EvidenceBundle =
            serde_json::from_value(case.expected["evidence_bundle"].clone()).unwrap();

        assert_eq!(bundle, expected);
    }
}

#[test]
fn entity_selector_is_checked_without_rewriting_gold_bundle() {
    let mut query = deploy_bad_rollout_query();
    query.entities = vec!["service:checkout".to_string()];

    let bundle = get_evidence_bundle(query).unwrap();
    let expected = load_bundle_by_scenario_id("deploy-bad-rollout").unwrap();

    assert_eq!(bundle, expected);
}

#[test]
fn query_time_and_entity_must_match_same_hot_context_record() {
    let mut query = deploy_bad_rollout_query();
    query.time_window = TimeWindow {
        start: "2026-06-01T14:03:20Z".to_string(),
        end: "2026-06-01T14:03:22Z".to_string(),
    };
    query.entities = vec!["res:api-gateway".to_string()];

    let error = get_evidence_bundle(query).unwrap_err();

    assert!(matches!(
        error,
        GetEvidenceBundleError::UnsatisfiedRequirement {
            requirement: "hot_context_time_window_entities",
            ..
        }
    ));
}

#[test]
fn query_entity_without_hot_context_match_fails() {
    let mut query = deploy_bad_rollout_query();
    query.entities = vec!["service:not-present".to_string()];

    let error = get_evidence_bundle(query).unwrap_err();

    assert!(matches!(
        error,
        GetEvidenceBundleError::UnsatisfiedRequirement {
            requirement: "hot_context_entities",
            ..
        }
    ));
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

fn repo_root() -> &'static std::path::Path {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn query_for_case(case: &FixtureCase) -> EvidenceQuery {
    EvidenceQuery {
        intent: EvidenceQueryIntent {
            question: Some(case.manifest.question.clone()),
            hypothesis: None,
        },
        time_window: serde_json::from_value(case.manifest.time_window.clone()).unwrap(),
        budget: EvidenceQueryBudget {
            max_items: 50,
            max_tokens: 10_000,
            min_counter_evidence_items: None,
            reserve_tokens_for_raw_refs: None,
        },
        scenario_id: Some(case.registry_entry.id.clone()),
        entities: Vec::new(),
        require_counter_evidence: false,
        require_raw_refs: true,
        freshness: FreshnessPreference::Any,
        privacy_scope: None,
    }
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
            max_tokens: 586,
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
            max_tokens: 642,
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
