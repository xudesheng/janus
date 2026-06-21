use janus::{
    entity_context::{
        EntityAlternative, EntityKind, RelationshipIdentity, RelationshipType, ResolvedEntity,
        ResolvedRelationship, compare_entity_context, insert_derived_entity_context,
        relationship_store_key, resolve_entities, resolve_relationships,
    },
    evidence::{SourceRef, SourceSignal, UnitInterval},
    fixture_simulator::plan_fixture_replay,
    fixture_validation::{FixtureCase, FixtureCorpus, FixtureSelector},
    hot_context_store::{
        HotContextStore, HotIngestEvent, SourceKey, SourceResolution, StoredRecord,
        StoredRecordKind,
    },
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[test]
fn ambiguous_fixture_entities_deserialize_into_phase1_model() {
    let case = fixture_case("ambiguous-entity-resolution");
    let entities: Vec<ResolvedEntity> =
        serde_json::from_value(case.expected["entities"].clone()).unwrap();

    let canary = entity(&entities, "service:payments@canary");
    assert_eq!(canary.kind, EntityKind::Service);
    assert_eq!(canary.confidence, UnitInterval(0.96));
    assert_eq!(
        canary.discriminators["rollout"],
        Value::String("canary".to_string())
    );

    let stable = entity(&entities, "service:payments@stable");
    assert_eq!(
        stable.discriminators["service.instance.id"],
        json!(["payments-7a", "payments-7b"])
    );

    let unresolved = entity(&entities, "service:payments@unresolved");
    assert!(unresolved.unresolved);
    assert_eq!(unresolved.estimated_share, Some(UnitInterval(0.18)));
    assert_eq!(
        unresolved.missing_attributes,
        vec!["service.version", "service.instance.id"]
    );
}

#[test]
fn ambiguous_fixture_relationships_deserialize_into_phase1_model() {
    let case = fixture_case("ambiguous-entity-resolution");
    let relationships: Vec<ResolvedRelationship> =
        serde_json::from_value(case.expected["relationships"].clone()).unwrap();

    assert!(relationships.iter().any(|relationship| {
        relationship.src == "service:payments@stable"
            && relationship.relationship_type == RelationshipType::DeployedAs
            && relationship.dst == "instance:payments-7a"
            && relationship.confidence == UnitInterval(0.98)
    }));
}

#[test]
fn all_current_fixture_entity_and_relationship_gold_deserializes() {
    let corpus = FixtureCorpus::load(repo_root()).unwrap();

    for case in &corpus.cases {
        serde_json::from_value::<Vec<ResolvedEntity>>(case.expected["entities"].clone())
            .unwrap_or_else(|error| {
                panic!(
                    "failed to deserialize expected entities for {}: {error}",
                    case.registry_entry.id
                )
            });
        serde_json::from_value::<Vec<ResolvedRelationship>>(case.expected["relationships"].clone())
            .unwrap_or_else(|error| {
                panic!(
                    "failed to deserialize expected relationships for {}: {error}",
                    case.registry_entry.id
                )
            });
    }

    let deploy_entities: Vec<ResolvedEntity> =
        serde_json::from_value(fixture_case("deploy-bad-rollout").expected["entities"].clone())
            .unwrap();
    assert_eq!(
        entity(&deploy_entities, "db:orders-pg").kind,
        EntityKind::Database
    );

    let traffic_entities: Vec<ResolvedEntity> =
        serde_json::from_value(fixture_case("traffic-shift-hotspot").expected["entities"].clone())
            .unwrap();
    assert_eq!(
        entity(&traffic_entities, "shard:orders-shard-3").kind,
        EntityKind::Partition
    );
}

#[test]
fn relationship_model_serializes_to_fixture_shape_and_store_key() {
    let mut attributes = BTreeMap::new();
    attributes.insert("role".to_string(), json!("primary"));
    let relationship = ResolvedRelationship {
        src: "service:checkout".to_string(),
        relationship_type: RelationshipType::ReadsFrom,
        dst: "db:orders-pg".to_string(),
        confidence: UnitInterval(0.97),
        evidence: vec!["trace:t-0001".to_string()],
        attributes,
    };

    let value = serde_json::to_value(&relationship).unwrap();

    assert_eq!(value["type"], "reads-from");
    assert!(value.get("relationship_type").is_none());
    assert_eq!(
        relationship_store_key(
            "service:checkout",
            RelationshipType::ReadsFrom,
            "db:orders-pg"
        ),
        "relationship:service:checkout|reads-from|db:orders-pg"
    );
}

#[test]
fn resolver_keeps_ambiguous_payments_identities_separate() {
    let case = fixture_case("ambiguous-entity-resolution");
    let store = HotContextStore::load_fixture_case(case).unwrap();
    let entities = resolve_entities(&store);

    let canary = entity(&entities, "service:payments@canary");
    assert_eq!(canary.kind, EntityKind::Service);
    assert_eq!(canary.from, vec!["res:payments-canary"]);
    assert_eq!(canary.confidence, UnitInterval(0.96));
    assert_eq!(canary.discriminators["service.version"], json!("5.0.0-rc1"));
    assert_eq!(
        canary.discriminators["service.instance.id"],
        json!("payments-canary-001")
    );
    assert_eq!(canary.discriminators["rollout"], json!("canary"));
    assert!(canary.alternatives.iter().any(|alternative| {
        alternative.id == "service:payments@stable"
            && alternative.reason.as_deref() == Some("same service.name")
            && alternative.confidence == Some(UnitInterval(0.04))
    }));

    let stable = entity(&entities, "service:payments@stable");
    assert_eq!(
        stable.from,
        vec!["res:payments-stable-a", "res:payments-stable-b"]
    );
    assert_eq!(stable.confidence, UnitInterval(0.95));
    assert_eq!(stable.discriminators["service.version"], json!("4.3.2"));
    assert_eq!(
        stable.discriminators["service.instance.id"],
        json!(["payments-7a", "payments-7b"])
    );

    let unresolved = entity(&entities, "service:payments@unresolved");
    assert!(unresolved.unresolved);
    assert_eq!(unresolved.confidence, UnitInterval(0.40));
    assert_eq!(
        unresolved.missing_attributes,
        vec!["service.version", "service.instance.id"]
    );
    assert_eq!(unresolved.estimated_share, Some(UnitInterval(0.18)));
    assert!(unresolved.alternatives.iter().any(|alternative| {
        alternative.id == "service:payments@canary"
            && alternative.confidence == Some(UnitInterval(0.50))
    }));
    assert!(unresolved.alternatives.iter().any(|alternative| {
        alternative.id == "service:payments@stable"
            && alternative.confidence == Some(UnitInterval(0.50))
    }));

    assert!(
        !entities
            .iter()
            .any(|entity| entity.id == "service:payments"),
        "the resolver must not emit the blended service.name aggregate"
    );
}

#[test]
fn resolver_derives_every_current_fixture_gold_entity_id_and_kind() {
    let corpus = FixtureCorpus::load(repo_root()).unwrap();

    for case in &corpus.cases {
        let store = HotContextStore::load_fixture_case(case).unwrap();
        let derived = resolve_entities(&store);
        let expected: Vec<ResolvedEntity> =
            serde_json::from_value(case.expected["entities"].clone()).unwrap();

        for expected_entity in expected {
            let actual = derived
                .iter()
                .find(|entity| entity.id == expected_entity.id)
                .unwrap_or_else(|| {
                    panic!(
                        "resolver missed expected entity {} for {}",
                        expected_entity.id, case.registry_entry.id
                    )
                });
            assert_eq!(
                actual.kind, expected_entity.kind,
                "resolver produced wrong kind for {} in {}",
                expected_entity.id, case.registry_entry.id
            );
        }
    }
}

#[test]
fn relationship_builder_derives_every_current_fixture_gold_relationship() {
    let corpus = FixtureCorpus::load(repo_root()).unwrap();

    for case in &corpus.cases {
        let store = HotContextStore::load_fixture_case(case).unwrap();
        let entities = resolve_entities(&store);
        let derived = resolve_relationships(&store, &entities);
        let expected: Vec<ResolvedRelationship> =
            serde_json::from_value(case.expected["relationships"].clone()).unwrap();

        for expected_relationship in expected {
            let actual = derived
                .iter()
                .find(|relationship| {
                    relationship.src == expected_relationship.src
                        && relationship.relationship_type == expected_relationship.relationship_type
                        && relationship.dst == expected_relationship.dst
                })
                .unwrap_or_else(|| {
                    panic!(
                        "relationship builder missed expected relationship {} {} {} for {}",
                        expected_relationship.src,
                        expected_relationship.relationship_type.as_str(),
                        expected_relationship.dst,
                        case.registry_entry.id
                    )
                });

            for evidence in &expected_relationship.evidence {
                assert!(
                    actual.evidence.contains(evidence),
                    "relationship {} {} {} in {} is missing expected evidence {}",
                    expected_relationship.src,
                    expected_relationship.relationship_type.as_str(),
                    expected_relationship.dst,
                    case.registry_entry.id,
                    evidence
                );
            }
            for (key, value) in &expected_relationship.attributes {
                assert_eq!(
                    actual.attributes.get(key),
                    Some(value),
                    "relationship {} {} {} in {} has wrong attribute {}",
                    expected_relationship.src,
                    expected_relationship.relationship_type.as_str(),
                    expected_relationship.dst,
                    case.registry_entry.id,
                    key
                );
            }
        }
    }
}

#[test]
fn comparison_accepts_current_fixture_gold_contract() {
    let corpus = FixtureCorpus::load(repo_root()).unwrap();

    for case in &corpus.cases {
        let store = HotContextStore::load_fixture_case(case).unwrap();
        let derived_entities = resolve_entities(&store);
        let derived_relationships = resolve_relationships(&store, &derived_entities);
        let expected_entities: Vec<ResolvedEntity> =
            serde_json::from_value(case.expected["entities"].clone()).unwrap();
        let expected_relationships: Vec<ResolvedRelationship> =
            serde_json::from_value(case.expected["relationships"].clone()).unwrap();

        let comparison = compare_entity_context(
            &expected_entities,
            &expected_relationships,
            &derived_entities,
            &derived_relationships,
        );

        assert!(
            !comparison.has_expected_mismatches(),
            "derived entity context mismatched expected gold for {}:\n{comparison:#?}",
            case.registry_entry.id
        );

        let gold_entity_ids = expected_entities
            .iter()
            .map(|entity| entity.id.as_str())
            .collect::<BTreeSet<_>>();
        let extra_gold_relationships = comparison
            .extra_relationships
            .iter()
            .filter(|relationship| {
                gold_entity_ids.contains(relationship.src.as_str())
                    && gold_entity_ids.contains(relationship.dst.as_str())
            })
            .cloned()
            .collect::<BTreeSet<_>>();
        let reviewed_extras =
            reviewed_extra_relationships_between_gold_entities(&case.registry_entry.id);
        let unreviewed_extras = extra_gold_relationships
            .difference(&reviewed_extras)
            .collect::<Vec<_>>();
        assert!(
            unreviewed_extras.is_empty(),
            "derived entity context produced unreviewed extra relationships between gold entities for {}:\n{unreviewed_extras:#?}",
            case.registry_entry.id
        );
    }
}

#[test]
fn comparison_reports_extra_entities_and_relationships_between_gold_entities() {
    let expected_entities = vec![
        test_entity("service:a", EntityKind::Service, 0.99),
        test_entity("service:b", EntityKind::Service, 0.99),
    ];
    let expected_relationships = Vec::new();
    let mut derived_entities = expected_entities.clone();
    derived_entities.push(test_entity("instance:a-1", EntityKind::Instance, 0.98));
    let derived_relationships = vec![test_relationship(
        "service:a",
        RelationshipType::Calls,
        "service:b",
        0.98,
    )];

    let comparison = compare_entity_context(
        &expected_entities,
        &expected_relationships,
        &derived_entities,
        &derived_relationships,
    );

    assert!(!comparison.has_expected_mismatches());
    assert_eq!(comparison.extra_entities, vec!["instance:a-1"]);
    assert_eq!(
        comparison.extra_relationships,
        vec![RelationshipIdentity {
            src: "service:a".to_string(),
            relationship_type: RelationshipType::Calls,
            dst: "service:b".to_string(),
        }]
    );
}

#[test]
fn comparison_checks_confidence_discriminators_and_alternatives() {
    let mut expected = test_entity("service:payments@stable", EntityKind::Service, 0.95);
    expected.discriminators.insert(
        "service.instance.id".to_string(),
        json!(["payments-7a", "payments-7b"]),
    );
    expected
        .discriminators
        .insert("rollout".to_string(), json!("stable"));
    expected.alternatives.push(EntityAlternative {
        id: "service:payments@canary".to_string(),
        reason: Some("same service.name".to_string()),
        confidence: Some(UnitInterval(0.05)),
    });

    let mut actual = expected.clone();
    actual.confidence = UnitInterval(0.70);
    actual.discriminators.insert(
        "service.instance.id".to_string(),
        json!(["payments-7b", "payments-7a"]),
    );
    actual
        .discriminators
        .insert("rollout".to_string(), json!("canary"));
    actual.alternatives[0].confidence = Some(UnitInterval(0.30));

    let comparison = compare_entity_context(&[expected], &[], &[actual], &[]);

    assert_eq!(comparison.entity_confidence_mismatches.len(), 1);
    assert_eq!(comparison.entity_discriminator_mismatches.len(), 1);
    assert_eq!(
        comparison.entity_discriminator_mismatches[0].field,
        "discriminators.rollout"
    );
    assert_eq!(comparison.entity_alternative_mismatches.len(), 1);
    assert_eq!(
        comparison.entity_alternative_mismatches[0].field,
        "alternatives.service:payments@canary.confidence"
    );
}

#[test]
fn comparison_checks_unresolved_markers_and_relationship_details() {
    let mut expected_entity = test_entity("service:payments@unresolved", EntityKind::Service, 0.40);
    expected_entity.unresolved = true;
    expected_entity.missing_attributes = vec![
        "service.version".to_string(),
        "service.instance.id".to_string(),
    ];
    expected_entity.estimated_share = Some(UnitInterval(0.18));
    let mut actual_entity = expected_entity.clone();
    actual_entity.unresolved = false;
    actual_entity.missing_attributes = vec!["service.version".to_string()];
    actual_entity.estimated_share = None;

    let mut expected_relationship = test_relationship(
        "service:checkout",
        RelationshipType::ReadsFrom,
        "db:orders-pg",
        0.97,
    );
    expected_relationship.evidence = vec!["trace:t-0001".to_string()];
    expected_relationship
        .attributes
        .insert("role".to_string(), json!("primary"));
    let mut actual_relationship = expected_relationship.clone();
    actual_relationship.confidence = UnitInterval(0.80);
    actual_relationship.evidence.clear();
    actual_relationship
        .attributes
        .insert("role".to_string(), json!("replica"));

    let comparison = compare_entity_context(
        &[expected_entity],
        &[expected_relationship],
        &[actual_entity],
        &[actual_relationship],
    );

    assert_eq!(comparison.entity_unresolved_mismatches.len(), 1);
    assert_eq!(comparison.entity_missing_attribute_mismatches.len(), 1);
    assert_eq!(comparison.entity_estimated_share_mismatches.len(), 1);
    assert_eq!(comparison.relationship_confidence_mismatches.len(), 1);
    assert_eq!(comparison.missing_relationship_evidence.len(), 1);
    assert_eq!(comparison.relationship_attribute_mismatches.len(), 1);
}

#[test]
fn relationship_builder_preserves_retry_and_cache_fallback_attributes() {
    let retry_store =
        HotContextStore::load_fixture_case(fixture_case("retry-storm-amplification")).unwrap();
    let retry_entities = resolve_entities(&retry_store);
    let retry_relationships = resolve_relationships(&retry_store, &retry_entities);
    let retry = relationship(
        &retry_relationships,
        "service:checkout",
        RelationshipType::Retries,
        "service:payment-svc",
    );
    assert_eq!(retry.evidence, vec!["trace:t-2001"]);
    assert_eq!(retry.attributes["max_attempts"], json!(5));
    assert_eq!(retry.attributes["backoff"], json!("none"));

    let cache_store =
        HotContextStore::load_fixture_case(fixture_case("coincidental-deploy-trap")).unwrap();
    let cache_entities = resolve_entities(&cache_store);
    let cache_relationships = resolve_relationships(&cache_store, &cache_entities);
    let fallback = relationship(
        &cache_relationships,
        "service:search-api",
        RelationshipType::ReadsFrom,
        "db:catalog-pg",
    );
    assert_eq!(fallback.evidence, vec!["trace:t-3001"]);
    assert_eq!(fallback.attributes["role"], json!("cache-miss-fallback"));
}

#[test]
fn derived_entity_context_records_resolve_through_hot_store_reference_boundary() {
    let mut store = source_only_store(fixture_case("deploy-bad-rollout"));
    let entities = resolve_entities(&store);
    let relationships = resolve_relationships(&store, &entities);

    insert_derived_entity_context(&mut store, &entities, &relationships).unwrap();

    let entity_record =
        found(store.resolve_source_ref(&source_ref(SourceSignal::Entity, "service:checkout")));
    assert_eq!(entity_record.kind, StoredRecordKind::Entity);
    assert_eq!(entity_record.key.as_str(), "service:checkout");
    assert_eq!(entity_record.entities, vec!["service:checkout"]);
    assert_eq!(entity_record.payload["id"], "service:checkout");
    assert_eq!(entity_record.payload["kind"], "service");

    let relationship_key = relationship_store_key(
        "service:api-gateway",
        RelationshipType::Calls,
        "service:checkout",
    );
    let relationship_record =
        found(store.resolve_source_ref(&source_ref(SourceSignal::Relationship, &relationship_key)));
    assert_eq!(relationship_record.kind, StoredRecordKind::Relationship);
    assert_eq!(relationship_record.key.as_str(), relationship_key);
    assert_eq!(
        relationship_record.entities,
        vec!["service:api-gateway", "service:checkout"]
    );
    assert_eq!(relationship_record.payload["src"], "service:api-gateway");
    assert_eq!(relationship_record.payload["type"], "calls");
    assert_eq!(relationship_record.payload["dst"], "service:checkout");
}

#[test]
fn inserted_derived_records_are_not_raw_resolver_inputs() {
    let mut store = source_only_store(fixture_case("ambiguous-entity-resolution"));
    let before_count = store.raw_source_records().count();
    let entities = resolve_entities(&store);
    let relationships = resolve_relationships(&store, &entities);

    insert_derived_entity_context(&mut store, &entities, &relationships).unwrap();

    assert_eq!(
        store.raw_source_records().count(),
        before_count,
        "derived entity context records must not widen the raw-source resolver boundary"
    );
    assert!(
        store
            .records()
            .iter()
            .any(|record| record.kind == StoredRecordKind::Entity)
    );
    assert!(
        store
            .records()
            .iter()
            .any(|record| record.kind == StoredRecordKind::Relationship)
    );
}

#[test]
fn relationship_builder_ignores_derived_gold_relationship_records() {
    let mut store = HotContextStore::new();
    store
        .insert_record(StoredRecord {
            key: SourceKey::new("relationship:gold-only"),
            kind: StoredRecordKind::Relationship,
            time_window: None,
            entities: vec!["service:source".to_string(), "service:target".to_string()],
            payload: json!({
                "src": "service:source",
                "type": "calls",
                "dst": "service:target",
                "confidence": 0.99
            }),
        })
        .unwrap();

    assert!(
        resolve_relationships(&store, &[]).is_empty(),
        "relationship builder must derive from raw source records, not expected/gold relationship records"
    );
}

#[test]
fn resolver_ignores_derived_gold_entity_records() {
    let mut store = HotContextStore::new();
    store
        .insert_record(StoredRecord {
            key: SourceKey::new("service:gold-only"),
            kind: StoredRecordKind::Entity,
            time_window: None,
            entities: vec!["service:gold-only".to_string()],
            payload: json!({
                "id": "service:gold-only",
                "kind": "service",
                "from": ["res:missing"],
                "confidence": 0.99
            }),
        })
        .unwrap();

    assert!(
        resolve_entities(&store).is_empty(),
        "resolver must derive from raw source records, not expected/gold entity records"
    );
}

#[test]
fn raw_source_records_exclude_expected_derived_records() {
    let case = fixture_case("ambiguous-entity-resolution");
    let store = HotContextStore::load_fixture_case(case).unwrap();

    assert!(
        store
            .records()
            .iter()
            .any(|record| record.kind == StoredRecordKind::Entity),
        "fixture store should include expected gold entity records"
    );
    assert!(
        store
            .records()
            .iter()
            .any(|record| record.kind == StoredRecordKind::AnomalyWindow),
        "fixture store should include expected derived artifact records"
    );

    let raw_records: Vec<_> = store.raw_source_records().collect();

    assert!(!raw_records.is_empty());
    assert!(raw_records.len() < store.records().len());
    assert!(
        raw_records.iter().all(|record| record.kind.is_raw_source()),
        "raw-source read boundary must not expose expected derived records"
    );
}

#[test]
fn stored_record_kind_marks_only_raw_sources_as_raw() {
    for kind in [
        StoredRecordKind::Resource,
        StoredRecordKind::Trace,
        StoredRecordKind::Span,
        StoredRecordKind::MetricSeries,
        StoredRecordKind::Log,
        StoredRecordKind::Change,
        StoredRecordKind::PriorIncident,
        StoredRecordKind::TelemetryGap,
    ] {
        assert!(kind.is_raw_source(), "{kind} should be raw");
    }

    for kind in [
        StoredRecordKind::Entity,
        StoredRecordKind::Relationship,
        StoredRecordKind::AnomalyWindow,
        StoredRecordKind::LogPattern,
        StoredRecordKind::EvidenceItem,
        StoredRecordKind::TimelineEvent,
        StoredRecordKind::SuspectedCause,
        StoredRecordKind::NextCheck,
        StoredRecordKind::EntityContext,
        StoredRecordKind::RelatedAnomaly,
        StoredRecordKind::WindowComparison,
    ] {
        assert!(!kind.is_raw_source(), "{kind} should be derived");
    }
}

fn fixture_case(id: &str) -> &'static FixtureCase {
    let corpus = Box::leak(Box::new(FixtureCorpus::load(repo_root()).unwrap()));
    let selected = corpus.select(&FixtureSelector {
        fixture_id: Some(id.to_string()),
        ..FixtureSelector::default()
    });

    selected.into_iter().next().expect("fixture should exist")
}

