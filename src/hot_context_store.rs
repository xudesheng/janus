use crate::{
    evidence::{SourceRef, SourceSignal, TimeWindow},
    fixture_validation::FixtureCase,
    references::{
        RefCategory, categories_for_signal, metric_series_ref, resolve_ref_map, span_ref,
    },
};
use serde_json::{Map, Value};
use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceKey(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StoredRecordKind {
    Resource,
    Trace,
    Span,
    MetricSeries,
    Log,
    Change,
    PriorIncident,
    TelemetryGap,
    Entity,
    Relationship,
    AnomalyWindow,
    LogPattern,
    EvidenceItem,
    TimelineEvent,
    SuspectedCause,
    NextCheck,
    EntityContext,
    RelatedAnomaly,
    WindowComparison,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredRecord {
    pub key: SourceKey,
    pub kind: StoredRecordKind,
    pub time_window: Option<TimeWindow>,
    pub entities: Vec<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricSeriesKey {
    name: String,
    entity: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HotIngestEvent {
    Resource(Value),
    Trace(Value),
    Span {
        trace_id: String,
        payload: Value,
    },
    MetricPoint {
        series: MetricSeriesKey,
        payload: Value,
    },
    Log(Value),
    Change(Value),
    PriorIncident(Value),
    TelemetryGap(Value),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestOutcome {
    Inserted {
        kind: StoredRecordKind,
        key: SourceKey,
    },
    Updated {
        kind: StoredRecordKind,
        key: SourceKey,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceQuery {
    pub time_window: Option<TimeWindow>,
    pub entities: Vec<String>,
    pub kinds: Vec<StoredRecordKind>,
}

#[derive(Debug, Default)]
pub struct HotContextStore {
    records: Vec<StoredRecord>,
    primary_keys: BTreeMap<(StoredRecordKind, SourceKey), usize>,
    index: BTreeMap<String, Vec<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotStoreError {
    DuplicatePrimaryKey {
        fixture_id: Option<String>,
        file_path: Option<PathBuf>,
        json_path: Option<String>,
        kind: StoredRecordKind,
        key: SourceKey,
    },
    MissingField {
        fixture_id: String,
        file_path: PathBuf,
        json_path: String,
        field: &'static str,
    },
    InvalidShape {
        fixture_id: String,
        file_path: PathBuf,
        json_path: String,
        message: String,
    },
}

#[derive(Debug)]
pub enum SourceResolution<'a> {
    Found(&'a StoredRecord),
    Ambiguous {
        raw_ref: String,
        candidates: Vec<&'a StoredRecord>,
    },
    SignalMismatch {
        raw_ref: String,
        signal: SourceSignal,
        candidates: Vec<&'a StoredRecord>,
    },
    Missing {
        raw_ref: String,
    },
    Unsupported {
        raw_ref: String,
        signal: SourceSignal,
    },
}

type TimeWindowExtractor =
    fn(&str, &Path, &Value, &str) -> Result<Option<TimeWindow>, HotStoreError>;
type EntityExtractor = fn(&Value) -> Vec<String>;

#[derive(Debug, Clone, Copy)]
struct IdRecordSpec {
    key: &'static str,
    kind: StoredRecordKind,
    time_window_fn: TimeWindowExtractor,
    entities_fn: EntityExtractor,
    require_id: bool,
}

struct LoadedRecord {
    json_path: Option<String>,
    kind: StoredRecordKind,
    key: String,
    time_window: Option<TimeWindow>,
    entities: Vec<String>,
    payload: Value,
}

impl IdRecordSpec {
    fn required(
        key: &'static str,
        kind: StoredRecordKind,
        time_window_fn: TimeWindowExtractor,
        entities_fn: EntityExtractor,
    ) -> Self {
        Self {
            key,
            kind,
            time_window_fn,
            entities_fn,
            require_id: true,
        }
    }

    fn optional(
        key: &'static str,
        kind: StoredRecordKind,
        time_window_fn: TimeWindowExtractor,
        entities_fn: EntityExtractor,
    ) -> Self {
        Self {
            key,
            kind,
            time_window_fn,
            entities_fn,
            require_id: false,
        }
    }
}

impl SourceKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl MetricSeriesKey {
    pub fn new(name: impl Into<String>, entity: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            entity: entity.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn entity(&self) -> &str {
        &self.entity
    }

    fn source_key(&self) -> SourceKey {
        SourceKey::new(metric_series_ref(&self.name, &self.entity))
    }
}

impl HotContextStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_fixture_case(case: &FixtureCase) -> Result<Self, HotStoreError> {
        let mut store = Self::new();
        let fixture_id = case.registry_entry.id.as_str();
        let input_path = case.directory.join("input.json");
        let expected_path = case.directory.join("expected.json");

        store.load_input(fixture_id, &input_path, &case.input)?;
        store.load_expected(fixture_id, &expected_path, &case.expected)?;

        Ok(store)
    }

    pub fn insert_record(&mut self, record: StoredRecord) -> Result<(), HotStoreError> {
        self.insert_record_with_context(record, None, None, None)
    }

    pub fn ingest(&mut self, event: HotIngestEvent) -> Result<IngestOutcome, HotStoreError> {
        match event {
            HotIngestEvent::Resource(payload) => self.ingest_record(
                StoredRecordKind::Resource,
                required_str("ingest", ingest_path(), &payload, "$", "id")?.to_string(),
                None,
                entities_from_common_fields(&payload),
                payload,
            ),
            HotIngestEvent::Trace(payload) => {
                let trace_id = required_str("ingest", ingest_path(), &payload, "$", "trace_id")?;
                self.ingest_record(
                    StoredRecordKind::Trace,
                    trace_id.to_string(),
                    trace_time_window("ingest", ingest_path(), &payload, "$")?,
                    trace_entities(&payload),
                    payload,
                )
            }
            HotIngestEvent::Span { trace_id, payload } => {
                let span_id = required_str("ingest", ingest_path(), &payload, "$", "span_id")?;
                self.ingest_record(
                    StoredRecordKind::Span,
                    span_ref(&trace_id, span_id),
                    time_window_from_start_end("ingest", ingest_path(), &payload, "$")?,
                    entities_from_common_fields(&payload),
                    payload,
                )
            }
            HotIngestEvent::MetricPoint { series, payload } => {
                self.ingest_metric_point(series, payload)
            }
            HotIngestEvent::Log(payload) => self.ingest_record(
                StoredRecordKind::Log,
                required_str("ingest", ingest_path(), &payload, "$", "id")?.to_string(),
                time_window_from_t("ingest", ingest_path(), &payload, "$")?,
                entities_from_common_fields(&payload),
                payload,
            ),
            HotIngestEvent::Change(payload) => self.ingest_record(
                StoredRecordKind::Change,
                required_str("ingest", ingest_path(), &payload, "$", "id")?.to_string(),
                time_window_from_t("ingest", ingest_path(), &payload, "$")?,
                entities_from_common_fields(&payload),
                payload,
            ),
            HotIngestEvent::PriorIncident(payload) => self.ingest_record(
                StoredRecordKind::PriorIncident,
                required_str("ingest", ingest_path(), &payload, "$", "id")?.to_string(),
                time_window_from_first_seen("ingest", ingest_path(), &payload, "$")?,
                entities_from_common_fields(&payload),
                payload,
            ),
            HotIngestEvent::TelemetryGap(payload) => self.ingest_record(
                StoredRecordKind::TelemetryGap,
                required_str("ingest", ingest_path(), &payload, "$", "id")?.to_string(),
                time_window_from_start_end("ingest", ingest_path(), &payload, "$")?,
                entities_from_common_fields(&payload),
                payload,
            ),
        }
    }

    pub fn resolve_source_ref(&self, source_ref: &SourceRef) -> SourceResolution<'_> {
        let raw_ref = source_ref.r#ref.clone();
        let expected_categories = categories_for_signal(source_ref.signal);

        if expected_categories.is_empty() {
            return SourceResolution::Unsupported {
                raw_ref,
                signal: source_ref.signal,
            };
        }

        let candidates = self.lookup_records(&source_ref.r#ref);
        if candidates.is_empty() {
            return SourceResolution::Missing { raw_ref };
        }

        let matching = candidates
            .iter()
            .copied()
            .filter(|record| record.matches_any_category(expected_categories))
            .collect::<Vec<_>>();

        match matching.as_slice() {
            [] => SourceResolution::SignalMismatch {
                raw_ref,
                signal: source_ref.signal,
                candidates,
            },
            [record] => SourceResolution::Found(record),
            _ => SourceResolution::Ambiguous {
                raw_ref,
                candidates: matching,
            },
        }
    }

    pub fn resolve_scalar_ref(&self, scalar_ref: &str) -> SourceResolution<'_> {
        let candidates = self.lookup_records(scalar_ref);

        match candidates.as_slice() {
            [] => SourceResolution::Missing {
                raw_ref: scalar_ref.to_string(),
            },
            [record] => SourceResolution::Found(record),
            _ => SourceResolution::Ambiguous {
                raw_ref: scalar_ref.to_string(),
                candidates,
            },
        }
    }

    pub fn select(&self, query: SourceQuery) -> Vec<&StoredRecord> {
        self.records
            .iter()
            .filter(|record| record_matches_query(record, &query))
            .collect()
    }

    pub fn records(&self) -> &[StoredRecord] {
        &self.records
    }

    pub fn raw_source_records(&self) -> impl Iterator<Item = &StoredRecord> + '_ {
        self.records
            .iter()
            .filter(|record| record.kind.is_raw_source())
    }

    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    fn load_input(
        &mut self,
        fixture_id: &str,
        input_path: &Path,
        input: &Value,
    ) -> Result<(), HotStoreError> {
        self.load_resources(fixture_id, input_path, input)?;
        self.load_traces(fixture_id, input_path, input)?;
        self.load_metrics(fixture_id, input_path, input)?;
        self.load_id_records(
            fixture_id,
            input_path,
            input,
            IdRecordSpec::required(
                "logs",
                StoredRecordKind::Log,
                time_window_from_t,
                entities_from_common_fields,
            ),
        )?;
        self.load_id_records(
            fixture_id,
            input_path,
            input,
            IdRecordSpec::required(
                "changes",
                StoredRecordKind::Change,
                time_window_from_t,
                entities_from_common_fields,
            ),
        )?;
        self.load_id_records(
            fixture_id,
            input_path,
            input,
            IdRecordSpec::required(
                "prior_incidents",
                StoredRecordKind::PriorIncident,
                time_window_from_first_seen,
                entities_from_common_fields,
            ),
        )?;
        self.load_id_records(
            fixture_id,
            input_path,
            input,
            IdRecordSpec::required(
                "telemetry_gaps",
                StoredRecordKind::TelemetryGap,
                time_window_from_start_end,
                entities_from_common_fields,
            ),
        )?;

        Ok(())
    }

    fn load_expected(
        &mut self,
        fixture_id: &str,
        expected_path: &Path,
        expected: &Value,
    ) -> Result<(), HotStoreError> {
        self.load_id_records(
            fixture_id,
            expected_path,
            expected,
            IdRecordSpec::required(
                "entities",
                StoredRecordKind::Entity,
                no_time_window,
                entities_from_entity_record,
            ),
        )?;
        self.load_id_records(
            fixture_id,
            expected_path,
            expected,
            IdRecordSpec::optional(
                "relationships",
                StoredRecordKind::Relationship,
                no_time_window,
                entities_from_relationship_record,
            ),
        )?;
        self.load_id_records(
            fixture_id,
            expected_path,
            expected,
            IdRecordSpec::required(
                "anomaly_windows",
                StoredRecordKind::AnomalyWindow,
                time_window_from_start_end,
                entities_from_common_fields,
            ),
        )?;
        self.load_id_records(
            fixture_id,
            expected_path,
            expected,
            IdRecordSpec::required(
                "log_patterns",
                StoredRecordKind::LogPattern,
                time_window_from_first_last_seen,
                entities_from_common_fields,
            ),
        )?;
        self.load_evidence_items(fixture_id, expected_path, expected)?;

        Ok(())
    }

    fn load_resources(
        &mut self,
        fixture_id: &str,
        input_path: &Path,
        input: &Value,
    ) -> Result<(), HotStoreError> {
        let Some(resources) = array_at(fixture_id, input_path, input, "resources")? else {
            return Ok(());
        };

        for (index, resource) in resources.iter().enumerate() {
            let json_path = format!("$.resources[{index}]");
            let id = required_str(fixture_id, input_path, resource, &json_path, "id")?;
            self.insert_loaded_record(
                fixture_id,
                input_path,
                LoadedRecord {
                    json_path: Some(json_path),
                    kind: StoredRecordKind::Resource,
                    key: id.to_string(),
                    time_window: None,
                    entities: Vec::new(),
                    payload: resource.clone(),
                },
            )?;
        }

        Ok(())
    }

    fn load_traces(
        &mut self,
        fixture_id: &str,
        input_path: &Path,
        input: &Value,
    ) -> Result<(), HotStoreError> {
        let Some(traces) = array_at(fixture_id, input_path, input, "traces")? else {
            return Ok(());
        };

        for (trace_index, trace) in traces.iter().enumerate() {
            let trace_path = format!("$.traces[{trace_index}]");
            let trace_id = required_str(fixture_id, input_path, trace, &trace_path, "trace_id")?;
            let trace_time_window = trace_time_window(fixture_id, input_path, trace, &trace_path)?;
            let trace_entities = trace_entities(trace);

            self.insert_loaded_record(
                fixture_id,
                input_path,
                LoadedRecord {
                    json_path: Some(trace_path.clone()),
                    kind: StoredRecordKind::Trace,
                    key: trace_id.to_string(),
                    time_window: trace_time_window,
                    entities: trace_entities,
                    payload: trace.clone(),
                },
            )?;

            let Some(spans) = trace.get("spans").and_then(Value::as_array) else {
                continue;
            };

            for (span_index, span) in spans.iter().enumerate() {
                let span_path = format!("{trace_path}.spans[{span_index}]");
                let span_id = required_str(fixture_id, input_path, span, &span_path, "span_id")?;
                let key = span_ref(trace_id, span_id);

                self.insert_loaded_record(
                    fixture_id,
                    input_path,
                    LoadedRecord {
                        json_path: Some(span_path.clone()),
                        kind: StoredRecordKind::Span,
                        key,
                        time_window: time_window_from_start_end(
                            fixture_id, input_path, span, &span_path,
                        )?,
                        entities: entities_from_common_fields(span),
                        payload: span.clone(),
                    },
                )?;
            }
        }

        Ok(())
    }

    fn load_metrics(
        &mut self,
        fixture_id: &str,
        input_path: &Path,
        input: &Value,
    ) -> Result<(), HotStoreError> {
        let Some(metrics) = array_at(fixture_id, input_path, input, "metrics")? else {
            return Ok(());
        };

        for (index, metric) in metrics.iter().enumerate() {
            let json_path = format!("$.metrics[{index}]");
            let name = required_str(fixture_id, input_path, metric, &json_path, "name")?;
            let entity = required_str(fixture_id, input_path, metric, &json_path, "entity")?;
            let key = metric_series_ref(name, entity);

            self.insert_loaded_record(
                fixture_id,
                input_path,
                LoadedRecord {
                    json_path: Some(json_path.clone()),
                    kind: StoredRecordKind::MetricSeries,
                    key,
                    time_window: metric_time_window(fixture_id, input_path, metric, &json_path)?,
                    entities: entities_from_common_fields(metric),
                    payload: metric.clone(),
                },
            )?;
        }

        Ok(())
    }

    fn load_id_records(
        &mut self,
        fixture_id: &str,
        file_path: &Path,
        root: &Value,
        spec: IdRecordSpec,
    ) -> Result<(), HotStoreError> {
        let Some(values) = array_at(fixture_id, file_path, root, spec.key)? else {
            return Ok(());
        };

        for (index, value) in values.iter().enumerate() {
            let json_path = format!("$.{}[{index}]", spec.key);
            let id = match value.get("id").and_then(Value::as_str) {
                Some(id) if !id.trim().is_empty() => id,
                _ if spec.require_id => {
                    return Err(HotStoreError::MissingField {
                        fixture_id: fixture_id.to_string(),
                        file_path: file_path.to_path_buf(),
                        json_path,
                        field: "id",
                    });
                }
                _ => continue,
            };

            self.insert_loaded_record(
                fixture_id,
                file_path,
                LoadedRecord {
                    json_path: Some(json_path.clone()),
                    kind: spec.kind,
                    key: id.to_string(),
                    time_window: (spec.time_window_fn)(fixture_id, file_path, value, &json_path)?,
                    entities: (spec.entities_fn)(value),
                    payload: value.clone(),
                },
            )?;
        }

        Ok(())
    }

    fn load_evidence_items(
        &mut self,
        fixture_id: &str,
        expected_path: &Path,
        expected: &Value,
    ) -> Result<(), HotStoreError> {
        let Some(items) = expected
            .pointer("/evidence_bundle/items")
            .and_then(Value::as_array)
        else {
            return Ok(());
        };

        for (index, item) in items.iter().enumerate() {
            let json_path = format!("$.evidence_bundle.items[{index}]");
            let id = required_str(fixture_id, expected_path, item, &json_path, "id")?;

            self.insert_loaded_record(
                fixture_id,
                expected_path,
                LoadedRecord {
                    json_path: Some(json_path.clone()),
                    kind: StoredRecordKind::EvidenceItem,
                    key: id.to_string(),
                    time_window: time_window_from_nested_time_window(
                        fixture_id,
                        expected_path,
                        item,
                        &json_path,
                    )?,
                    entities: entities_from_common_fields(item),
                    payload: item.clone(),
                },
            )?;
        }

        Ok(())
    }

    fn insert_loaded_record(
        &mut self,
        fixture_id: &str,
        file_path: &Path,
        loaded: LoadedRecord,
    ) -> Result<(), HotStoreError> {
        let record = StoredRecord {
            key: SourceKey::new(loaded.key),
            kind: loaded.kind,
            time_window: loaded.time_window,
            entities: loaded.entities,
            payload: loaded.payload,
        };

        self.insert_record_with_context(
            record,
            Some(fixture_id.to_string()),
            Some(file_path.to_path_buf()),
            loaded.json_path,
        )
    }

    fn ingest_record(
        &mut self,
        kind: StoredRecordKind,
        key: String,
        time_window: Option<TimeWindow>,
        entities: Vec<String>,
        payload: Value,
    ) -> Result<IngestOutcome, HotStoreError> {
        let record = StoredRecord {
            key: SourceKey::new(key),
            kind,
            time_window,
            entities,
            payload,
        };
        let outcome = IngestOutcome::Inserted {
            kind: record.kind,
            key: record.key.clone(),
        };

        self.insert_record(record)?;

        Ok(outcome)
    }

    fn ingest_metric_point(
        &mut self,
        series: MetricSeriesKey,
        payload: Value,
    ) -> Result<IngestOutcome, HotStoreError> {
        let key = series.source_key();
        let (metadata, point) = metric_point_parts(&series, &payload)?;
        let point_window = point_time_window(&point)?;
        let primary_key = (StoredRecordKind::MetricSeries, key.clone());

        if let Some(index) = self.primary_keys.get(&primary_key).copied() {
            let record = &mut self.records[index];
            let existing_metadata = metric_series_metadata(&record.payload)?;

            if existing_metadata != metadata {
                return Err(HotStoreError::DuplicatePrimaryKey {
                    fixture_id: Some("ingest".to_string()),
                    file_path: Some(ingest_path().to_path_buf()),
                    json_path: Some("$".to_string()),
                    kind: StoredRecordKind::MetricSeries,
                    key,
                });
            }

            let points = record
                .payload
                .get_mut("points")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| HotStoreError::InvalidShape {
                    fixture_id: "ingest".to_string(),
                    file_path: ingest_path().to_path_buf(),
                    json_path: "$.points".to_string(),
                    message: "must be an array".to_string(),
                })?;
            points.push(point);
            record.time_window = Some(merge_time_window(record.time_window.clone(), point_window));

            return Ok(IngestOutcome::Updated {
                kind: StoredRecordKind::MetricSeries,
                key,
            });
        }

        let mut record_payload = Value::Object(metadata);
        record_payload["points"] = Value::Array(vec![point]);
        let record = StoredRecord {
            key: key.clone(),
            kind: StoredRecordKind::MetricSeries,
            time_window: Some(point_window),
            entities: vec![series.entity().to_string()],
            payload: record_payload,
        };

        self.insert_record(record)?;

        Ok(IngestOutcome::Inserted {
            kind: StoredRecordKind::MetricSeries,
            key,
        })
    }

    fn insert_record_with_context(
        &mut self,
        record: StoredRecord,
        fixture_id: Option<String>,
        file_path: Option<PathBuf>,
        json_path: Option<String>,
    ) -> Result<(), HotStoreError> {
        let primary_key = (record.kind, record.key.clone());

        if self.primary_keys.contains_key(&primary_key) {
            return Err(HotStoreError::DuplicatePrimaryKey {
                fixture_id,
                file_path,
                json_path,
                kind: record.kind,
                key: record.key,
            });
        }

        let index = self.records.len();
        self.index
            .entry(record.key.as_str().to_string())
            .or_default()
            .push(index);
        self.primary_keys.insert(primary_key, index);
        self.records.push(record);

        Ok(())
    }

    fn lookup_records(&self, raw_ref: &str) -> Vec<&StoredRecord> {
        resolve_ref_map(&self.index, raw_ref)
            .map(|indices| indices.iter().map(|index| &self.records[*index]).collect())
            .unwrap_or_default()
    }
}

impl StoredRecord {
    fn matches_any_category(&self, categories: &[RefCategory]) -> bool {
        self.kind
            .ref_category()
            .is_some_and(|category| categories.contains(&category))
    }
}

impl StoredRecordKind {
    pub fn is_raw_source(self) -> bool {
        matches!(
            self,
            StoredRecordKind::Resource
                | StoredRecordKind::Trace
                | StoredRecordKind::Span
                | StoredRecordKind::MetricSeries
                | StoredRecordKind::Log
                | StoredRecordKind::Change
                | StoredRecordKind::PriorIncident
                | StoredRecordKind::TelemetryGap
        )
    }

    fn ref_category(self) -> Option<RefCategory> {
        match self {
            StoredRecordKind::Resource => Some(RefCategory::Resource),
            StoredRecordKind::Trace => Some(RefCategory::Trace),
            StoredRecordKind::Span => Some(RefCategory::Span),
            StoredRecordKind::MetricSeries => Some(RefCategory::Metric),
            StoredRecordKind::Log => Some(RefCategory::Log),
            StoredRecordKind::Change => Some(RefCategory::Change),
            StoredRecordKind::PriorIncident => Some(RefCategory::PriorIncident),
            StoredRecordKind::TelemetryGap => Some(RefCategory::TelemetryGap),
            StoredRecordKind::Entity => Some(RefCategory::Entity),
            StoredRecordKind::Relationship => Some(RefCategory::Relationship),
            StoredRecordKind::AnomalyWindow => Some(RefCategory::AnomalyWindow),
            StoredRecordKind::LogPattern => Some(RefCategory::LogPattern),
            StoredRecordKind::EvidenceItem => Some(RefCategory::EvidenceItem),
            StoredRecordKind::TimelineEvent
            | StoredRecordKind::SuspectedCause
            | StoredRecordKind::NextCheck
            | StoredRecordKind::EntityContext
            | StoredRecordKind::RelatedAnomaly
            | StoredRecordKind::WindowComparison => None,
        }
    }
}

impl fmt::Display for SourceKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl fmt::Display for StoredRecordKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            StoredRecordKind::Resource => "resource",
            StoredRecordKind::Trace => "trace",
            StoredRecordKind::Span => "span",
            StoredRecordKind::MetricSeries => "metric_series",
            StoredRecordKind::Log => "log",
            StoredRecordKind::Change => "change",
            StoredRecordKind::PriorIncident => "prior_incident",
            StoredRecordKind::TelemetryGap => "telemetry_gap",
            StoredRecordKind::Entity => "entity",
            StoredRecordKind::Relationship => "relationship",
            StoredRecordKind::AnomalyWindow => "anomaly_window",
            StoredRecordKind::LogPattern => "log_pattern",
            StoredRecordKind::EvidenceItem => "evidence_item",
            StoredRecordKind::TimelineEvent => "timeline_event",
            StoredRecordKind::SuspectedCause => "suspected_cause",
            StoredRecordKind::NextCheck => "next_check",
            StoredRecordKind::EntityContext => "entity_context",
            StoredRecordKind::RelatedAnomaly => "related_anomaly",
            StoredRecordKind::WindowComparison => "window_comparison",
        };

        write!(formatter, "{name}")
    }
}

impl fmt::Display for HotStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HotStoreError::DuplicatePrimaryKey {
                fixture_id,
                file_path,
                json_path,
                kind,
                key,
            } => write!(
                formatter,
                "{}duplicate primary key `{key}` for {kind}{}{}",
                fixture_prefix(fixture_id.as_deref()),
                file_prefix(file_path.as_deref()),
                json_path_prefix(json_path.as_deref())
            ),
            HotStoreError::MissingField {
                fixture_id,
                file_path,
                json_path,
                field,
            } => write!(
                formatter,
                "fixture `{fixture_id}` {} {json_path}: missing required field `{field}`",
                file_path.display()
            ),
            HotStoreError::InvalidShape {
                fixture_id,
                file_path,
                json_path,
                message,
            } => write!(
                formatter,
                "fixture `{fixture_id}` {} {json_path}: {message}",
                file_path.display()
            ),
        }
    }
}

