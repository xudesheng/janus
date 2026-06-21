use janus::{
    derived_context::{derive_and_insert_context, derive_full_context},
    evidence_compiler::{
        EvidenceCandidateSource, EvidenceCompileError, EvidenceCompilerInput, NextCheck,
        SuspectedCause, apply_compiler_token_estimates, canonical_evidence_item_payload_json_bytes,
        compare_compiled_evidence, compare_compiled_evidence_for_case, compile_evidence,
        estimate_evidence_item_tokens, generate_evidence_candidates, load_expected_compilation,
        rank_suspected_causes_from_candidates, score_evidence_candidates,
    },
    fixture_simulator::{plan_fixture_replay, replay_plan_into_store},
    fixture_validation::{FixtureCorpus, FixtureSelector},
    hot_context_store::{HotContextStore, SourceResolution},
    query::{EvidenceQuery, EvidenceQueryBudget, EvidenceQueryIntent, FreshnessPreference},
};
use std::collections::BTreeSet;

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
fn suspected_cause_reasons_reject_unknown_category_tokens() {
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
fn suspected_cause_reasons_accept_expected_subset() {
    let case = fixture_case("deploy-bad-rollout");
    let expected = load_expected_compilation(&case).unwrap();
    let mut actual = expected.clone();

    actual.suspected_causes[0].reasons = vec![expected.suspected_causes[0].reasons[0].clone()];

    let comparison = compare_compiled_evidence(&expected, &actual);

    assert!(!comparison.has_expected_mismatches());
}

#[test]
fn extra_suspected_causes_and_next_checks_are_mismatches() {
    let case = fixture_case("deploy-bad-rollout");
    let expected = load_expected_compilation(&case).unwrap();
    let mut actual = expected.clone();

    actual.suspected_causes.push(SuspectedCause {
        rank: 99,
        entity: "service:unrelated".to_string(),
        hypothesis: "unrelated extra cause".to_string(),
        score: janus::evidence::UnitInterval(0.1),
        reasons: vec!["extra".to_string()],
        supporting: Vec::new(),
        counter: Vec::new(),
        note: None,
        trap_note: None,
    });
    actual.next_checks.push(NextCheck {
        action: "Inspect unrelated service.".to_string(),
        rationale: "extra check should fail comparison".to_string(),
        expected_signal: "metric_anomaly".to_string(),
    });

    let comparison = compare_compiled_evidence(&expected, &actual);

    assert!(comparison.has_expected_mismatches());
    assert_eq!(comparison.extra_suspected_causes, vec![99]);
    assert_eq!(comparison.extra_next_checks, vec![3]);
}

#[test]
fn next_check_expected_signal_is_exact_category_token() {
    let case = fixture_case("deploy-bad-rollout");
    let expected = load_expected_compilation(&case).unwrap();
    let mut actual = expected.clone();

    actual.next_checks[0].expected_signal = "log_cluster".to_string();

    let comparison = compare_compiled_evidence(&expected, &actual);

    assert!(comparison.has_expected_mismatches());
    assert!(
        comparison
            .next_check_mismatches
            .iter()
            .any(|mismatch| mismatch.field == "expected_signal")
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
fn generates_source_backed_candidates_for_current_corpus() {
    let corpus = FixtureCorpus::load(repo_root()).unwrap();
    let mut observed_sources = BTreeSet::new();

    for case in &corpus.cases {
        let query = query_for_case(case);
        let mut store = raw_fixture_store(case);
        let derived = derive_and_insert_context(case, &mut store).unwrap();
        let candidates = generate_evidence_candidates(EvidenceCompilerInput {
            query: &query,
            store: &store,
            derived: &derived,
        })
        .unwrap();
        let mut candidate_ids = BTreeSet::new();

        assert!(
            !candidates.is_empty(),
            "{} should generate at least one candidate",
            case.registry_entry.id
        );

        for candidate in &candidates {
            observed_sources.insert(candidate.source);
            assert_eq!(candidate.candidate_id, candidate.item.id);
            assert!(
                candidate.candidate_id.starts_with("cand-"),
                "{} has non-internal candidate id {}",
                case.registry_entry.id,
                candidate.candidate_id
            );
            assert!(
                candidate_ids.insert(candidate.candidate_id.clone()),
                "{} duplicate candidate id {}",
                case.registry_entry.id,
                candidate.candidate_id
            );
            candidate.item.validate().unwrap_or_else(|errors| {
                panic!(
                    "{} invalid candidate {}: {errors}",
                    case.registry_entry.id, candidate.candidate_id
                )
            });
            assert_eq!(
                candidate.item.token_cost,
                estimate_evidence_item_tokens(&candidate.item).unwrap(),
                "{} {} token cost should come from compiler estimator",
                case.registry_entry.id,
                candidate.candidate_id
            );

            for source_ref in candidate.item.source_refs.iter() {
                assert!(
                    matches!(
                        store.resolve_source_ref(source_ref),
                        SourceResolution::Found(_)
                    ),
                    "{} {} source ref should resolve: {:?}",
                    case.registry_entry.id,
                    candidate.candidate_id,
                    source_ref
                );
            }
        }
    }

    for source in [
        EvidenceCandidateSource::MetricAnomaly,
        EvidenceCandidateSource::LogPattern,
        EvidenceCandidateSource::ChangeEvent,
        EvidenceCandidateSource::TraceExemplar,
        EvidenceCandidateSource::DependencyEdge,
        EvidenceCandidateSource::PreviousIncident,
        EvidenceCandidateSource::MissingData,
        EvidenceCandidateSource::CounterEvidence,
    ] {
        assert!(
            observed_sources.contains(&source),
            "current corpus should exercise {source:?} candidates"
        );
    }
}

#[test]
fn scored_metric_strength_is_not_detector_confidence_copy() {
    let case = fixture_case("deploy-bad-rollout");
    let (query, store, derived, candidates) = scored_candidates_for_case(&case);
    let rescored = score_evidence_candidates(
        EvidenceCompilerInput {
            query: &query,
            store: &store,
            derived: &derived,
        },
        candidates,
    )
    .unwrap();

    let metric = rescored
        .iter()
        .find(|candidate| {
            candidate.source == EvidenceCandidateSource::MetricAnomaly
                && candidate.item.confidence.contains_key("detector")
        })
        .expect("deploy-bad-rollout should have a metric anomaly candidate");
    let detector = metric.item.confidence["detector"];

    assert!(
        (metric.item.strength.0 - detector.0).abs() > 1e-9,
        "metric strength must combine dimensions, not copy confidence.detector"
    );
    assert!(metric.item.confidence.contains_key("magnitude"));
    assert!(metric.item.confidence.contains_key("coverage"));
    assert!(metric.item.confidence.contains_key("source_ref_quality"));
}

#[test]
fn suspected_cause_ranking_downgrades_false_deploy_with_counter_evidence() {
    let case = fixture_case("coincidental-deploy-trap");
    let (query, store, derived, candidates) = scored_candidates_for_case(&case);
    let causes = rank_suspected_causes_from_candidates(
        EvidenceCompilerInput {
            query: &query,
            store: &store,
            derived: &derived,
        },
        &candidates,
    );

    let redis = causes
        .iter()
        .find(|cause| cause.entity == "infra:redis-cache")
        .expect("redis-cache should be ranked as a plausible cause");
    let search_ui = causes
        .iter()
        .find(|cause| cause.entity == "service:search-ui")
        .expect("coincidental search-ui deploy should remain inspectable");

    assert!(
        redis.rank < search_ui.rank,
        "actual cache failure should outrank the coincidental deploy: {causes:#?}"
    );
    assert!(redis.score.0 > search_ui.score.0);
    assert!(
        !search_ui.counter.is_empty(),
        "false deploy should carry explicit counter-evidence"
    );
    assert!(
        search_ui.trap_note.is_some(),
        "false deploy should be marked as a false-causality trap"
    );
}

#[test]
fn missing_data_ranking_surfaces_under_determined_uncertainty() {
    let case = fixture_case("missing-data-gap");
    let (query, store, derived, candidates) = scored_candidates_for_case(&case);
    let causes = rank_suspected_causes_from_candidates(
        EvidenceCompilerInput {
            query: &query,
            store: &store,
            derived: &derived,
        },
        &candidates,
    );

    let under_determined = causes
        .iter()
        .find(|cause| cause.entity == "under-determined")
        .expect("missing-data fixture should surface an uncertainty suspect");

    assert!(
        under_determined
            .reasons
            .iter()
            .any(|reason| reason == "telemetry_gap_across_peak")
    );
    assert!(!under_determined.supporting.is_empty());

    if let Some(concrete) = causes
        .iter()
        .find(|cause| cause.entity == "db:inventory-pg")
    {
        assert!(
            under_determined.score.0 > concrete.score.0,
            "under-determined should be less confident than a source-backed concrete cause only when the data gap is not material"
        );
    }
}

#[test]
fn compile_evidence_selects_ev_ids_and_reports_dropped_candidates() {
    let case = fixture_case("deploy-bad-rollout");
    let mut query = query_for_case(&case);
    query.budget.max_items = 3;
    query.budget.max_tokens = 10_000;
    let mut store = raw_fixture_store(&case);
    let derived = derive_and_insert_context(&case, &mut store).unwrap();

    let compilation = compile_evidence(&query, &store, &derived).unwrap();

    assert_eq!(compilation.bundle.items.len(), 3);
    assert_eq!(compilation.report.selected_items, 3);
    assert!(compilation.report.generated_items > compilation.report.selected_items);
    assert_eq!(
        compilation.bundle.budget.items_dropped,
        compilation.report.dropped_items.len() as u32
    );
    assert!(
        compilation
            .report
            .dropped_items
            .iter()
            .all(|item| item.id.starts_with("cand-"))
    );

    for (index, item) in compilation.bundle.items.iter().enumerate() {
        assert_eq!(item.id, format!("ev-{}", index + 1));
        assert_eq!(
            item.token_cost,
            estimate_evidence_item_tokens(item).unwrap()
        );
        for source_ref in item.source_refs.iter() {
            assert!(matches!(
                store.resolve_source_ref(source_ref),
                SourceResolution::Found(_)
            ));
        }
    }

    let tokens_used = compilation
        .bundle
        .items
        .iter()
        .fold(0u32, |tokens, item| tokens + item.token_cost);
    assert_eq!(compilation.bundle.budget.tokens_used, tokens_used);
    compilation.bundle.validate().unwrap();

    for cause in &compilation.suspected_causes {
        for evidence_id in cause.supporting.iter().chain(cause.counter.iter()) {
            assert!(
                evidence_id.starts_with("ev-"),
                "selected suspected-cause links should use final ev-* ids: {cause:#?}"
            );
        }
    }
}

#[test]
fn compile_evidence_token_budget_drops_whole_candidates() {
    let case = fixture_case("deploy-bad-rollout");
    let (mut query, store, derived, candidates) = scored_candidates_for_case(&case);
    let smallest_candidate_cost = candidates
        .iter()
        .map(|candidate| candidate.item.token_cost)
        .min()
        .unwrap();
    query.budget.max_items = 50;
    query.budget.max_tokens = smallest_candidate_cost;

    let compilation = compile_evidence(&query, &store, &derived).unwrap();

    assert_eq!(compilation.bundle.items.len(), 1);
    assert!(compilation.bundle.budget.tokens_used <= query.budget.max_tokens);
    assert_eq!(
        compilation.report.generated_items - compilation.report.selected_items,
        compilation.report.dropped_items.len()
    );
    assert!(
        compilation
            .report
            .dropped_items
            .iter()
            .any(|item| item.reason == "max_tokens")
    );
}

#[test]
fn compile_evidence_counter_requirement_selects_counter_first_when_needed() {
    let case = fixture_case("deploy-bad-rollout");
    let mut query = query_for_case(&case);
    query.budget.max_items = 1;
    query.budget.max_tokens = 10_000;
    query.require_counter_evidence = true;
    query.budget.min_counter_evidence_items = Some(1);
    let mut store = raw_fixture_store(&case);
    let derived = derive_and_insert_context(&case, &mut store).unwrap();

    let compilation = compile_evidence(&query, &store, &derived).unwrap();
    let selected = compilation.bundle.items.first().unwrap();

    assert_eq!(compilation.bundle.items.len(), 1);
    assert!(
        selected.kind == janus::evidence::EvidenceKind::CounterEvidence
            || matches!(
                selected.direction,
                janus::evidence::EvidenceDirection::Weakens
                    | janus::evidence::EvidenceDirection::Contradicts
            ),
        "required counter-evidence should be selected under tight item budget: {selected:#?}"
    );
}

#[test]
fn compile_evidence_counter_requirement_errors_when_budget_cannot_fit_counter() {
    let case = fixture_case("deploy-bad-rollout");
    let mut query = query_for_case(&case);
    query.budget.max_items = 50;
    query.budget.max_tokens = 1;
    query.require_counter_evidence = true;
    query.budget.min_counter_evidence_items = Some(1);
    let mut store = raw_fixture_store(&case);
    let derived = derive_and_insert_context(&case, &mut store).unwrap();

    let error = compile_evidence(&query, &store, &derived).unwrap_err();

    match error {
        EvidenceCompileError::RequirementUnsatisfied { requirement, .. } => {
            assert_eq!(requirement, "counter_evidence");
        }
        other => panic!("expected counter_evidence requirement error, got {other:?}"),
    }
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

fn raw_fixture_store(case: &janus::fixture_validation::FixtureCase) -> HotContextStore {
    let plan = plan_fixture_replay(case).unwrap();
    replay_plan_into_store(&plan).unwrap()
}

fn scored_candidates_for_case(
    case: &janus::fixture_validation::FixtureCase,
) -> (
    EvidenceQuery,
    HotContextStore,
    janus::derived_context::DerivedContext,
    Vec<janus::evidence_compiler::EvidenceCandidate>,
) {
    let query = query_for_case(case);
    let mut store = raw_fixture_store(case);
    let derived = derive_and_insert_context(case, &mut store).unwrap();
    let candidates = generate_evidence_candidates(EvidenceCompilerInput {
        query: &query,
        store: &store,
        derived: &derived,
    })
    .unwrap();

    (query, store, derived, candidates)
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
