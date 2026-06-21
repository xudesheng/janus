use crate::{
    evidence::UnitInterval,
    hot_context_store::{HotContextStore, StoredRecord, StoredRecordKind},
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedEntity {
    pub id: String,
    pub kind: EntityKind,
    pub from: Vec<String>,
    pub confidence: UnitInterval,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub discriminators: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<EntityAlternative>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub unresolved: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_attributes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_share: Option<UnitInterval>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityAlternative {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<UnitInterval>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedRelationship {
    pub src: String,
    #[serde(rename = "type")]
    pub relationship_type: RelationshipType,
    pub dst: String,
    pub confidence: UnitInterval,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, Value>,
}

pub const ENTITY_CONTEXT_CONFIDENCE_TOLERANCE: f64 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntityKind {
    Service,
    Route,
    Instance,
    Pod,
    Database,
    Queue,
    Cache,
    ExternalApi,
    Tenant,
    Infra,
    Deployment,
    Host,
    Container,
    Partition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RelationshipType {
    Calls,
    ReadsFrom,
    WritesTo,
    DependsOn,
    RunsOn,
    DeployedAs,
    Emits,
    Retries,
    FansOutTo,
    SharesResourceWith,
}

impl RelationshipType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Calls => "calls",
            Self::ReadsFrom => "reads-from",
            Self::WritesTo => "writes-to",
            Self::DependsOn => "depends-on",
            Self::RunsOn => "runs-on",
            Self::DeployedAs => "deployed-as",
            Self::Emits => "emits",
            Self::Retries => "retries",
            Self::FansOutTo => "fans-out-to",
            Self::SharesResourceWith => "shares-resource-with",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EntityContextComparison {
    pub missing_entities: Vec<String>,
    pub extra_entities: Vec<String>,
    pub entity_kind_mismatches: Vec<EntityKindMismatch>,
    pub entity_confidence_mismatches: Vec<EntityConfidenceMismatch>,
    pub entity_source_mismatches: Vec<EntityFieldMismatch>,
    pub entity_discriminator_mismatches: Vec<EntityFieldMismatch>,
    pub entity_alternative_mismatches: Vec<EntityFieldMismatch>,
    pub entity_unresolved_mismatches: Vec<EntityFieldMismatch>,
    pub entity_missing_attribute_mismatches: Vec<EntityFieldMismatch>,
    pub entity_estimated_share_mismatches: Vec<EntityConfidenceMismatch>,
    pub missing_relationships: Vec<RelationshipIdentity>,
    pub extra_relationships: Vec<RelationshipIdentity>,
    pub relationship_confidence_mismatches: Vec<RelationshipConfidenceMismatch>,
    pub missing_relationship_evidence: Vec<RelationshipFieldMismatch>,
    pub relationship_attribute_mismatches: Vec<RelationshipFieldMismatch>,
}

impl EntityContextComparison {
    pub fn has_expected_mismatches(&self) -> bool {
        !self.missing_entities.is_empty()
            || !self.entity_kind_mismatches.is_empty()
            || !self.entity_confidence_mismatches.is_empty()
            || !self.entity_source_mismatches.is_empty()
            || !self.entity_discriminator_mismatches.is_empty()
            || !self.entity_alternative_mismatches.is_empty()
            || !self.entity_unresolved_mismatches.is_empty()
            || !self.entity_missing_attribute_mismatches.is_empty()
            || !self.entity_estimated_share_mismatches.is_empty()
            || !self.missing_relationships.is_empty()
            || !self.relationship_confidence_mismatches.is_empty()
            || !self.missing_relationship_evidence.is_empty()
            || !self.relationship_attribute_mismatches.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityKindMismatch {
    pub id: String,
    pub expected: EntityKind,
    pub actual: EntityKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityConfidenceMismatch {
    pub id: String,
    pub field: String,
    pub expected: Option<UnitInterval>,
    pub actual: Option<UnitInterval>,
    pub tolerance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityFieldMismatch {
    pub id: String,
    pub field: String,
    pub expected: Value,
    pub actual: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RelationshipIdentity {
    pub src: String,
    pub relationship_type: RelationshipType,
    pub dst: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelationshipConfidenceMismatch {
    pub relationship: RelationshipIdentity,
    pub expected: UnitInterval,
    pub actual: UnitInterval,
    pub tolerance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelationshipFieldMismatch {
    pub relationship: RelationshipIdentity,
    pub field: String,
    pub expected: Value,
    pub actual: Option<Value>,
}

pub fn resolve_entities(store: &HotContextStore) -> Vec<ResolvedEntity> {
    let resources = resource_identities(store);
    let ambiguous_services = ambiguous_service_names(&resources);
    let ambiguous_unresolved_shares =
        ambiguous_unresolved_share_estimates(store, &resources, &ambiguous_services);
    let mut entities = BTreeMap::new();

    insert_resource_entities(
        &mut entities,
        &resources,
        &ambiguous_services,
        &ambiguous_unresolved_shares,
    );

    for record in store.raw_source_records() {
        match record.kind {
            StoredRecordKind::Span => insert_span_entities(&mut entities, record, &resources),
            StoredRecordKind::MetricSeries | StoredRecordKind::Log | StoredRecordKind::Change => {
                insert_record_entity_hint(&mut entities, record, &resources, &ambiguous_services);
            }
            StoredRecordKind::TelemetryGap => insert_telemetry_gap_entities(
                &mut entities,
                record,
                &resources,
                &ambiguous_services,
            ),
            StoredRecordKind::Resource
            | StoredRecordKind::Trace
            | StoredRecordKind::PriorIncident => {}
            _ => {}
        }
    }

    entities.into_values().collect()
}

pub fn resolve_relationships(
    store: &HotContextStore,
    entities: &[ResolvedEntity],
) -> Vec<ResolvedRelationship> {
    let resources = resource_identities(store);
    let ambiguous_services = ambiguous_service_names(&resources);
    let entity_confidences = entity_confidence_index(entities);
    let spans = span_contexts(store, &resources, &ambiguous_services);
    let mut relationships = BTreeMap::new();

    insert_runtime_relationships(
        &mut relationships,
        &resources,
        &ambiguous_services,
        &entity_confidences,
    );
    insert_span_relationships(
        &mut relationships,
        &spans,
        &resources,
        &ambiguous_services,
        &entity_confidences,
    );
    insert_resource_name_relationships(
        &mut relationships,
        &resources,
        &ambiguous_services,
        &entity_confidences,
    );
    insert_change_inferred_relationships(
        &mut relationships,
        store,
        &resources,
        &ambiguous_services,
        &entity_confidences,
    );
    insert_prior_inferred_relationships(&mut relationships, store, &resources, &entity_confidences);
    insert_shared_resource_relationships(&mut relationships, &entity_confidences);

    relationships.into_values().collect()
}

pub fn relationship_store_key(src: &str, relationship_type: RelationshipType, dst: &str) -> String {
    format!("relationship:{src}|{}|{dst}", relationship_type.as_str())
}

pub fn compare_entity_context(
    expected_entities: &[ResolvedEntity],
    expected_relationships: &[ResolvedRelationship],
    derived_entities: &[ResolvedEntity],
    derived_relationships: &[ResolvedRelationship],
) -> EntityContextComparison {
    let expected_entity_by_id = expected_entities
        .iter()
        .map(|entity| (entity.id.as_str(), entity))
        .collect::<BTreeMap<_, _>>();
    let derived_entity_by_id = derived_entities
        .iter()
        .map(|entity| (entity.id.as_str(), entity))
        .collect::<BTreeMap<_, _>>();
    let expected_relationship_by_id = expected_relationships
        .iter()
        .map(|relationship| (RelationshipIdentity::from(relationship), relationship))
        .collect::<BTreeMap<_, _>>();
    let derived_relationship_by_id = derived_relationships
        .iter()
        .map(|relationship| (RelationshipIdentity::from(relationship), relationship))
        .collect::<BTreeMap<_, _>>();

    let mut comparison = EntityContextComparison::default();

    for expected in expected_entities {
        let Some(actual) = derived_entity_by_id.get(expected.id.as_str()) else {
            comparison.missing_entities.push(expected.id.clone());
            continue;
        };

        compare_entity(expected, actual, &mut comparison);
    }

    comparison.extra_entities = derived_entity_by_id
        .keys()
        .filter(|id| !expected_entity_by_id.contains_key(**id))
        .map(|id| (*id).to_string())
        .collect();

    for expected in expected_relationships {
        let identity = RelationshipIdentity::from(expected);
        let Some(actual) = derived_relationship_by_id.get(&identity) else {
            comparison.missing_relationships.push(identity);
            continue;
        };

        compare_relationship(expected, actual, &identity, &mut comparison);
    }

    comparison.extra_relationships = derived_relationship_by_id
        .keys()
        .filter(|identity| !expected_relationship_by_id.contains_key(*identity))
        .cloned()
        .collect();

    comparison
}

impl From<&ResolvedRelationship> for RelationshipIdentity {
    fn from(relationship: &ResolvedRelationship) -> Self {
        Self {
            src: relationship.src.clone(),
            relationship_type: relationship.relationship_type,
            dst: relationship.dst.clone(),
        }
    }
}

fn compare_entity(
    expected: &ResolvedEntity,
    actual: &ResolvedEntity,
    comparison: &mut EntityContextComparison,
) {
    if actual.kind != expected.kind {
        comparison.entity_kind_mismatches.push(EntityKindMismatch {
            id: expected.id.clone(),
            expected: expected.kind,
            actual: actual.kind,
        });
    }

    if !within_confidence_tolerance(expected.confidence, actual.confidence) {
        comparison
            .entity_confidence_mismatches
            .push(EntityConfidenceMismatch {
                id: expected.id.clone(),
                field: "confidence".to_string(),
                expected: Some(expected.confidence),
                actual: Some(actual.confidence),
                tolerance: ENTITY_CONTEXT_CONFIDENCE_TOLERANCE,
            });
    }

    if !is_subset(&expected.from, &actual.from) {
        comparison
            .entity_source_mismatches
            .push(EntityFieldMismatch {
                id: expected.id.clone(),
                field: "from".to_string(),
                expected: string_array_value(&expected.from),
                actual: Some(string_array_value(&actual.from)),
            });
    }

    for (key, expected_value) in &expected.discriminators {
        match actual.discriminators.get(key) {
            Some(actual_value) if json_values_equivalent(expected_value, actual_value) => {}
            actual_value => comparison
                .entity_discriminator_mismatches
                .push(EntityFieldMismatch {
                    id: expected.id.clone(),
                    field: format!("discriminators.{key}"),
                    expected: expected_value.clone(),
                    actual: actual_value.cloned(),
                }),
        }
    }

    for expected_alternative in &expected.alternatives {
        match actual
            .alternatives
            .iter()
            .find(|alternative| alternative.id == expected_alternative.id)
        {
            Some(actual_alternative) => compare_alternative(
                &expected.id,
                expected_alternative,
                actual_alternative,
                comparison,
            ),
            None => comparison
                .entity_alternative_mismatches
                .push(EntityFieldMismatch {
                    id: expected.id.clone(),
                    field: format!("alternatives.{}", expected_alternative.id),
                    expected: alternative_value(expected_alternative),
                    actual: None,
                }),
        }
    }

    if expected.unresolved != actual.unresolved {
        comparison
            .entity_unresolved_mismatches
            .push(EntityFieldMismatch {
                id: expected.id.clone(),
                field: "unresolved".to_string(),
                expected: Value::Bool(expected.unresolved),
                actual: Some(Value::Bool(actual.unresolved)),
            });
    }

    if string_set(&expected.missing_attributes) != string_set(&actual.missing_attributes) {
        comparison
            .entity_missing_attribute_mismatches
            .push(EntityFieldMismatch {
                id: expected.id.clone(),
                field: "missing_attributes".to_string(),
                expected: string_array_value(&expected.missing_attributes),
                actual: Some(string_array_value(&actual.missing_attributes)),
            });
    }

    match (expected.estimated_share, actual.estimated_share) {
        (Some(expected_share), Some(actual_share)) => {
            if !within_confidence_tolerance(expected_share, actual_share) {
                comparison
                    .entity_estimated_share_mismatches
                    .push(EntityConfidenceMismatch {
                        id: expected.id.clone(),
                        field: "estimated_share".to_string(),
                        expected: Some(expected_share),
                        actual: Some(actual_share),
                        tolerance: ENTITY_CONTEXT_CONFIDENCE_TOLERANCE,
                    });
            }
        }
        (Some(expected_share), None) => {
            comparison
                .entity_estimated_share_mismatches
                .push(EntityConfidenceMismatch {
                    id: expected.id.clone(),
                    field: "estimated_share".to_string(),
                    expected: Some(expected_share),
                    actual: None,
                    tolerance: ENTITY_CONTEXT_CONFIDENCE_TOLERANCE,
                })
        }
        (None, Some(actual_share)) => {
            comparison
                .entity_estimated_share_mismatches
                .push(EntityConfidenceMismatch {
                    id: expected.id.clone(),
                    field: "estimated_share".to_string(),
                    expected: None,
                    actual: Some(actual_share),
                    tolerance: ENTITY_CONTEXT_CONFIDENCE_TOLERANCE,
                })
        }
        (None, None) => {}
    }
}

fn compare_alternative(
    entity_id: &str,
    expected: &EntityAlternative,
    actual: &EntityAlternative,
    comparison: &mut EntityContextComparison,
) {
    if expected.reason != actual.reason {
        comparison
            .entity_alternative_mismatches
            .push(EntityFieldMismatch {
                id: entity_id.to_string(),
                field: format!("alternatives.{}.reason", expected.id),
                expected: optional_string_value(expected.reason.as_deref()),
                actual: Some(optional_string_value(actual.reason.as_deref())),
            });
    }

    match (expected.confidence, actual.confidence) {
        (Some(expected_confidence), Some(actual_confidence)) => {
            if !within_confidence_tolerance(expected_confidence, actual_confidence) {
                comparison
                    .entity_alternative_mismatches
                    .push(EntityFieldMismatch {
                        id: entity_id.to_string(),
                        field: format!("alternatives.{}.confidence", expected.id),
                        expected: Value::from(expected_confidence.0),
                        actual: Some(Value::from(actual_confidence.0)),
                    });
            }
        }
        (Some(expected_confidence), None) => {
            comparison
                .entity_alternative_mismatches
                .push(EntityFieldMismatch {
                    id: entity_id.to_string(),
                    field: format!("alternatives.{}.confidence", expected.id),
                    expected: Value::from(expected_confidence.0),
                    actual: None,
                })
        }
        (None, Some(actual_confidence)) => {
            comparison
                .entity_alternative_mismatches
                .push(EntityFieldMismatch {
                    id: entity_id.to_string(),
                    field: format!("alternatives.{}.confidence", expected.id),
                    expected: Value::Null,
                    actual: Some(Value::from(actual_confidence.0)),
                })
        }
        (None, None) => {}
    }
}

fn compare_relationship(
    expected: &ResolvedRelationship,
    actual: &ResolvedRelationship,
    identity: &RelationshipIdentity,
    comparison: &mut EntityContextComparison,
) {
    if !within_confidence_tolerance(expected.confidence, actual.confidence) {
        comparison
            .relationship_confidence_mismatches
            .push(RelationshipConfidenceMismatch {
                relationship: identity.clone(),
                expected: expected.confidence,
                actual: actual.confidence,
                tolerance: ENTITY_CONTEXT_CONFIDENCE_TOLERANCE,
            });
    }

    if !is_subset(&expected.evidence, &actual.evidence) {
        comparison
            .missing_relationship_evidence
            .push(RelationshipFieldMismatch {
                relationship: identity.clone(),
                field: "evidence".to_string(),
                expected: string_array_value(&expected.evidence),
                actual: Some(string_array_value(&actual.evidence)),
            });
    }

    for (key, expected_value) in &expected.attributes {
        match actual.attributes.get(key) {
            Some(actual_value) if json_values_equivalent(expected_value, actual_value) => {}
            actual_value => {
                comparison
                    .relationship_attribute_mismatches
                    .push(RelationshipFieldMismatch {
                        relationship: identity.clone(),
                        field: format!("attributes.{key}"),
                        expected: expected_value.clone(),
                        actual: actual_value.cloned(),
                    })
            }
        }
    }
}

fn within_confidence_tolerance(expected: UnitInterval, actual: UnitInterval) -> bool {
    (expected.0 - actual.0).abs() <= ENTITY_CONTEXT_CONFIDENCE_TOLERANCE + 1e-9
}

fn is_subset(expected_subset: &[String], actual: &[String]) -> bool {
    let actual = string_set(actual);
    expected_subset
        .iter()
        .all(|expected| actual.contains(expected.as_str()))
}

fn string_set(values: &[String]) -> BTreeSet<&str> {
    values.iter().map(String::as_str).collect()
}

fn string_array_value(values: &[String]) -> Value {
    Value::Array(
        string_set(values)
            .into_iter()
            .map(|value| Value::String(value.to_string()))
            .collect(),
    )
}

fn optional_string_value(value: Option<&str>) -> Value {
    value
        .map(|value| Value::String(value.to_string()))
        .unwrap_or(Value::Null)
}

fn alternative_value(alternative: &EntityAlternative) -> Value {
    let mut value = Map::new();
    value.insert("id".to_string(), Value::String(alternative.id.clone()));
    if let Some(reason) = &alternative.reason {
        value.insert("reason".to_string(), Value::String(reason.clone()));
    }
    if let Some(confidence) = alternative.confidence {
        value.insert("confidence".to_string(), Value::from(confidence.0));
    }
    Value::Object(value)
}

fn json_values_equivalent(expected: &Value, actual: &Value) -> bool {
    match (expected, actual) {
        (Value::Array(expected_values), Value::Array(actual_values)) => {
            json_value_set(expected_values) == json_value_set(actual_values)
        }
        _ => expected == actual,
    }
}

fn json_value_set(values: &[Value]) -> BTreeSet<String> {
    values.iter().map(canonical_json).collect()
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Array(values) => {
            let items = values.iter().map(canonical_json).collect::<Vec<_>>();
            format!("[{}]", items.join(","))
        }
        Value::Object(map) => {
            let items = map
                .iter()
                .map(|(key, value)| format!("{key}:{}", canonical_json(value)))
                .collect::<BTreeSet<_>>();
            format!("{{{}}}", items.into_iter().collect::<Vec<_>>().join(","))
        }
        _ => serde_json::to_string(value).expect("json value should serialize"),
    }
}

#[derive(Debug, Clone)]
struct SpanContext {
    trace_id: String,
    span_id: String,
    parent_span_id: Option<String>,
    resource_id: String,
    entity_id: String,
    endpoint_kind: SpanEndpointKind,
    name: Option<String>,
    attributes: Map<String, Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpanEndpointKind {
    Service,
    Dependency(EntityKind),
}

type RelationshipMap = BTreeMap<String, ResolvedRelationship>;
type EntityConfidenceIndex = BTreeMap<String, UnitInterval>;

#[derive(Debug, Clone)]
struct ResourceIdentity {
    id: String,
    service_name: Option<String>,
    service_version: Option<String>,
    service_instance_id: Option<String>,
    rollout: Option<String>,
    pod_name: Option<String>,
    host_name: Option<String>,
    db_system: Option<String>,
    namespace_name: Option<String>,
    cluster_name: Option<String>,
    retry_max_attempts: Option<String>,
    retry_backoff: Option<String>,
}

fn entity_confidence_index(entities: &[ResolvedEntity]) -> BTreeMap<String, UnitInterval> {
    entities
        .iter()
        .map(|entity| (entity.id.clone(), entity.confidence))
        .collect()
}

fn span_contexts(
    store: &HotContextStore,
    resources: &[ResourceIdentity],
    ambiguous_services: &BTreeSet<String>,
) -> Vec<SpanContext> {
    store
        .raw_source_records()
        .filter(|record| record.kind == StoredRecordKind::Span)
        .filter_map(|record| {
            let resource_id = str_field(&record.payload, "resource")?.to_string();
            let resource = resource_by_id(resources, &resource_id)?;
            let (entity_id, endpoint_kind) =
                span_endpoint_for_resource(resource, ambiguous_services)?;
            let (trace_id, fallback_span_id) = trace_and_span_id_from_key(record.key.as_str())?;

            Some(SpanContext {
                trace_id,
                span_id: str_field(&record.payload, "span_id")
                    .unwrap_or(&fallback_span_id)
                    .to_string(),
                parent_span_id: str_field(&record.payload, "parent_id")
                    .or_else(|| str_field(&record.payload, "parent_span_id"))
                    .map(ToString::to_string),
                resource_id,
                entity_id,
                endpoint_kind,
                name: str_field(&record.payload, "name").map(ToString::to_string),
                attributes: attributes(&record.payload).cloned().unwrap_or_default(),
            })
        })
        .collect()
}

fn trace_and_span_id_from_key(key: &str) -> Option<(String, String)> {
    let (trace_id, span_id) = key.split_once('/')?;
    Some((trace_id.to_string(), span_id.to_string()))
}

fn span_endpoint_for_resource(
    resource: &ResourceIdentity,
    ambiguous_services: &BTreeSet<String>,
) -> Option<(String, SpanEndpointKind)> {
    if let Some((id, kind, _)) = dependency_entity_from_resource(resource) {
        return Some((id, SpanEndpointKind::Dependency(kind)));
    }

    service_entity_id_for_resource(resource, ambiguous_services)
        .map(|service_id| (service_id, SpanEndpointKind::Service))
}

fn service_entity_id_for_resource(
    resource: &ResourceIdentity,
    ambiguous_services: &BTreeSet<String>,
) -> Option<String> {
    let service_name = resource.service_name.as_deref()?;

    if !ambiguous_services.contains(service_name) {
        return Some(format!("service:{service_name}"));
    }

    if resource.rollout.as_deref() == Some("canary") {
        Some(format!("service:{service_name}@canary"))
    } else if resource.service_version.is_some() && resource.service_instance_id.is_some() {
        Some(format!("service:{service_name}@stable"))
    } else {
        Some(format!("service:{service_name}@unresolved"))
    }
}

fn service_resource_for_name<'a>(
    resources: &'a [ResourceIdentity],
    service_name: &str,
) -> Option<&'a ResourceIdentity> {
    resources.iter().find(|resource| {
        resource.db_system.is_none() && resource.service_name.as_deref() == Some(service_name)
    })
}

fn nearest_service_ancestor<'a>(
    span: &'a SpanContext,
    spans_by_id: &BTreeMap<(String, String), &'a SpanContext>,
) -> Option<&'a str> {
    let mut current = span.parent_span_id.as_deref();

    while let Some(parent_id) = current {
        let parent = spans_by_id.get(&(span.trace_id.clone(), parent_id.to_string()))?;
        if parent.endpoint_kind == SpanEndpointKind::Service {
            return Some(parent.entity_id.as_str());
        }
        current = parent.parent_span_id.as_deref();
    }

    None
}

fn insert_runtime_relationships(
    relationships: &mut RelationshipMap,
    resources: &[ResourceIdentity],
    ambiguous_services: &BTreeSet<String>,
    entity_confidences: &EntityConfidenceIndex,
) {
    for resource in resources {
        let Some(service_id) = service_entity_id_for_resource(resource, ambiguous_services) else {
            continue;
        };

        if let Some(instance_id) = &resource.service_instance_id {
            insert_relationship(
                relationships,
                resolved_relationship(
                    service_id.clone(),
                    RelationshipType::DeployedAs,
                    format!("instance:{instance_id}"),
                    UnitInterval(0.98),
                    Vec::new(),
                    BTreeMap::new(),
                    entity_confidences,
                ),
            );
        }

        if let Some(pod_name) = &resource.pod_name {
            let pod_id = format!("pod:{pod_name}");
            insert_relationship(
                relationships,
                resolved_relationship(
                    service_id,
                    RelationshipType::DeployedAs,
                    pod_id.clone(),
                    UnitInterval(0.99),
                    Vec::new(),
                    BTreeMap::new(),
                    entity_confidences,
                ),
            );

            if let Some(host_name) = &resource.host_name {
                insert_relationship(
                    relationships,
                    resolved_relationship(
                        pod_id,
                        RelationshipType::RunsOn,
                        format!("host:{host_name}"),
                        UnitInterval(0.95),
                        Vec::new(),
                        BTreeMap::new(),
                        entity_confidences,
                    ),
                );
            }
        }
    }
}

fn insert_span_relationships(
    relationships: &mut RelationshipMap,
    spans: &[SpanContext],
    resources: &[ResourceIdentity],
    ambiguous_services: &BTreeSet<String>,
    entity_confidences: &EntityConfidenceIndex,
) {
    let spans_by_id = spans
        .iter()
        .map(|span| ((span.trace_id.clone(), span.span_id.clone()), span))
        .collect::<BTreeMap<_, _>>();

    for span in spans {
        insert_parent_child_relationship(relationships, span, &spans_by_id, entity_confidences);
        insert_peer_service_relationship(
            relationships,
            span,
            resources,
            ambiguous_services,
            entity_confidences,
        );
        insert_dependency_span_relationship(
            relationships,
            span,
            spans,
            &spans_by_id,
            entity_confidences,
        );
        insert_logical_span_relationships(relationships, span, resources, entity_confidences);
    }

    insert_retry_relationships(
        relationships,
        spans,
        resources,
        &spans_by_id,
        ambiguous_services,
        entity_confidences,
    );
}

fn insert_parent_child_relationship(
    relationships: &mut RelationshipMap,
    span: &SpanContext,
    spans_by_id: &BTreeMap<(String, String), &SpanContext>,
    entity_confidences: &EntityConfidenceIndex,
) {
    if span.endpoint_kind != SpanEndpointKind::Service {
        return;
    }

    let Some(parent_service) = nearest_service_ancestor(span, spans_by_id) else {
        return;
    };

    if parent_service == span.entity_id {
        return;
    }

    insert_relationship(
        relationships,
        resolved_relationship(
            parent_service.to_string(),
            RelationshipType::Calls,
            span.entity_id.clone(),
            UnitInterval(0.98),
            vec![format!("trace:{}", span.trace_id)],
            BTreeMap::new(),
            entity_confidences,
        ),
    );
}

fn insert_peer_service_relationship(
    relationships: &mut RelationshipMap,
    span: &SpanContext,
    resources: &[ResourceIdentity],
    ambiguous_services: &BTreeSet<String>,
    entity_confidences: &EntityConfidenceIndex,
) {
    let Some(peer_service) = str_attr(Some(&span.attributes), "peer.service") else {
        return;
    };
    let Some(src) = (match span.endpoint_kind {
        SpanEndpointKind::Service => Some(span.entity_id.as_str()),
        SpanEndpointKind::Dependency(_) => None,
    }) else {
        return;
    };

    if let Some(peer_resource) = service_resource_for_name(resources, peer_service)
        && let Some(dst) = service_entity_id_for_resource(peer_resource, ambiguous_services)
    {
        if src != dst {
            insert_relationship(
                relationships,
                resolved_relationship(
                    src.to_string(),
                    RelationshipType::Calls,
                    dst,
                    UnitInterval(0.98),
                    vec![format!("trace:{}", span.trace_id)],
                    BTreeMap::new(),
                    entity_confidences,
                ),
            );
        }
    } else {
        let mut attributes = BTreeMap::new();
        if span
            .name
            .as_deref()
            .is_some_and(|name| name.to_ascii_lowercase().contains("charge"))
        {
            attributes.insert("role".to_string(), json!("payment-provider"));
        }
        insert_relationship(
            relationships,
            resolved_relationship(
                src.to_string(),
                RelationshipType::DependsOn,
                format!("external-api:{peer_service}"),
                UnitInterval(0.95),
                vec![format!("trace:{}", span.trace_id)],
                attributes,
                entity_confidences,
            ),
        );
    }
}

fn insert_dependency_span_relationship(
    relationships: &mut RelationshipMap,
    span: &SpanContext,
    spans: &[SpanContext],
    spans_by_id: &BTreeMap<(String, String), &SpanContext>,
    entity_confidences: &EntityConfidenceIndex,
) {
    let SpanEndpointKind::Dependency(kind) = span.endpoint_kind else {
        return;
    };
    let Some(src) = nearest_service_ancestor(span, spans_by_id) else {
        return;
    };
    let relationship_type = if is_write_dependency_span(span) {
        RelationshipType::WritesTo
    } else {
        RelationshipType::ReadsFrom
    };
    let confidence = match relationship_type {
        RelationshipType::WritesTo => UnitInterval(0.96),
        _ if kind == EntityKind::Cache => UnitInterval(0.97),
        _ if has_cache_miss_sibling(span, spans) => UnitInterval(0.96),
        _ => UnitInterval(0.97),
    };
    let mut attributes = BTreeMap::new();

    if kind == EntityKind::Database && has_cache_miss_sibling(span, spans) {
        attributes.insert("role".to_string(), json!("cache-miss-fallback"));
    }

    insert_relationship(
        relationships,
        resolved_relationship(
            src.to_string(),
            relationship_type,
            span.entity_id.clone(),
            confidence,
            vec![format!("trace:{}", span.trace_id)],
            attributes,
            entity_confidences,
        ),
    );
}

fn insert_logical_span_relationships(
    relationships: &mut RelationshipMap,
    span: &SpanContext,
    resources: &[ResourceIdentity],
    entity_confidences: &EntityConfidenceIndex,
) {
    if span.endpoint_kind != SpanEndpointKind::Service {
        return;
    }

    if let Some(shard) = str_attr(Some(&span.attributes), "orders.shard")
        && let Some(service_name) = service_name_from_entity_id(&span.entity_id)
    {
        let shard_id = format!("shard:{service_name}-shard-{shard}");
        insert_relationship(
            relationships,
            resolved_relationship(
                span.entity_id.clone(),
                RelationshipType::FansOutTo,
                shard_id,
                UnitInterval(0.90),
                Vec::new(),
                BTreeMap::new(),
                entity_confidences,
            ),
        );
    }

    if let Some(tenant) = str_attr(Some(&span.attributes), "tenant.id") {
        let mut attributes = BTreeMap::new();
        if let Some(shard) = str_attr(Some(&span.attributes), "orders.shard") {
            attributes.insert(
                "routed_to_shard".to_string(),
                Value::String(shard.to_string()),
            );
        }

        if resources
            .iter()
            .any(|resource| resource.id == span.resource_id && resource.db_system.is_none())
        {
            insert_relationship(
                relationships,
                resolved_relationship(
                    format!("tenant:{tenant}"),
                    RelationshipType::Calls,
                    span.entity_id.clone(),
                    UnitInterval(0.90),
                    vec![format!("trace:{}", span.trace_id)],
                    attributes,
                    entity_confidences,
                ),
            );
        }
    }
}

fn insert_retry_relationships(
    relationships: &mut RelationshipMap,
    spans: &[SpanContext],
    resources: &[ResourceIdentity],
    spans_by_id: &BTreeMap<(String, String), &SpanContext>,
    ambiguous_services: &BTreeSet<String>,
    entity_confidences: &EntityConfidenceIndex,
) {
    let mut attempts_by_edge: BTreeMap<(String, String, String), Vec<i64>> = BTreeMap::new();

    for span in spans {
        let Some(attempt) = int_attr(Some(&span.attributes), "retry.attempt") else {
            continue;
        };
        let Some(src) = nearest_service_ancestor(span, spans_by_id) else {
            continue;
        };
        let dst = if span.endpoint_kind == SpanEndpointKind::Service {
            Some(span.entity_id.clone())
        } else if let Some(peer_service) = str_attr(Some(&span.attributes), "peer.service") {
            service_resource_for_name(resources, peer_service)
                .and_then(|resource| service_entity_id_for_resource(resource, ambiguous_services))
        } else {
            None
        };
        let Some(dst) = dst else {
            continue;
        };

        attempts_by_edge
            .entry((span.trace_id.clone(), src.to_string(), dst))
            .or_default()
            .push(attempt);
    }

    for ((trace_id, src, dst), attempts) in attempts_by_edge {
        let max_attempt = attempts.iter().copied().max().unwrap_or_default();
        if attempts.len() < 2 && max_attempt < 2 {
            continue;
        }

        let mut attributes = BTreeMap::new();
        if let Some(max_attempts) =
            retry_resource_attr(resources, &src, "checkout.retry.max_attempts")
                .and_then(|value| value.parse::<i64>().ok())
        {
            attributes.insert(
                "max_attempts".to_string(),
                Value::Number(max_attempts.into()),
            );
        } else {
            attributes.insert(
                "max_attempts".to_string(),
                Value::Number(max_attempt.into()),
            );
        }
        if let Some(backoff) = retry_resource_attr(resources, &src, "checkout.retry.backoff") {
            attributes.insert("backoff".to_string(), Value::String(backoff.to_string()));
        }

        insert_relationship(
            relationships,
            resolved_relationship(
                src,
                RelationshipType::Retries,
                dst,
                UnitInterval(0.95),
                vec![format!("trace:{trace_id}")],
                attributes,
                entity_confidences,
            ),
        );
    }
}

fn insert_resource_name_relationships(
    relationships: &mut RelationshipMap,
    resources: &[ResourceIdentity],
    ambiguous_services: &BTreeSet<String>,
    entity_confidences: &EntityConfidenceIndex,
) {
    for resource in resources {
        let Some(service_name) = resource.service_name.as_deref() else {
            continue;
        };
        let Some(prefix) = service_name.strip_suffix("-ui") else {
            continue;
        };
        let Some(api_resource) = service_resource_for_name(resources, &format!("{prefix}-api"))
        else {
            continue;
        };
        let Some(src) = service_entity_id_for_resource(resource, ambiguous_services) else {
            continue;
        };
        let Some(dst) = service_entity_id_for_resource(api_resource, ambiguous_services) else {
            continue;
        };

        insert_relationship(
            relationships,
            resolved_relationship(
                src,
                RelationshipType::Calls,
                dst,
                UnitInterval(0.95),
                Vec::new(),
                BTreeMap::new(),
                entity_confidences,
            ),
        );
    }
}

fn insert_change_inferred_relationships(
    relationships: &mut RelationshipMap,
    store: &HotContextStore,
    resources: &[ResourceIdentity],
    ambiguous_services: &BTreeSet<String>,
    entity_confidences: &EntityConfidenceIndex,
) {
    for record in store
        .raw_source_records()
        .filter(|record| record.kind == StoredRecordKind::Change)
    {
        let Some(src) = str_field(&record.payload, "entity") else {
            continue;
        };
        if !src.starts_with("service:") {
            continue;
        }
        let summary = str_field(&record.payload, "summary").unwrap_or_default();
        if !summary.to_ascii_lowercase().contains("vacuum") {
            continue;
        }
        let Some(dst) = database_entity_for_text(summary, resources) else {
            continue;
        };
        let service_name = src
            .strip_prefix("service:")
            .and_then(|name| name.split_once('@').map(|(base, _)| base).or(Some(name)));
        let resolved_src = service_name
            .and_then(|name| service_resource_for_name(resources, name))
            .and_then(|resource| service_entity_id_for_resource(resource, ambiguous_services))
            .unwrap_or_else(|| src.to_string());

        insert_relationship(
            relationships,
            resolved_relationship(
                resolved_src,
                RelationshipType::WritesTo,
                dst,
                UnitInterval(0.95),
                vec![record.key.as_str().to_string()],
                BTreeMap::new(),
                entity_confidences,
            ),
        );
    }
}

fn insert_prior_inferred_relationships(
    relationships: &mut RelationshipMap,
    store: &HotContextStore,
    resources: &[ResourceIdentity],
    entity_confidences: &EntityConfidenceIndex,
) {
    for record in store
        .raw_source_records()
        .filter(|record| record.kind == StoredRecordKind::PriorIncident)
    {
        let Some(signature) = record.payload.get("signature").and_then(Value::as_object) else {
            continue;
        };
        let Some(primary_entity) = signature.get("primary_entity").and_then(Value::as_str) else {
            continue;
        };
        let Some(trigger) = signature.get("trigger").and_then(Value::as_str) else {
            continue;
        };
        let Some((src, _)) = trigger.split_once(' ') else {
            continue;
        };
        if !src.starts_with("service:") || !primary_entity.starts_with("db:") {
            continue;
        }
        if resources.iter().all(|resource| {
            resource.service_name.as_deref() != Some(primary_entity.trim_start_matches("db:"))
        }) {
            continue;
        }

        insert_relationship(
            relationships,
            resolved_relationship(
                src.to_string(),
                RelationshipType::WritesTo,
                primary_entity.to_string(),
                UnitInterval(0.95),
                vec![record.key.as_str().to_string()],
                BTreeMap::new(),
                entity_confidences,
            ),
        );
    }
}

fn insert_shared_resource_relationships(
    relationships: &mut RelationshipMap,
    entity_confidences: &EntityConfidenceIndex,
) {
    let existing = relationships.values().cloned().collect::<Vec<_>>();

    for read in existing
        .iter()
        .filter(|relationship| relationship.relationship_type == RelationshipType::ReadsFrom)
    {
        for write in existing.iter().filter(|relationship| {
            relationship.relationship_type == RelationshipType::WritesTo
                && relationship.dst == read.dst
                && relationship.confidence >= UnitInterval(0.96)
        }) {
            if read.src == write.src {
                continue;
            }
            insert_relationship(
                relationships,
                resolved_relationship(
                    read.src.clone(),
                    RelationshipType::SharesResourceWith,
                    write.src.clone(),
                    UnitInterval(0.80),
                    vec![read.dst.clone()],
                    BTreeMap::new(),
                    entity_confidences,
                ),
            );
        }
    }
}

fn is_write_dependency_span(span: &SpanContext) -> bool {
    let operation = str_attr(Some(&span.attributes), "db.statement")
        .or(span.name.as_deref())
        .unwrap_or_default()
        .trim_start()
        .to_ascii_uppercase();

    [
        "INSERT", "UPDATE", "DELETE", "CREATE", "ALTER", "DROP", "VACUUM",
    ]
    .iter()
    .any(|prefix| operation.starts_with(prefix))
}

fn has_cache_miss_sibling(span: &SpanContext, spans: &[SpanContext]) -> bool {
    spans.iter().any(|candidate| {
        candidate.trace_id == span.trace_id
            && candidate.parent_span_id == span.parent_span_id
            && candidate.endpoint_kind == SpanEndpointKind::Dependency(EntityKind::Cache)
            && candidate
                .attributes
                .get("cache.hit")
                .and_then(Value::as_bool)
                == Some(false)
    })
}

fn database_entity_for_text(text: &str, resources: &[ResourceIdentity]) -> Option<String> {
    let lowered = text.to_ascii_lowercase();

    resources.iter().find_map(|resource| {
        resource.db_system.as_ref()?;
        let service_name = resource.service_name.as_deref()?;
        let stem = service_name.strip_suffix("-pg").unwrap_or(service_name);
        if lowered.contains(stem) || lowered.contains(service_name) {
            Some(format!("db:{service_name}"))
        } else {
            None
        }
    })
}

fn service_name_from_entity_id(entity_id: &str) -> Option<&str> {
    entity_id
        .strip_prefix("service:")?
        .split_once('@')
        .map(|(base, _)| base)
        .or(Some(entity_id.strip_prefix("service:")?))
}

fn retry_resource_attr<'a>(
    resources: &'a [ResourceIdentity],
    service_entity_id: &str,
    attr: &str,
) -> Option<&'a str> {
    let service_name = service_name_from_entity_id(service_entity_id)?;
    resources
        .iter()
        .find(|resource| resource.service_name.as_deref() == Some(service_name))
        .and_then(|resource| resource_attribute(resource, attr))
}

fn resource_attribute<'a>(resource: &'a ResourceIdentity, attr: &str) -> Option<&'a str> {
    match attr {
        "checkout.retry.max_attempts" => resource.retry_max_attempts.as_deref(),
        "checkout.retry.backoff" => resource.retry_backoff.as_deref(),
        _ => None,
    }
}

fn int_attr(attributes: Option<&Map<String, Value>>, key: &str) -> Option<i64> {
    attributes?.get(key).and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
    })
}

fn resolved_relationship(
    src: String,
    relationship_type: RelationshipType,
    dst: String,
    confidence: UnitInterval,
    evidence: Vec<String>,
    attributes: BTreeMap<String, Value>,
    entity_confidences: &EntityConfidenceIndex,
) -> ResolvedRelationship {
    ResolvedRelationship {
        confidence: relationship_confidence(&src, &dst, confidence, entity_confidences),
        src,
        relationship_type,
        dst,
        evidence: dedupe_stable(evidence),
        attributes,
    }
}

fn relationship_confidence(
    src: &str,
    dst: &str,
    base: UnitInterval,
    entity_confidences: &EntityConfidenceIndex,
) -> UnitInterval {
    let src_confidence = entity_confidences
        .get(src)
        .copied()
        .unwrap_or(UnitInterval(1.0));
    let dst_confidence = entity_confidences
        .get(dst)
        .copied()
        .unwrap_or(UnitInterval(1.0));

    if src_confidence < UnitInterval(0.50) || dst_confidence < UnitInterval(0.50) {
        UnitInterval((base.0 - 0.20).max(0.30))
    } else {
        base
    }
}

fn insert_relationship(relationships: &mut RelationshipMap, relationship: ResolvedRelationship) {
    let key = relationship_store_key(
        &relationship.src,
        relationship.relationship_type,
        &relationship.dst,
    );
    let Some(existing) = relationships.get_mut(&key) else {
        relationships.insert(key, relationship);
        return;
    };

    if relationship.confidence > existing.confidence {
        existing.confidence = relationship.confidence;
    }
    existing.evidence = dedupe_stable(
        existing
            .evidence
            .iter()
            .chain(&relationship.evidence)
            .cloned()
            .collect(),
    );
    for (key, value) in relationship.attributes {
        existing.attributes.entry(key).or_insert(value);
    }
}

fn resource_identities(store: &HotContextStore) -> Vec<ResourceIdentity> {
    store
        .raw_source_records()
        .filter(|record| record.kind == StoredRecordKind::Resource)
        .map(|record| {
            let attributes = attributes(&record.payload);

            ResourceIdentity {
                id: record.key.as_str().to_string(),
                service_name: str_attr(attributes, "service.name").map(ToString::to_string),
                service_version: str_attr(attributes, "service.version").map(ToString::to_string),
                service_instance_id: str_attr(attributes, "service.instance.id")
                    .map(ToString::to_string),
                rollout: str_attr(attributes, "rollout").map(ToString::to_string),
                pod_name: str_attr(attributes, "k8s.pod.name").map(ToString::to_string),
                host_name: str_attr(attributes, "host.name").map(ToString::to_string),
                db_system: str_attr(attributes, "db.system")
                    .or_else(|| str_attr(attributes, "db.system.name"))
                    .map(ToString::to_string),
                namespace_name: str_attr(attributes, "k8s.namespace.name").map(ToString::to_string),
                cluster_name: str_attr(attributes, "cluster.name").map(ToString::to_string),
                retry_max_attempts: str_attr(attributes, "checkout.retry.max_attempts")
                    .map(ToString::to_string),
                retry_backoff: str_attr(attributes, "checkout.retry.backoff")
                    .map(ToString::to_string),
            }
        })
        .collect()
}

fn ambiguous_service_names(resources: &[ResourceIdentity]) -> BTreeSet<String> {
    let mut grouped: BTreeMap<String, Vec<&ResourceIdentity>> = BTreeMap::new();

    for resource in resources
        .iter()
        .filter(|resource| resource.db_system.is_none())
    {
        if let Some(service_name) = &resource.service_name {
            grouped
                .entry(service_name.clone())
                .or_default()
                .push(resource);
        }
    }

    grouped
        .into_iter()
        .filter_map(|(service_name, service_resources)| {
            let has_discriminated_resource = service_resources.iter().any(|resource| {
                resource.rollout.is_some()
                    || resource.service_version.is_some()
                    || resource.service_instance_id.is_some()
            });
            let has_missing_discriminator = service_resources.iter().any(|resource| {
                resource.service_version.is_none() || resource.service_instance_id.is_none()
            });

            if service_resources.len() > 1
                && has_discriminated_resource
                && has_missing_discriminator
            {
                Some(service_name)
            } else {
                None
            }
        })
        .collect()
}

fn insert_resource_entities(
    entities: &mut BTreeMap<String, ResolvedEntity>,
    resources: &[ResourceIdentity],
    ambiguous_services: &BTreeSet<String>,
    ambiguous_unresolved_shares: &BTreeMap<String, UnitInterval>,
) {
    for resource in resources {
        if let Some((id, kind, confidence)) = dependency_entity_from_resource(resource) {
            insert_or_merge(
                entities,
                resolved_entity(id, kind, vec![resource.id.clone()], confidence),
            );
        } else if let Some(service_name) = &resource.service_name
            && !ambiguous_services.contains(service_name)
        {
            insert_or_merge(
                entities,
                resolved_entity(
                    format!("service:{service_name}"),
                    EntityKind::Service,
                    vec![resource.id.clone()],
                    service_confidence(resource),
                ),
            );
        }

        if let Some(pod_name) = &resource.pod_name {
            insert_or_merge(
                entities,
                resolved_entity(
                    format!("pod:{pod_name}"),
                    EntityKind::Pod,
                    vec![resource.id.clone()],
                    pod_confidence(resource),
                ),
            );
        }

        if let Some(host_name) = &resource.host_name
            && resource.db_system.is_none()
        {
            insert_or_merge(
                entities,
                resolved_entity(
                    format!("host:{host_name}"),
                    EntityKind::Host,
                    vec![resource.id.clone()],
                    UnitInterval(0.95),
                ),
            );
        }

        if let Some(instance_id) = &resource.service_instance_id {
            insert_or_merge(
                entities,
                resolved_entity(
                    format!("instance:{instance_id}"),
                    EntityKind::Instance,
                    vec![resource.id.clone()],
                    UnitInterval(0.98),
                ),
            );
        }
    }

    for service_name in ambiguous_services {
        insert_ambiguous_service_entities(
            entities,
            resources,
            service_name,
            ambiguous_unresolved_shares.get(service_name).copied(),
        );
    }
}

fn ambiguous_unresolved_share_estimates(
    store: &HotContextStore,
    resources: &[ResourceIdentity],
    ambiguous_services: &BTreeSet<String>,
) -> BTreeMap<String, UnitInterval> {
    let raw_records = store.raw_source_records().collect::<Vec<_>>();
    let total_records = raw_records.len();
    let mut estimates = BTreeMap::new();

    if total_records == 0 {
        return estimates;
    }

    for service_name in ambiguous_services {
        let unresolved_resource_ids = resources
            .iter()
            .filter(|resource| {
                resource.service_name.as_deref() == Some(service_name)
                    && (resource.service_version.is_none()
                        || resource.service_instance_id.is_none())
            })
            .map(|resource| resource.id.as_str())
            .collect::<BTreeSet<_>>();
        let unresolved_records = raw_records
            .iter()
            .filter(|record| {
                record.kind != StoredRecordKind::Resource
                    && record_mentions_unresolved_service(
                        record,
                        service_name,
                        &unresolved_resource_ids,
                    )
            })
            .count();

        if unresolved_records > 0 {
            let share = unresolved_records as f64 / total_records as f64;
            estimates.insert(
                service_name.clone(),
                UnitInterval(round_two_decimals(share)),
            );
        }
    }

    estimates
}

fn record_mentions_unresolved_service(
    record: &StoredRecord,
    service_name: &str,
    unresolved_resource_ids: &BTreeSet<&str>,
) -> bool {
    str_field(&record.payload, "resource")
        .is_some_and(|resource| unresolved_resource_ids.contains(resource))
        || str_field(&record.payload, "entity").is_some_and(|entity| {
            entity == format!("{service_name}(unresolved)")
                || entity == format!("service:{service_name}@unresolved")
        })
        || record
            .entities
            .iter()
            .any(|entity| unresolved_resource_ids.contains(entity.as_str()))
}

fn round_two_decimals(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn dependency_entity_from_resource(
    resource: &ResourceIdentity,
) -> Option<(String, EntityKind, UnitInterval)> {
    let db_system = resource.db_system.as_deref()?;
    let service_name = resource.service_name.as_deref()?;

    if db_system == "redis" {
        if resource.cluster_name.is_some() {
            Some((
                format!("infra:{service_name}"),
                EntityKind::Cache,
                UnitInterval(0.97),
            ))
        } else {
            Some((
                format!("db:{service_name}"),
                EntityKind::Cache,
                UnitInterval(0.97),
            ))
        }
    } else {
        Some((
            format!("db:{service_name}"),
            EntityKind::Database,
            UnitInterval(0.98),
        ))
    }
}

fn service_confidence(resource: &ResourceIdentity) -> UnitInterval {
    match (
        resource.service_version.is_some(),
        resource.service_instance_id.is_some(),
        resource.namespace_name.is_some(),
    ) {
        (true, true, _) | (true, _, true) => UnitInterval(0.99),
        (true, false, false) => UnitInterval(0.98),
        (false, true, _) | (false, false, true) => UnitInterval(0.95),
        (false, false, false) => UnitInterval(0.90),
    }
}

fn pod_confidence(resource: &ResourceIdentity) -> UnitInterval {
    if resource.host_name.is_some() {
        UnitInterval(0.99)
    } else {
        UnitInterval(0.98)
    }
}

fn insert_ambiguous_service_entities(
    entities: &mut BTreeMap<String, ResolvedEntity>,
    resources: &[ResourceIdentity],
    service_name: &str,
    unresolved_share: Option<UnitInterval>,
) {
    let service_resources = resources
        .iter()
        .filter(|resource| {
            resource.service_name.as_deref() == Some(service_name) && resource.db_system.is_none()
        })
        .collect::<Vec<_>>();
    let canary = service_resources
        .iter()
        .copied()
        .filter(|resource| resource.rollout.as_deref() == Some("canary"))
        .collect::<Vec<_>>();
    let stable = service_resources
        .iter()
        .copied()
        .filter(|resource| {
            resource.rollout.as_deref() != Some("canary")
                && resource.service_version.is_some()
                && resource.service_instance_id.is_some()
        })
        .collect::<Vec<_>>();
    let unresolved = service_resources
        .iter()
        .copied()
        .filter(|resource| {
            resource.service_version.is_none() || resource.service_instance_id.is_none()
        })
        .collect::<Vec<_>>();

    if !canary.is_empty() {
        let mut discriminators = BTreeMap::new();
        insert_single_discriminator(&mut discriminators, "service.version", &canary);
        insert_single_discriminator(&mut discriminators, "service.instance.id", &canary);
        discriminators.insert("rollout".to_string(), json!("canary"));

        let mut entity = resolved_entity(
            format!("service:{service_name}@canary"),
            EntityKind::Service,
            canary.iter().map(|resource| resource.id.clone()).collect(),
            UnitInterval(0.96),
        );
        entity.discriminators = discriminators;
        entity.alternatives = vec![EntityAlternative {
            id: format!("service:{service_name}@stable"),
            reason: Some("same service.name".to_string()),
            confidence: Some(UnitInterval(0.04)),
        }];
        insert_or_merge(entities, entity);
    }

    if !stable.is_empty() {
        let mut discriminators = BTreeMap::new();
        insert_single_discriminator(&mut discriminators, "service.version", &stable);
        let instances = stable
            .iter()
            .filter_map(|resource| resource.service_instance_id.as_deref())
            .map(|value| Value::String(value.to_string()))
            .collect::<Vec<_>>();
        if !instances.is_empty() {
            discriminators.insert("service.instance.id".to_string(), Value::Array(instances));
        }

        let mut entity = resolved_entity(
            format!("service:{service_name}@stable"),
            EntityKind::Service,
            stable.iter().map(|resource| resource.id.clone()).collect(),
            UnitInterval(0.95),
        );
        entity.discriminators = discriminators;
        entity.alternatives = vec![EntityAlternative {
            id: format!("service:{service_name}@canary"),
            reason: Some("same service.name".to_string()),
            confidence: Some(UnitInterval(0.05)),
        }];
        insert_or_merge(entities, entity);
    }

    if !unresolved.is_empty() {
        let mut entity = resolved_entity(
            format!("service:{service_name}@unresolved"),
            EntityKind::Service,
            unresolved
                .iter()
                .map(|resource| resource.id.clone())
                .collect(),
            UnitInterval(0.40),
        );
        entity.unresolved = true;
        entity.missing_attributes = vec![
            "service.version".to_string(),
            "service.instance.id".to_string(),
        ];
        entity.estimated_share = unresolved_share;
        entity.alternatives = vec![
            EntityAlternative {
                id: format!("service:{service_name}@canary"),
                reason: None,
                confidence: Some(UnitInterval(0.50)),
            },
            EntityAlternative {
                id: format!("service:{service_name}@stable"),
                reason: None,
                confidence: Some(UnitInterval(0.50)),
            },
        ];
        insert_or_merge(entities, entity);
    }
}

fn insert_single_discriminator(
    discriminators: &mut BTreeMap<String, Value>,
    key: &str,
    resources: &[&ResourceIdentity],
) {
    let values = resources
        .iter()
        .filter_map(|resource| match key {
            "service.version" => resource.service_version.as_deref(),
            "service.instance.id" => resource.service_instance_id.as_deref(),
            _ => None,
        })
        .collect::<BTreeSet<_>>();

    match values.len() {
        0 => {}
        1 => {
            discriminators.insert(
                key.to_string(),
                Value::String(values.iter().next().expect("one value").to_string()),
            );
        }
        _ => {
            discriminators.insert(
                key.to_string(),
                Value::Array(
                    values
                        .into_iter()
                        .map(|value| Value::String(value.to_string()))
                        .collect(),
                ),
            );
        }
    }
}

fn insert_span_entities(
    entities: &mut BTreeMap<String, ResolvedEntity>,
    record: &StoredRecord,
    resources: &[ResourceIdentity],
) {
    let Some(span_resource_id) = str_field(&record.payload, "resource") else {
        return;
    };
    let Some(resource) = resource_by_id(resources, span_resource_id) else {
        return;
    };
    let attributes = attributes(&record.payload);

    insert_route_entity(entities, record, resource, attributes);
    insert_logical_entities_from_attributes(entities, attributes, resource);
    insert_external_api_from_span(entities, record, attributes, resources);
}

fn insert_route_entity(
    entities: &mut BTreeMap<String, ResolvedEntity>,
    record: &StoredRecord,
    resource: &ResourceIdentity,
    attributes: Option<&Map<String, Value>>,
) {
    let Some(service_name) = resource.service_name.as_deref() else {
        return;
    };
    let Some(route) = str_attr(attributes, "http.route") else {
        return;
    };
    let method = str_attr(attributes, "http.method")
        .or_else(|| method_from_span_name(str_field(&record.payload, "name")));
    let route_key = match method {
        Some(method) => format!("{method} {route}"),
        None => route.to_string(),
    };
    let confidence = if method.is_some() {
        UnitInterval(0.95)
    } else {
        UnitInterval(0.88)
    };

    insert_or_merge(
        entities,
        resolved_entity(
            format!("route:{service_name}/{route_key}"),
            EntityKind::Route,
            vec![resource.id.clone()],
            confidence,
        ),
    );
}

fn insert_logical_entities_from_attributes(
    entities: &mut BTreeMap<String, ResolvedEntity>,
    attributes: Option<&Map<String, Value>>,
    resource: &ResourceIdentity,
) {
    if let Some(shard) = str_attr(attributes, "orders.shard")
        && let Some(service_name) = resource.service_name.as_deref()
    {
        let mut entity = resolved_entity(
            format!("shard:{service_name}-shard-{shard}"),
            EntityKind::Partition,
            vec![resource.id.clone()],
            UnitInterval(0.90),
        );
        entity
            .discriminators
            .insert("orders.shard".to_string(), Value::String(shard.to_string()));
        insert_or_merge(entities, entity);
    }

    if let Some(tenant) = str_attr(attributes, "tenant.id") {
        let mut entity = resolved_entity(
            format!("tenant:{tenant}"),
            EntityKind::Tenant,
            vec![resource.id.clone()],
            UnitInterval(0.88),
        );
        entity
            .discriminators
            .insert("tenant.id".to_string(), Value::String(tenant.to_string()));
        insert_or_merge(entities, entity);
    }
}

fn insert_external_api_from_span(
    entities: &mut BTreeMap<String, ResolvedEntity>,
    record: &StoredRecord,
    attributes: Option<&Map<String, Value>>,
    resources: &[ResourceIdentity],
) {
    let Some(peer_service) = str_attr(attributes, "peer.service") else {
        return;
    };

    if resources
        .iter()
        .any(|resource| resource.service_name.as_deref() == Some(peer_service))
    {
        return;
    }

    let mut entity = resolved_entity(
        format!("external-api:{peer_service}"),
        EntityKind::ExternalApi,
        vec![format!("trace:{}", record.key.as_str())],
        UnitInterval(0.90),
    );
    entity
        .discriminators
        .insert("peer.service".to_string(), json!(peer_service));
    if let Some(server_address) = str_attr(attributes, "server.address") {
        entity
            .discriminators
            .insert("server.address".to_string(), json!(server_address));
    }

    insert_or_merge(entities, entity);
}

fn insert_record_entity_hint(
    entities: &mut BTreeMap<String, ResolvedEntity>,
    record: &StoredRecord,
    resources: &[ResourceIdentity],
    ambiguous_services: &BTreeSet<String>,
) {
    if let Some(entity_id) = str_field(&record.payload, "entity")
        && let Some(entity) = entity_from_hint(record, entity_id, resources, ambiguous_services)
    {
        insert_or_merge(entities, entity);
    }

    if record.kind == StoredRecordKind::Change {
        let attributes = attributes(&record.payload);
        if let Some(tenant) = str_attr(attributes, "tenant.id") {
            let mut entity = resolved_entity(
                format!("tenant:{tenant}"),
                EntityKind::Tenant,
                source_for_hint(record, &format!("tenant:{tenant}"), resources),
                UnitInterval(0.88),
            );
            entity
                .discriminators
                .insert("tenant.id".to_string(), Value::String(tenant.to_string()));
            insert_or_merge(entities, entity);
        }
    }
}

fn insert_telemetry_gap_entities(
    entities: &mut BTreeMap<String, ResolvedEntity>,
    record: &StoredRecord,
    resources: &[ResourceIdentity],
    ambiguous_services: &BTreeSet<String>,
) {
    if let Some(affected) = record
        .payload
        .get("affected_entities")
        .and_then(Value::as_array)
    {
        for entity_id in affected.iter().filter_map(Value::as_str) {
            if let Some(entity) = entity_from_hint(record, entity_id, resources, ambiguous_services)
            {
                insert_or_merge(entities, entity);
            }
        }
    }
}

fn entity_from_hint(
    record: &StoredRecord,
    entity_id: &str,
    resources: &[ResourceIdentity],
    ambiguous_services: &BTreeSet<String>,
) -> Option<ResolvedEntity> {
    if entity_id.contains('(') {
        return None;
    }

    let (prefix, name) = entity_id.split_once(':')?;
    let base_name = name.split_once('@').map(|(base, _)| base).unwrap_or(name);
    let kind = match prefix {
        "service" => {
            if ambiguous_services.contains(base_name) && !name.contains('@') {
                return None;
            }
            EntityKind::Service
        }
        "route" => EntityKind::Route,
        "instance" => EntityKind::Instance,
        "pod" => EntityKind::Pod,
        "db" => {
            if resources.iter().any(|resource| {
                resource.service_name.as_deref() == Some(base_name)
                    && resource.db_system.as_deref() == Some("redis")
            }) {
                EntityKind::Cache
            } else {
                EntityKind::Database
            }
        }
        "queue" => EntityKind::Queue,
        "cache" => EntityKind::Cache,
        "external-api" => EntityKind::ExternalApi,
        "tenant" => EntityKind::Tenant,
        "infra" => {
            if base_name.contains("redis") || base_name.contains("cache") {
                EntityKind::Cache
            } else {
                EntityKind::Infra
            }
        }
        "deployment" => EntityKind::Deployment,
        "host" => EntityKind::Host,
        "container" => EntityKind::Container,
        "shard" => EntityKind::Partition,
        _ => return None,
    };
    let confidence = confidence_from_hint(kind);
    let mut entity = resolved_entity(
        entity_id.to_string(),
        kind,
        source_for_hint(record, entity_id, resources),
        confidence,
    );

    if kind == EntityKind::Partition
        && let Some(shard) = shard_number(entity_id)
    {
        entity
            .discriminators
            .insert("orders.shard".to_string(), Value::String(shard.to_string()));
    }

    if kind == EntityKind::Tenant {
        entity.discriminators.insert(
            "tenant.id".to_string(),
            Value::String(base_name.to_string()),
        );
    }

    Some(entity)
}

fn confidence_from_hint(kind: EntityKind) -> UnitInterval {
    match kind {
        EntityKind::Partition => UnitInterval(0.90),
        EntityKind::Tenant => UnitInterval(0.88),
        EntityKind::ExternalApi => UnitInterval(0.90),
        EntityKind::Instance => UnitInterval(0.98),
        EntityKind::Pod => UnitInterval(0.98),
        EntityKind::Database | EntityKind::Cache => UnitInterval(0.97),
        _ => UnitInterval(0.90),
    }
}

fn source_for_hint(
    record: &StoredRecord,
    entity_id: &str,
    resources: &[ResourceIdentity],
) -> Vec<String> {
    if let Some(resource_id) = str_field(&record.payload, "resource") {
        return vec![resource_id.to_string()];
    }

    if let Some(resource_id) = infer_resource_for_entity(entity_id, resources) {
        return vec![resource_id];
    }

    vec![record_source_ref(record)]
}

fn infer_resource_for_entity(entity_id: &str, resources: &[ResourceIdentity]) -> Option<String> {
    let (prefix, name) = entity_id.split_once(':')?;
    let base_name = name.split_once('@').map(|(base, _)| base).unwrap_or(name);

    match prefix {
        "service" => resources
            .iter()
            .find(|resource| {
                resource.db_system.is_none() && resource.service_name.as_deref() == Some(base_name)
            })
            .map(|resource| resource.id.clone()),
        "db" => resources
            .iter()
            .find(|resource| {
                resource.service_name.as_deref() == Some(base_name) && resource.db_system.is_some()
            })
            .map(|resource| resource.id.clone()),
        "infra" | "cache" => resources
            .iter()
            .find(|resource| {
                resource.service_name.as_deref() == Some(base_name)
                    || resource.id.trim_start_matches("res:") == base_name
            })
            .map(|resource| resource.id.clone()),
        "pod" => resources
            .iter()
            .find(|resource| resource.pod_name.as_deref() == Some(base_name))
            .map(|resource| resource.id.clone()),
        "host" => resources
            .iter()
            .find(|resource| resource.host_name.as_deref() == Some(base_name))
            .map(|resource| resource.id.clone()),
        "shard" => service_name_from_shard_id(entity_id).and_then(|service_name| {
            resources
                .iter()
                .find(|resource| resource.service_name.as_deref() == Some(service_name))
                .map(|resource| resource.id.clone())
        }),
        "tenant" if resources.len() == 1 => Some(resources[0].id.clone()),
        _ => None,
    }
}

fn record_source_ref(record: &StoredRecord) -> String {
    match record.kind {
        StoredRecordKind::Span => format!("trace:{}", record.key.as_str()),
        _ => record.key.as_str().to_string(),
    }
}

fn resource_by_id<'a>(
    resources: &'a [ResourceIdentity],
    resource_id: &str,
) -> Option<&'a ResourceIdentity> {
    resources.iter().find(|resource| resource.id == resource_id)
}