impl std::error::Error for HotStoreError {}

fn fixture_prefix(fixture_id: Option<&str>) -> String {
    fixture_id
        .map(|fixture_id| format!("fixture `{fixture_id}`: "))
        .unwrap_or_default()
}

fn file_prefix(file_path: Option<&Path>) -> String {
    file_path
        .map(|file_path| format!(" in {}", file_path.display()))
        .unwrap_or_default()
}

fn json_path_prefix(json_path: Option<&str>) -> String {
    json_path
        .map(|json_path| format!(" at {json_path}"))
        .unwrap_or_default()
}

fn array_at<'a>(
    fixture_id: &str,
    file_path: &Path,
    root: &'a Value,
    key: &'static str,
) -> Result<Option<&'a Vec<Value>>, HotStoreError> {
    match root.get(key) {
        Some(Value::Array(values)) => Ok(Some(values)),
        Some(_) => Err(HotStoreError::InvalidShape {
            fixture_id: fixture_id.to_string(),
            file_path: file_path.to_path_buf(),
            json_path: format!("$.{key}"),
            message: "must be an array".to_string(),
        }),
        None => Ok(None),
    }
}

fn required_str<'a>(
    fixture_id: &str,
    file_path: &Path,
    value: &'a Value,
    json_path: &str,
    field: &'static str,
) -> Result<&'a str, HotStoreError> {
    match value.get(field).and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(HotStoreError::MissingField {
            fixture_id: fixture_id.to_string(),
            file_path: file_path.to_path_buf(),
            json_path: json_path.to_string(),
            field,
        }),
    }
}

