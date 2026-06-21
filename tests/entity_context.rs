use janus::{
    entity_context::{
        EntityKind, RelationshipType, ResolvedEntity, ResolvedRelationship, relationship_store_key,
    },
    evidence::UnitInterval,
    fixture_validation::{FixtureCase, FixtureCorpus, FixtureSelector},
    hot_context_store::{HotContextStore, StoredRecordKind},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

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

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