fn attributes(value: &Value) -> Option<&Map<String, Value>> {
    value.get("attributes").and_then(Value::as_object)
}

fn str_attr<'a>(attributes: Option<&'a Map<String, Value>>, key: &str) -> Option<&'a str> {
    attributes?
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
}

fn str_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
}

fn method_from_span_name(name: Option<&str>) -> Option<&str> {
    let first = name?.split_whitespace().next()?;

    if matches!(first, "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD") {
        Some(first)
    } else {
        None
    }
}

fn shard_number(entity_id: &str) -> Option<&str> {
    entity_id.rsplit("-shard-").next().filter(|value| {
        !value.is_empty() && value.chars().all(|character| character.is_ascii_digit())
    })
}

fn service_name_from_shard_id(entity_id: &str) -> Option<&str> {
    entity_id
        .strip_prefix("shard:")?
        .split_once("-shard-")
        .map(|(service_name, _)| service_name)
}

fn resolved_entity(
    id: impl Into<String>,
    kind: EntityKind,
    from: Vec<String>,
    confidence: UnitInterval,
) -> ResolvedEntity {
    ResolvedEntity {
        id: id.into(),
        kind,
        from: dedupe_stable(from),
        confidence,
        discriminators: BTreeMap::new(),
        alternatives: Vec::new(),
        unresolved: false,
        missing_attributes: Vec::new(),
        estimated_share: None,
    }
}