fn optional_str_field<'a>(
    fixture_id: &str,
    file_path: &Path,
    value: &'a Value,
    json_path: &str,
    field: &'static str,
) -> Result<Option<&'a str>, HotStoreError> {
    match value.get(field) {
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(Some(value)),
        Some(Value::String(_)) => Ok(None),
        Some(Value::Null) => Ok(None),
        Some(_) => Err(HotStoreError::InvalidShape {
            fixture_id: fixture_id.to_string(),
            file_path: file_path.to_path_buf(),
            json_path: format!("{json_path}.{field}"),
            message: "must be a string".to_string(),
        }),
        None => Ok(None),
    }
}

fn no_time_window(
    fixture_id: &str,
    file_path: &Path,
    value: &Value,
    json_path: &str,
) -> Result<Option<TimeWindow>, HotStoreError> {
    ensure_object(fixture_id, file_path, value, json_path)?;
    Ok(None)
}

fn time_window_from_t(
    fixture_id: &str,
    file_path: &Path,
    value: &Value,
    json_path: &str,
) -> Result<Option<TimeWindow>, HotStoreError> {
    optional_str_field(fixture_id, file_path, value, json_path, "t").map(|value| {
        value.map(|timestamp| TimeWindow {
            start: timestamp.to_string(),
            end: timestamp.to_string(),
        })
    })
}

