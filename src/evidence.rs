use schemars::{
    JsonSchema,
    schema::{
        ArrayValidation, InstanceType, NumberValidation, RootSchema, Schema, SchemaObject,
        SingleOrVec,
    },
    schema_for,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt, fs, path::Path};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceItem {
    pub id: String,
    pub claim: String,
    pub kind: EvidenceKind,
    pub direction: EvidenceDirection,
    pub strength: UnitInterval,
    pub time_window: TimeWindow,
    pub entities: Vec<String>,
    pub source_refs: SourceRefs,
    pub freshness: EvidenceFreshness,
    pub missing_data: Vec<String>,
    pub token_cost: u32,
    pub privacy_scope: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub confidence: BTreeMap<String, UnitInterval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceBundle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hypothesis: Option<String>,
    pub time_window: TimeWindow,
    pub budget: EvidenceBudget,
    pub items: Vec<EvidenceItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TimeWindow {
    #[schemars(schema_with = "date_time_string_schema")]
    pub start: String,
    #[schemars(schema_with = "date_time_string_schema")]
    pub end: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceRef {
    pub signal: SourceSignal,
    pub r#ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourceRefs(pub Vec<SourceRef>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceBudget {
    pub max_items: u32,
    pub max_tokens: u32,
    pub tokens_used: u32,
    pub items_dropped: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    MetricAnomaly,
    TraceExemplar,
    LogCluster,
    ChangeEvent,
    DependencyEdge,
    ProfileHotspot,
    PreviousIncident,
    CounterEvidence,
    MissingData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceDirection {
    Supports,
    Weakens,
    Contradicts,
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceFreshness {
    Settled,
    Changing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceSignal {
    Trace,
    Metric,
    Log,
    Change,
    Profile,
    AnomalyWindow,
    LogPattern,
    PriorIncident,
    TelemetryGap,
    Entity,
    Relationship,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UnitInterval(pub f64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationErrors {
    errors: Vec<ValidationError>,
}

impl EvidenceBundle {
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = Vec::new();

        if self
            .question
            .as_ref()
            .is_none_or(|question| question.trim().is_empty())
            && self
                .hypothesis
                .as_ref()
                .is_none_or(|hypothesis| hypothesis.trim().is_empty())
        {
            push_error(
                &mut errors,
                "EvidenceBundle",
                "at least one of question or hypothesis must be present",
            );
        }

        if self.budget.tokens_used > self.budget.max_tokens {
            push_error(
                &mut errors,
                "EvidenceBundle.budget.tokens_used",
                "must be less than or equal to max_tokens",
            );
        }

        if self.items.len() > self.budget.max_items as usize {
            push_error(
                &mut errors,
                "EvidenceBundle.items",
                "item count must be less than or equal to budget.max_items",
            );
        }

        validate_time_window(&self.time_window, "EvidenceBundle.time_window", &mut errors);

        for (index, item) in self.items.iter().enumerate() {
            item.validate_with_path(&format!("EvidenceBundle.items[{index}]"), &mut errors);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors::new(errors))
        }
    }
}

impl EvidenceItem {
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = Vec::new();
        self.validate_with_path("EvidenceItem", &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors::new(errors))
        }
    }

    fn validate_with_path(&self, path: &str, errors: &mut Vec<ValidationError>) {
        if !self.strength.is_valid() {
            push_error(
                errors,
                &format!("{path}.strength"),
                "must be between 0 and 1",
            );
        }

        validate_time_window(&self.time_window, &format!("{path}.time_window"), errors);

        if self.source_refs.is_empty() {
            push_error(errors, &format!("{path}.source_refs"), "must be non-empty");
        }

        for (index, source_ref) in self.source_refs.iter().enumerate() {
            if source_ref.r#ref.trim().is_empty() {
                push_error(
                    errors,
                    &format!("{path}.source_refs[{index}].ref"),
                    "must be non-empty",
                );
            }
        }

        for (dimension, value) in &self.confidence {
            if dimension.trim().is_empty() {
                push_error(
                    errors,
                    &format!("{path}.confidence"),
                    "dimension names must be non-empty",
                );
            }

            if !value.is_valid() {
                push_error(
                    errors,
                    &format!("{path}.confidence.{dimension}"),
                    "must be between 0 and 1",
                );
            }
        }
    }
}

impl SourceRefs {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, SourceRef> {
        self.0.iter()
    }
}

impl UnitInterval {
    pub fn is_valid(self) -> bool {
        (0.0..=1.0).contains(&self.0)
    }
}

impl ValidationErrors {
    fn new(errors: Vec<ValidationError>) -> Self {
        Self { errors }
    }

    pub fn errors(&self) -> &[ValidationError] {
        &self.errors
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, error) in self.errors.iter().enumerate() {
            if index > 0 {
                write!(formatter, "; ")?;
            }
            write!(formatter, "{}: {}", error.path, error.message)?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

impl JsonSchema for SourceRefs {
    fn schema_name() -> String {
        "SourceRefs".to_string()
    }

    fn json_schema(generator: &mut schemars::r#gen::SchemaGenerator) -> Schema {
        Schema::Object(SchemaObject {
            instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Array))),
            array: Some(Box::new(ArrayValidation {
                items: Some(SingleOrVec::Single(Box::new(
                    generator.subschema_for::<SourceRef>(),
                ))),
                min_items: Some(1),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}

impl JsonSchema for UnitInterval {
    fn schema_name() -> String {
        "UnitInterval".to_string()
    }

    fn json_schema(_generator: &mut schemars::r#gen::SchemaGenerator) -> Schema {
        Schema::Object(SchemaObject {
            instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Number))),
            number: Some(Box::new(NumberValidation {
                minimum: Some(0.0),
                maximum: Some(1.0),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}

pub fn evidence_item_schema() -> RootSchema {
    schema_for!(EvidenceItem)
}

pub fn evidence_bundle_schema() -> RootSchema {
    schema_for!(EvidenceBundle)
}

pub fn write_schema_files(schema_dir: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let schema_dir = schema_dir.as_ref();
    fs::create_dir_all(schema_dir)?;

    write_schema_file(
        &schema_dir.join("evidence-item.schema.json"),
        &evidence_item_schema(),
    )?;
    write_schema_file(
        &schema_dir.join("evidence-bundle.schema.json"),
        &evidence_bundle_schema(),
    )?;

    Ok(())
}

fn write_schema_file(path: &Path, schema: &RootSchema) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(schema)?;
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

fn date_time_string_schema(_generator: &mut schemars::r#gen::SchemaGenerator) -> Schema {
    Schema::Object(SchemaObject {
        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
        format: Some("date-time".to_string()),
        ..Default::default()
    })
}

fn validate_time_window(time_window: &TimeWindow, path: &str, errors: &mut Vec<ValidationError>) {
    if time_window.start.trim().is_empty() {
        push_error(errors, &format!("{path}.start"), "must be non-empty");
    }

    if time_window.end.trim().is_empty() {
        push_error(errors, &format!("{path}.end"), "must be non-empty");
    }
}

fn push_error(errors: &mut Vec<ValidationError>, path: &str, message: &str) {
    errors.push(ValidationError {
        path: path.to_string(),
        message: message.to_string(),
    });
}