fn entity<'a>(entities: &'a [ResolvedEntity], id: &str) -> &'a ResolvedEntity {
    entities
        .iter()
        .find(|entity| entity.id == id)
        .expect("entity should exist")
}

fn relationship<'a>(
    relationships: &'a [ResolvedRelationship],
    src: &str,
    relationship_type: RelationshipType,
    dst: &str,
) -> &'a ResolvedRelationship {
    relationships
        .iter()
        .find(|relationship| {
            relationship.src == src
                && relationship.relationship_type == relationship_type
                && relationship.dst == dst
        })
        .expect("relationship should exist")
}

fn source_only_store(case: &FixtureCase) -> HotContextStore {
    let plan = plan_fixture_replay(case).unwrap();
    let mut store = HotContextStore::new();

    for event in plan.events() {
        let ingest_event = HotIngestEvent::try_from(event).unwrap();
        store.ingest(ingest_event).unwrap();
    }

    store
}

fn source_ref(signal: SourceSignal, raw_ref: &str) -> SourceRef {
    SourceRef {
        signal,
        r#ref: raw_ref.to_string(),
    }
}

fn reviewed_extra_relationships_between_gold_entities(
    fixture_id: &str,
) -> BTreeSet<RelationshipIdentity> {
    let mut relationships = BTreeSet::new();

    if fixture_id == "traffic-shift-hotspot" {
        relationships.insert(RelationshipIdentity {
            src: "service:orders".to_string(),
            relationship_type: RelationshipType::FansOutTo,
            dst: "shard:orders-shard-1".to_string(),
        });
    }

    relationships
}

fn found(resolution: SourceResolution<'_>) -> &StoredRecord {
    match resolution {
        SourceResolution::Found(record) => record,
        other => panic!("expected found record, got {other:?}"),
    }
}

fn test_entity(id: &str, kind: EntityKind, confidence: f64) -> ResolvedEntity {
    ResolvedEntity {
        id: id.to_string(),
        kind,
        from: Vec::new(),
        confidence: UnitInterval(confidence),
        discriminators: BTreeMap::new(),
        alternatives: Vec::new(),
        unresolved: false,
        missing_attributes: Vec::new(),
        estimated_share: None,
    }
}

fn test_relationship(
    src: &str,
    relationship_type: RelationshipType,
    dst: &str,
    confidence: f64,
) -> ResolvedRelationship {
    ResolvedRelationship {
        src: src.to_string(),
        relationship_type,
        dst: dst.to_string(),
        confidence: UnitInterval(confidence),
        evidence: Vec::new(),
        attributes: BTreeMap::new(),
    }
}

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