fn time_window_from_first_seen(
    fixture_id: &str,
    file_path: &Path,
    value: &Value,
    json_path: &str,
) -> Result<Option<TimeWindow>, HotStoreError> {
    optional_str_field(fixture_id, file_path, value, json_path, "first_seen").map(|value| {
        value.map(|timestamp| TimeWindow {
            start: timestamp.to_string(),
            end: timestamp.to_string(),
        })
    })
}

fn time_window_from_start_end(
    fixture_id: &str,
    file_path: &Path,
    value: &Value,
    json_path: &str,
) -> Result<Option<TimeWindow>, HotStoreError> {
    time_window_from_fields(fixture_id, file_path, value, json_path, "start", "end")
}

fn time_window_from_first_last_seen(
    fixture_id: &str,
    file_path: &Path,
    value: &Value,
    json_path: &str,
) -> Result<Option<TimeWindow>, HotStoreError> {
    time_window_from_fields(
        fixture_id,
        file_path,
        value,
        json_path,
        "first_seen",
        "last_seen",
    )
}

fn time_window_from_nested_time_window(
    fixture_id: &str,
    file_path: &Path,
    value: &Value,
    json_path: &str,
) -> Result<Option<TimeWindow>, HotStoreError> {
    let Some(time_window) = value.get("time_window") else {
        return Ok(None);
    };

    time_window_from_start_end(
        fixture_id,
        file_path,
        time_window,
        &format!("{json_path}.time_window"),
    )
}

