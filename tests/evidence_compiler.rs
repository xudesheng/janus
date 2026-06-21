use janus::{
    derived_context::derive_full_context,
    evidence_compiler::{
        EvidenceCompilerInput, apply_compiler_token_estimates,
        canonical_evidence_item_payload_json_bytes, compare_compiled_evidence,
        compare_compiled_evidence_for_case, estimate_evidence_item_tokens,
        load_expected_compilation,
    },
    fixture_validation::{FixtureCorpus, FixtureSelector},
    hot_context_store::HotContextStore,
    query::{EvidenceQuery, EvidenceQueryBudget, EvidenceQueryIntent, FreshnessPreference},
};

#[test]
fn loads_expected_compilation_artifacts_for_comparison_only() {
    let case = fixture_case("deploy-bad-rollout");
    let expected = load_expected_compilation(&case).unwrap();

    assert_eq!(expected.bundle.items.len(), 5);
    assert_eq!(expected.suspected_causes.len(), 2);
    assert_eq!(expected.next_checks.len(), 3);
    assert_eq!(expected.report.selected_items, 5);
}

#[test]
fn comparison_accepts_gold_clone() {
    let case = fixture_case("deploy-bad-rollout");
    let expected = load_expected_compilation(&case).unwrap();

    let comparison = compare_compiled_evidence_for_case(&case, &expected).unwrap();

    assert!(!comparison.has_expected_mismatches());
}

#[test]
fn text_fields_are_structural_not_verbatim() {
    let case = fixture_case("deploy-bad-rollout");
    let expected = load_expected_compilation(&case).unwrap();
    let mut actual = expected.clone();

    actual.bundle.items[0].claim = "Different deterministic wording for the same evidence.".into();
    actual.suspected_causes[0].hypothesis = "Different deterministic suspected-cause text.".into();
    actual.next_checks[0].action = "Inspect the same code path with different wording.".into();

    let comparison = compare_compiled_evidence(&expected, &actual);

    assert!(!comparison.has_expected_mismatches());
    assert_eq!(comparison.text_differences.len(), 3);
}

#[test]
fn empty_required_text_is_a_mismatch() {
    let case = fixture_case("deploy-bad-rollout");
    let expected = load_expected_compilation(&case).unwrap();
    let mut actual = expected.clone();

    actual.bundle.items[0].claim.clear();

    let comparison = compare_compiled_evidence(&expected, &actual);

    assert!(comparison.has_expected_mismatches());
    assert!(
        comparison
            .item_mismatches
            .iter()
            .any(|mismatch| mismatch.field == "claim")
    );
}

#[test]
fn suspected_cause_reasons_are_exact_category_sets() {
    let case = fixture_case("deploy-bad-rollout");
    let expected = load_expected_compilation(&case).unwrap();
    let mut actual = expected.clone();

    actual.suspected_causes[0].reasons = vec!["different_reason".to_string()];

    let comparison = compare_compiled_evidence(&expected, &actual);

    assert!(comparison.has_expected_mismatches());
    assert!(
        comparison
            .suspected_cause_mismatches
            .iter()
            .any(|mismatch| mismatch.field == "reasons")
    );
}

#[test]
fn canonical_token_payload_is_pinned() {
    let case = fixture_case("deploy-bad-rollout");
    let expected = load_expected_compilation(&case).unwrap();
    let item = &expected.bundle.items[0];

    let bytes = canonical_evidence_item_payload_json_bytes(item).unwrap();

    assert_eq!(bytes.len(), 486);
    assert_eq!(estimate_evidence_item_tokens(item).unwrap(), 122);
}

#[test]
fn current_fixture_token_fields_match_compiler_estimator() {
    let corpus = FixtureCorpus::load(repo_root()).unwrap();
    let mut mismatches = Vec::new();

    for case in &corpus.cases {
        let expected = load_expected_compilation(case).unwrap();
        let mut estimated_bundle = expected.bundle.clone();
        apply_compiler_token_estimates(&mut estimated_bundle).unwrap();

        if expected.bundle.budget.tokens_used != estimated_bundle.budget.tokens_used {
            mismatches.push(format!(
                "{} budget.tokens_used {} -> {}",
                case.registry_entry.id,
                expected.bundle.budget.tokens_used,
                estimated_bundle.budget.tokens_used
            ));
        }

        for (expected_item, estimated_item) in expected
            .bundle
            .items
            .iter()
            .zip(estimated_bundle.items.iter())
        {
            if expected_item.token_cost != estimated_item.token_cost {
                mismatches.push(format!(
                    "{} {} token_cost {} -> {}",
                    case.registry_entry.id,
                    expected_item.id,
                    expected_item.token_cost,
                    estimated_item.token_cost
                ));
            }
        }
    }

    assert!(
        mismatches.is_empty(),
        "fixture token fields need estimator migration:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn runtime_input_excludes_expected_artifacts() {
    let case = fixture_case("deploy-bad-rollout");
    let query = query_for_case(&case);
    let store = HotContextStore::load_fixture_case(&case).unwrap();
    let derived = derive_full_context(&case, &store);

    let input = EvidenceCompilerInput {
        query: &query,
        store: &store,
        derived: &derived,
    };

    assert_eq!(
        input.query.intent.question.as_deref(),
        Some(case.manifest.question.as_str())
    );
    assert!(input.store.record_count() > 0);
    assert!(!input.derived.anomaly_windows.is_empty());
}

fn fixture_case(id: &str) -> janus::fixture_validation::FixtureCase {
    let corpus = FixtureCorpus::load(repo_root()).unwrap();
    let selector = FixtureSelector {
        fixture_id: Some(id.to_string()),
        ..FixtureSelector::default()
    };

    corpus.select(&selector).into_iter().next().unwrap().clone()
}

fn query_for_case(case: &janus::fixture_validation::FixtureCase) -> EvidenceQuery {
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

fn repo_root() -> &'static std::path::Path {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
}
