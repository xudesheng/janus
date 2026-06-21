use janus::{
    evidence::{EvidenceBundle, EvidenceDirection, EvidenceKind, TimeWindow},
    fixture_validation::{FixtureCase, FixtureCorpus},
    query::{
        EvidenceQuery, EvidenceQueryBudget, EvidenceQueryIntent, FreshnessPreference,
        GetEvidenceBundleError, evidence_query_schema, get_evidence_bundle,
    },
};
use serde_json::Value;

#[test]
fn returns_compiled_deploy_bad_rollout_bundle() {
    let query = deploy_bad_rollout_query();
    let bundle = get_evidence_bundle(query.clone()).unwrap();

    assert_compiled_bundle_contract(&query, &bundle);
    assert!(
        bundle
            .items
            .iter()
            .any(|item| item.kind == EvidenceKind::ChangeEvent),
        "deploy scenario should include source-backed change evidence: {bundle:#?}"
    );
}

#[test]
fn preserves_counter_evidence_for_coincidental_deploy_trap() {
    let mut query = coincidental_deploy_trap_query();
    query.require_counter_evidence = true;
    query.budget.min_counter_evidence_items = Some(2);
    query.budget.max_items = 6;
    query.budget.max_tokens = 10_000;

    let bundle = get_evidence_bundle(query).unwrap();
    let counter_items: Vec<&janus::evidence::EvidenceItem> = bundle
        .items
        .iter()
        .filter(|item| {
            item.kind == EvidenceKind::CounterEvidence
                || matches!(
                    item.direction,
                    EvidenceDirection::Weakens | EvidenceDirection::Contradicts
                )
        })
        .collect();

    assert!(
        counter_items.len() >= 2,
        "required counter evidence should be preserved structurally: {bundle:#?}"
    );
    assert!(
        counter_items
            .iter()
            .all(|item| !item.source_refs.is_empty())
    );
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
fn respects_small_budget_by_dropping_whole_items() {
    let mut query = deploy_bad_rollout_query();
    query.budget.max_items = 2;
    query.budget.max_tokens = 10_000;

    let bundle = get_evidence_bundle(query.clone()).unwrap();

    assert_compiled_bundle_contract(&query, &bundle);
    assert!(bundle.items.len() <= 2);
    assert!(bundle.budget.items_dropped > 0);
}

#[test]
fn rejects_unsatisfied_counter_evidence_requirement() {
    let mut query = coincidental_deploy_trap_query();
    query.require_counter_evidence = true;
    query.budget.min_counter_evidence_items = Some(99);

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
    let query = deploy_bad_rollout_query();
    let bundle = get_evidence_bundle(query.clone()).unwrap();
    let json = serde_json::to_value(&bundle).unwrap();

    assert_compiled_bundle_contract(&query, &bundle);
    assert!(
        json["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert!(json["items"][0]["source_refs"][0]["signal"].is_string());
    assert!(json["budget"]["tokens_used"].as_u64().unwrap() <= 586);
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

        assert_compiled_bundle_contract(&query_for_case(case), &bundle);
    }
}

#[test]
fn entity_selector_is_checked_without_rewriting_gold_bundle() {
    let mut query = deploy_bad_rollout_query();
    query.entities = vec!["service:checkout".to_string()];

    let bundle = get_evidence_bundle(query.clone()).unwrap();

    assert_compiled_bundle_contract(&query, &bundle);
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

fn assert_compiled_bundle_contract(query: &EvidenceQuery, bundle: &EvidenceBundle) {
    bundle.validate().unwrap();
    assert_eq!(bundle.question, query.intent.question);
    assert_eq!(bundle.hypothesis, query.intent.hypothesis);
    assert_eq!(bundle.time_window, query.time_window);
    assert!(bundle.items.len() <= query.budget.max_items as usize);
    assert!(bundle.budget.tokens_used <= query.budget.max_tokens);
    assert!(!bundle.items.is_empty());

    let token_sum = bundle
        .items
        .iter()
        .fold(0u32, |sum, item| sum + item.token_cost);
    assert_eq!(bundle.budget.tokens_used, token_sum);

    for (index, item) in bundle.items.iter().enumerate() {
        assert_eq!(item.id, format!("ev-{}", index + 1));
        assert!(
            !item.source_refs.is_empty(),
            "{} should retain source refs",
            item.id
        );
    }
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