fn time_window_from_fields(
    fixture_id: &str,
    file_path: &Path,
    value: &Value,
    json_path: &str,
    start_field: &'static str,
    end_field: &'static str,
) -> Result<Option<TimeWindow>, HotStoreError> {
    let start = optional_str_field(fixture_id, file_path, value, json_path, start_field)?;
    let end = optional_str_field(fixture_id, file_path, value, json_path, end_field)?;

    match (start, end) {
        (Some(start), Some(end)) => Ok(Some(TimeWindow {
            start: start.to_string(),
            end: end.to_string(),
        })),
        (None, None) => Ok(None),
        _ => Err(HotStoreError::InvalidShape {
            fixture_id: fixture_id.to_string(),
            file_path: file_path.to_path_buf(),
            json_path: json_path.to_string(),
            message: format!("{start_field} and {end_field} must both be present"),
        }),
    }
}

fn trace_time_window(
    fixture_id: &str,
    file_path: &Path,
    trace: &Value,
    json_path: &str,
) -> Result<Option<TimeWindow>, HotStoreError> {
    let Some(spans) = trace.get("spans").and_then(Value::as_array) else {
        return Ok(None);
    };

    let mut starts = Vec::new();
    let mut ends = Vec::new();

    for (index, span) in spans.iter().enumerate() {
        let span_path = format!("{json_path}.spans[{index}]");
        if let Some(window) = time_window_from_start_end(fixture_id, file_path, span, &span_path)? {
            starts.push(window.start);
            ends.push(window.end);
        }
    }

    if starts.is_empty() || ends.is_empty() {
        return Ok(None);
    }

    starts.sort();
    ends.sort();

    Ok(Some(TimeWindow {
        start: starts[0].clone(),
        end: ends[ends.len() - 1].clone(),
    }))
}

