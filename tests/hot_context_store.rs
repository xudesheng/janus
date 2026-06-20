use janus::{
    evidence::{EvidenceBundle, SourceRef, SourceSignal, TimeWindow},
    fixture_validation::{FixtureCase, FixtureCorpus, FixtureSelector},
    hot_context_store::{
        HotContextStore, HotStoreError, SourceKey, SourceQuery, SourceResolution, StoredRecord,
        StoredRecordKind,
    },
};
use serde_json::{Value, json};
use std::path::Path;

#[test]
fn current_fixture_inputs_load_into_hot_context_store() {
    let corpus = FixtureCorpus::load(repo_root()).unwrap();

    for case in &corpus.cases {
        let store = HotContextStore::load_fixture_case(case).unwrap_or_else(|error| {
            panic!(
                "failed to load fixture {} into hot store: {error}",
                case.registry_entry.id
            )
        });

        assert!(
            store.record_count() > 0,
            "fixture {} should load at least one record",
            case.registry_entry.id
        );
    }
}

#[test]
fn every_current_evidence_source_ref_resolves_to_concrete_record() {
    let corpus = FixtureCorpus::load(repo_root()).unwrap();

    for case in &corpus.cases {
        let store = HotContextStore::load_fixture_case(case).unwrap();
        let bundle = evidence_bundle(case);

        for item in &bundle.items {
            for source_ref in item.source_refs.iter() {
                match store.resolve_source_ref(source_ref) {
                    SourceResolution::Found(record) => {
                        assert!(
                            !record.payload.is_null(),
                            "resolved {} in {} to a null payload",
                            source_ref.r#ref,
                            case.registry_entry.id
                        );
                    }
                    other => panic!(
                        "failed to resolve {:?} in fixture {}: {:?}",
                        source_ref, case.registry_entry.id, other
                    ),
                }
            }
        }
    }
}

#[test]
fn resolves_raw_and_derived_records_to_payloads() {
    let case = fixture_case("deploy-bad-rollout");
    let store = HotContextStore::load_fixture_case(case).unwrap();

    let span = found(store.resolve_scalar_ref("t-0001/s-3"));
    assert_eq!(span.kind, StoredRecordKind::Span);
    assert_eq!(span.payload["span_id"], "s-3");

    let metric = found(store.resolve_source_ref(&source_ref(
        SourceSignal::Metric,
        "http.server.error_rate@service:checkout",
    )));
    assert_eq!(metric.kind, StoredRecordKind::MetricSeries);
    assert_eq!(metric.payload["name"], "http.server.error_rate");

    let change = found(store.resolve_source_ref(&source_ref(
        SourceSignal::Change,
        "change:deploy-checkout-v2",
    )));
    assert_eq!(change.kind, StoredRecordKind::Change);
    assert_eq!(change.payload["kind"], "deploy");

    let log_pattern =
        found(store.resolve_source_ref(&source_ref(SourceSignal::LogPattern, "lp-1")));
    assert_eq!(log_pattern.kind, StoredRecordKind::LogPattern);
    assert_eq!(log_pattern.payload["id"], "lp-1");
}

#[test]
fn signal_category_mismatch_is_a_distinct_resolution_outcome() {
    let case = fixture_case("deploy-bad-rollout");
    let store = HotContextStore::load_fixture_case(case).unwrap();

    match store.resolve_source_ref(&source_ref(SourceSignal::Log, "lp-1")) {
        SourceResolution::SignalMismatch {
            raw_ref,
            signal,
            candidates,
        } => {
            assert_eq!(raw_ref, "lp-1");
            assert_eq!(signal, SourceSignal::Log);
            assert_eq!(candidates.len(), 1);
            assert_eq!(candidates[0].kind, StoredRecordKind::LogPattern);
        }
        other => panic!("expected signal mismatch, got {other:?}"),
    }
}

#[test]
fn unsupported_profile_and_external_refs_are_distinct_outcomes() {
    let store = HotContextStore::new();

    assert!(matches!(
        store.resolve_source_ref(&source_ref(SourceSignal::Profile, "profile-1")),
        SourceResolution::Unsupported {
            signal: SourceSignal::Profile,
            ..
        }
    ));
    assert!(matches!(
        store.resolve_source_ref(&source_ref(SourceSignal::External, "external-1")),
        SourceResolution::Unsupported {
            signal: SourceSignal::External,
            ..
        }
    ));
}