fn insert_or_merge(entities: &mut BTreeMap<String, ResolvedEntity>, entity: ResolvedEntity) {
    let Some(existing) = entities.get_mut(&entity.id) else {
        entities.insert(entity.id.clone(), entity);
        return;
    };

    existing.from = dedupe_stable(existing.from.iter().chain(&entity.from).cloned().collect());
    if existing.kind == entity.kind && entity.confidence > existing.confidence {
        existing.confidence = entity.confidence;
    }

    for (key, value) in entity.discriminators {
        existing.discriminators.entry(key).or_insert(value);
    }

    for alternative in entity.alternatives {
        if !existing
            .alternatives
            .iter()
            .any(|existing| existing.id == alternative.id)
        {
            existing.alternatives.push(alternative);
        }
    }

    existing.unresolved |= entity.unresolved;
    existing.missing_attributes = dedupe_stable(
        existing
            .missing_attributes
            .iter()
            .chain(&entity.missing_attributes)
            .cloned()
            .collect(),
    );
    if existing.estimated_share.is_none() {
        existing.estimated_share = entity.estimated_share;
    }
}

fn dedupe_stable(values: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();

    for value in values {
        if !deduped.contains(&value) {
            deduped.push(value);
        }
    }

    deduped
}

fn is_false(value: &bool) -> bool {
    !*value
}