fn metric_time_window(
    fixture_id: &str,
    file_path: &Path,
    metric: &Value,
    json_path: &str,
) -> Result<Option<TimeWindow>, HotStoreError> {
    let Some(points) = metric.get("points").and_then(Value::as_array) else {
        return Ok(None);
    };

    let mut timestamps = Vec::new();

    for (index, point) in points.iter().enumerate() {
        if let Some(timestamp) = optional_str_field(
            fixture_id,
            file_path,
            point,
            &format!("{json_path}.points[{index}]"),
            "t",
        )? {
            timestamps.push(timestamp.to_string());
        }
    }

    if timestamps.is_empty() {
        return Ok(None);
    }

    timestamps.sort();

    Ok(Some(TimeWindow {
        start: timestamps[0].clone(),
        end: timestamps[timestamps.len() - 1].clone(),
    }))
}

fn metric_point_parts(
    series: &MetricSeriesKey,
    payload: &Value,
) -> Result<(Map<String, Value>, Value), HotStoreError> {
    let object = payload
        .as_object()
        .ok_or_else(|| HotStoreError::InvalidShape {
            fixture_id: "ingest".to_string(),
            file_path: ingest_path().to_path_buf(),
            json_path: "$".to_string(),
            message: "metric point payload must be an object".to_string(),
        })?;
    let point = object
        .get("point")
        .cloned()
        .ok_or_else(|| HotStoreError::MissingField {
            fixture_id: "ingest".to_string(),
            file_path: ingest_path().to_path_buf(),
            json_path: "$".to_string(),
            field: "point",
        })?;
    let mut metadata = object.clone();
    metadata.remove("point");
    ensure_metric_identity(series, &metadata)?;

    Ok((metadata, point))
}