#[test]
fn duplicate_primary_key_within_kind_fails() {
    let mut store = HotContextStore::new();
    store
        .insert_record(test_record(StoredRecordKind::Log, "same"))
        .unwrap();

    let error = store
        .insert_record(test_record(StoredRecordKind::Log, "same"))
        .unwrap_err();

    assert!(matches!(
        error,
        HotStoreError::DuplicatePrimaryKey {
            kind: StoredRecordKind::Log,
            ..
        }
    ));
}

#[test]
fn same_raw_key_across_kinds_disambiguates_by_signal_and_ambiguous_scalar() {
    let mut store = HotContextStore::new();
    store
        .insert_record(test_record(StoredRecordKind::Log, "same"))
        .unwrap();
    store
        .insert_record(test_record(StoredRecordKind::Change, "same"))
        .unwrap();

    let log = found(store.resolve_source_ref(&source_ref(SourceSignal::Log, "same")));
    assert_eq!(log.kind, StoredRecordKind::Log);

    let change = found(store.resolve_source_ref(&source_ref(SourceSignal::Change, "same")));
    assert_eq!(change.kind, StoredRecordKind::Change);

    match store.resolve_scalar_ref("same") {
        SourceResolution::Ambiguous {
            raw_ref,
            candidates,
        } => {
            assert_eq!(raw_ref, "same");
            assert_eq!(candidates.len(), 2);
        }
        other => panic!("expected scalar ambiguity, got {other:?}"),
    }
}

#[test]
fn time_window_selector_returns_overlapping_records_in_stable_order() {
    let case = fixture_case("deploy-bad-rollout");
    let store = HotContextStore::load_fixture_case(case).unwrap();

    let selected = store.select(SourceQuery {
        time_window: Some(TimeWindow {
            start: "2026-06-01T14:03:00Z".to_string(),
            end: "2026-06-01T14:04:00Z".to_string(),
        }),
        ..SourceQuery::default()
    });
    let keys = selected
        .iter()
        .map(|record| record.key.as_str())
        .collect::<Vec<_>>();

    assert!(keys.contains(&"http.server.error_rate@service:checkout"));
    assert!(keys.contains(&"log-1"));
    assert!(keys.contains(&"change:deploy-checkout-v2"));
}

#[test]
fn entity_and_kind_selectors_filter_without_reordering_fixture_records() {
    let case = fixture_case("deploy-bad-rollout");
    let store = HotContextStore::load_fixture_case(case).unwrap();

    let resources = store.select(SourceQuery {
        kinds: vec![StoredRecordKind::Resource],
        ..SourceQuery::default()
    });
    let resource_keys = resources
        .iter()
        .map(|record| record.key.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        resource_keys,
        vec!["res:api-gateway", "res:checkout-v2", "res:orders-pg"]
    );

    let checkout_logs = store.select(SourceQuery {
        entities: vec!["service:checkout".to_string()],
        kinds: vec![StoredRecordKind::Log],
        ..SourceQuery::default()
    });
    let log_keys = checkout_logs
        .iter()
        .map(|record| record.key.as_str())
        .collect::<Vec<_>>();

    assert_eq!(log_keys, vec!["log-1", "log-2", "log-3", "log-4"]);
}

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn fixture_case(id: &str) -> &'static FixtureCase {
    let corpus = Box::leak(Box::new(FixtureCorpus::load(repo_root()).unwrap()));
    let selected = corpus.select(&FixtureSelector {
        fixture_id: Some(id.to_string()),
        ..FixtureSelector::default()
    });

    selected.into_iter().next().expect("fixture should exist")
}

fn evidence_bundle(case: &FixtureCase) -> EvidenceBundle {
    serde_json::from_value(case.expected["evidence_bundle"].clone()).unwrap()
}

fn source_ref(signal: SourceSignal, raw_ref: &str) -> SourceRef {
    SourceRef {
        signal,
        r#ref: raw_ref.to_string(),
    }
}

fn found(resolution: SourceResolution<'_>) -> &StoredRecord {
    match resolution {
        SourceResolution::Found(record) => record,
        other => panic!("expected found record, got {other:?}"),
    }
}

fn test_record(kind: StoredRecordKind, key: &str) -> StoredRecord {
    StoredRecord {
        key: SourceKey::new(key),
        kind,
        time_window: None,
        entities: Vec::new(),
        payload: payload(key),
    }
}

fn payload(id: &str) -> Value {
    json!({ "id": id })
}
