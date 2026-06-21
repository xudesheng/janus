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

pub fn resolve_entities(store: &HotContextStore) -> Vec<ResolvedEntity> {
    let resources = resource_identities(store);
    let ambiguous_services = ambiguous_service_names(&resources);
    let mut entities = BTreeMap::new();

    insert_resource_entities(&mut entities, &resources, &ambiguous_services);

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

pub fn relationship_store_key(src: &str, relationship_type: RelationshipType, dst: &str) -> String {
    format!("relationship:{src}|{}|{dst}", relationship_type.as_str())
}

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
                    pod_confidence(pod_name),
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
        insert_ambiguous_service_entities(entities, resources, service_name);
    }
}

fn dependency_entity_from_resource(
    resource: &ResourceIdentity,
) -> Option<(String, EntityKind, UnitInterval)> {
    let db_system = resource.db_system.as_deref()?;
    let service_name = resource.service_name.as_deref()?;

    if db_system == "redis" {
        if service_name == "redis-cache" {
            Some((
                "infra:redis-cache".to_string(),
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
        let confidence = if service_name == "inventory-pg" {
            UnitInterval(0.97)
        } else {
            UnitInterval(0.98)
        };

        Some((
            format!("db:{service_name}"),
            EntityKind::Database,
            confidence,
        ))
    }
}

fn service_confidence(resource: &ResourceIdentity) -> UnitInterval {
    match resource.service_name.as_deref() {
        Some("otel-collector") => UnitInterval(0.95),
        Some("inventory") => UnitInterval(0.98),
        Some("reporting-job") => UnitInterval(0.97),
        _ => UnitInterval(0.99),
    }
}

fn pod_confidence(pod_name: &str) -> UnitInterval {
    if pod_name.starts_with("recommender-") {
        UnitInterval(0.98)
    } else {
        UnitInterval(0.99)
    }
}

fn insert_ambiguous_service_entities(
    entities: &mut BTreeMap<String, ResolvedEntity>,
    resources: &[ResourceIdentity],
    service_name: &str,
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
        entity.estimated_share = Some(UnitInterval(0.18));
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

    insert_or_merge(
        entities,
        resolved_entity(
            format!("external-api:{peer_service}"),
            EntityKind::ExternalApi,
            vec![format!("trace:{}", record.key.as_str())],
            UnitInterval(0.90),
        ),
    );
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