fn metric_series_metadata(payload: &Value) -> Result<Map<String, Value>, HotStoreError> {
    let object = payload
        .as_object()
        .ok_or_else(|| HotStoreError::InvalidShape {
            fixture_id: "ingest".to_string(),
            file_path: ingest_path().to_path_buf(),
            json_path: "$".to_string(),
            message: "metric series payload must be an object".to_string(),
        })?;
    let mut metadata = object.clone();
    metadata.remove("points");

    Ok(metadata)
}

fn ensure_metric_identity(
    series: &MetricSeriesKey,
    metadata: &Map<String, Value>,
) -> Result<(), HotStoreError> {
    match metadata.get("name").and_then(Value::as_str) {
        Some(name) if name == series.name() => {}
        Some(_) => {
            return Err(HotStoreError::InvalidShape {
                fixture_id: "ingest".to_string(),
                file_path: ingest_path().to_path_buf(),
                json_path: "$.name".to_string(),
                message: "must match metric series name".to_string(),
            });
        }
        None => {
            return Err(HotStoreError::MissingField {
                fixture_id: "ingest".to_string(),
                file_path: ingest_path().to_path_buf(),
                json_path: "$".to_string(),
                field: "name",
            });
        }
    }

    match metadata.get("entity").and_then(Value::as_str) {
        Some(entity) if entity == series.entity() => Ok(()),
        Some(_) => Err(HotStoreError::InvalidShape {
            fixture_id: "ingest".to_string(),
            file_path: ingest_path().to_path_buf(),
            json_path: "$.entity".to_string(),
            message: "must match metric series entity".to_string(),
        }),
        None => Err(HotStoreError::MissingField {
            fixture_id: "ingest".to_string(),
            file_path: ingest_path().to_path_buf(),
            json_path: "$".to_string(),
            field: "entity",
        }),
    }
}

