use crate::evidence::UnitInterval;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

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
    Db,
    Queue,
    Cache,
    ExternalApi,
    Tenant,
    Infra,
    Deployment,
    Host,
    Container,
    Shard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RelationshipType {
    Calls,
    ReadsFrom,
    WritesTo,
    DependsOn,
    RunsOn,
    Owns,
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
            Self::Owns => "owns",
            Self::DeployedAs => "deployed-as",
            Self::Emits => "emits",
            Self::Retries => "retries",
            Self::FansOutTo => "fans-out-to",
            Self::SharesResourceWith => "shares-resource-with",
        }
    }
}

pub fn relationship_store_key(src: &str, relationship_type: RelationshipType, dst: &str) -> String {
    format!("relationship:{src}|{}|{dst}", relationship_type.as_str())
}

fn is_false(value: &bool) -> bool {
    !*value
}