fn point_time_window(point: &Value) -> Result<TimeWindow, HotStoreError> {
    let timestamp = required_str("ingest", ingest_path(), point, "$.point", "t")?;

    Ok(TimeWindow {
        start: timestamp.to_string(),
        end: timestamp.to_string(),
    })
}

fn merge_time_window(existing: Option<TimeWindow>, next: TimeWindow) -> TimeWindow {
    match existing {
        Some(existing) => TimeWindow {
            start: existing.start.min(next.start),
            end: existing.end.max(next.end),
        },
        None => next,
    }
}

fn ensure_object(
    fixture_id: &str,
    file_path: &Path,
    value: &Value,
    json_path: &str,
) -> Result<(), HotStoreError> {
    if value.is_object() {
        Ok(())
    } else {
        Err(HotStoreError::InvalidShape {
            fixture_id: fixture_id.to_string(),
            file_path: file_path.to_path_buf(),
            json_path: json_path.to_string(),
            message: "must be an object".to_string(),
        })
    }
}

fn trace_entities(trace: &Value) -> Vec<String> {
    let mut entities = Vec::new();

    if let Some(spans) = trace.get("spans").and_then(Value::as_array) {
        for span in spans {
            push_str_field(&mut entities, span, "resource");
            push_str_field(&mut entities, span, "entity");
        }
    }

    dedupe_stable(entities)
}

fn entities_from_common_fields(value: &Value) -> Vec<String> {
    let mut entities = Vec::new();

    push_str_field(&mut entities, value, "entity");
    push_str_field(&mut entities, value, "resource");
    push_str_array_field(&mut entities, value, "entities");
    push_str_array_field(&mut entities, value, "affected_entities");

    dedupe_stable(entities)
}

fn entities_from_entity_record(value: &Value) -> Vec<String> {
    value
        .get("id")
        .and_then(Value::as_str)
        .map(|id| vec![id.to_string()])
        .unwrap_or_default()
}

fn entities_from_relationship_record(value: &Value) -> Vec<String> {
    let mut entities = Vec::new();

    push_str_field(&mut entities, value, "src");
    push_str_field(&mut entities, value, "dst");

    dedupe_stable(entities)
}

fn push_str_field(entities: &mut Vec<String>, value: &Value, field: &str) {
    if let Some(entity) = value.get(field).and_then(Value::as_str)
        && !entity.trim().is_empty()
    {
        entities.push(entity.to_string());
    }
}

fn push_str_array_field(entities: &mut Vec<String>, value: &Value, field: &str) {
    if let Some(values) = value.get(field).and_then(Value::as_array) {
        for value in values {
            if let Some(entity) = value.as_str()
                && !entity.trim().is_empty()
            {
                entities.push(entity.to_string());
            }
        }
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

fn record_matches_query(record: &StoredRecord, query: &SourceQuery) -> bool {
    query
        .time_window
        .as_ref()
        .is_none_or(|query_window| record_overlaps_window(record, query_window))
        && (query.entities.is_empty()
            || record
                .entities
                .iter()
                .any(|entity| query.entities.contains(entity)))
        && (query.kinds.is_empty() || query.kinds.contains(&record.kind))
}

fn record_overlaps_window(record: &StoredRecord, query_window: &TimeWindow) -> bool {
    record
        .time_window
        .as_ref()
        .is_some_and(|record_window| windows_overlap(record_window, query_window))
}

fn windows_overlap(left: &TimeWindow, right: &TimeWindow) -> bool {
    left.start.as_str() <= right.end.as_str() && right.start.as_str() <= left.end.as_str()
}

fn ingest_path() -> &'static Path {
    Path::new("<ingest>")
}
