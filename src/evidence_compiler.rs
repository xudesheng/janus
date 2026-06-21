use crate::{
    derived_context::{DerivedContext, TimelineMarker, WindowDelta},
    entity_context::{RelationshipType, ResolvedRelationship},
    evidence::{
        EvidenceBudget, EvidenceBundle, EvidenceDirection, EvidenceFreshness, EvidenceItem,
        EvidenceKind, SourceRef, SourceRefs, SourceSignal, TimeWindow, UnitInterval,
    },
    fixture_validation::FixtureCase,
    hot_context_store::{
        HotContextStore, HotStoreError, SourceKey, SourceResolution, StoredRecord, StoredRecordKind,
    },
    query::EvidenceQuery,
    references::metric_series_ref,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

#[derive(Debug, Clone, Copy)]
pub struct EvidenceCompilerInput<'a> {
    pub query: &'a EvidenceQuery,
    pub store: &'a HotContextStore,
    pub derived: &'a DerivedContext,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceCandidate {
    pub candidate_id: String,
    pub item: EvidenceItem,
    pub source: EvidenceCandidateSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceCandidateSource {
    MetricAnomaly,
    LogPattern,
    ChangeEvent,
    TraceExemplar,
    DependencyEdge,
    PreviousIncident,
    MissingData,
    CounterEvidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceCompilation {
    pub bundle: EvidenceBundle,
    pub suspected_causes: Vec<SuspectedCause>,
    pub next_checks: Vec<NextCheck>,
    pub report: EvidenceCompilationReport,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvidenceCompilationReport {
    pub generated_items: usize,
    pub selected_items: usize,
    pub dropped_items: Vec<DroppedEvidenceItem>,
    pub requirement_failures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DroppedEvidenceItem {
    pub id: String,
    pub reason: String,
    pub token_cost: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuspectedCause {
    pub rank: u32,
    pub entity: String,
    pub hypothesis: String,
    pub score: UnitInterval,
    #[serde(default)]
    pub reasons: Vec<String>,
    #[serde(default)]
    pub supporting: Vec<String>,
    #[serde(default)]
    pub counter: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trap_note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NextCheck {
    pub action: String,
    pub rationale: String,
    pub expected_signal: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EvidenceCompilationComparison {
    pub bundle_mismatches: Vec<EvidenceCompilationFieldMismatch>,
    pub item_order_mismatches: Vec<EvidenceItemOrderMismatch>,
    pub item_mismatches: Vec<EvidenceCompilationFieldMismatch>,
    pub missing_suspected_causes: Vec<u32>,
    pub extra_suspected_causes: Vec<u32>,
    pub suspected_cause_mismatches: Vec<EvidenceCompilationFieldMismatch>,
    pub missing_next_checks: Vec<usize>,
    pub extra_next_checks: Vec<usize>,
    pub next_check_mismatches: Vec<EvidenceCompilationFieldMismatch>,
    pub text_differences: Vec<EvidenceCompilationFieldMismatch>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceCompilationFieldMismatch {
    pub artifact: String,
    pub field: String,
    pub expected: Value,
    pub actual: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceItemOrderMismatch {
    pub index: usize,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

#[derive(Debug)]
pub enum EvidenceCompilationGoldError {
    MissingArtifact {
        fixture_id: String,
        artifact: &'static str,
    },
    ParseArtifact {
        fixture_id: String,
        artifact: &'static str,
        source: serde_json::Error,
    },
}

#[derive(Debug)]
pub enum EvidenceCompileError {
    TokenEstimate {
        item_id: String,
        source: serde_json::Error,
    },
    TokenCostOverflow {
        item_id: String,
        bytes: usize,
    },
    RequirementUnsatisfied {
        requirement: &'static str,
        message: String,
    },
}

#[derive(Serialize)]
struct EvidenceItemTokenPayload<'a> {
    id: &'a str,
    claim: &'a str,
    kind: EvidenceKind,
    direction: EvidenceDirection,
    strength: UnitInterval,
    time_window: &'a TimeWindow,
    entities: &'a Vec<String>,
    source_refs: &'a SourceRefs,
    freshness: EvidenceFreshness,
    missing_data: &'a Vec<String>,
    privacy_scope: &'a str,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    confidence: &'a BTreeMap<String, UnitInterval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: &'a Option<String>,
}

impl EvidenceCompilation {
    pub fn from_bundle(
        bundle: EvidenceBundle,
        suspected_causes: Vec<SuspectedCause>,
        next_checks: Vec<NextCheck>,
    ) -> Self {
        let selected_items = bundle.items.len();

        Self {
            bundle,
            suspected_causes,
            next_checks,
            report: EvidenceCompilationReport {
                generated_items: selected_items,
                selected_items,
                dropped_items: Vec::new(),
                requirement_failures: Vec::new(),
            },
        }
    }
}

impl EvidenceCompilationComparison {
    pub fn has_expected_mismatches(&self) -> bool {
        !self.bundle_mismatches.is_empty()
            || !self.item_order_mismatches.is_empty()
            || !self.item_mismatches.is_empty()
            || !self.missing_suspected_causes.is_empty()
            || !self.extra_suspected_causes.is_empty()
            || !self.suspected_cause_mismatches.is_empty()
            || !self.missing_next_checks.is_empty()
            || !self.extra_next_checks.is_empty()
            || !self.next_check_mismatches.is_empty()
    }
}

pub fn generate_evidence_candidates(
    input: EvidenceCompilerInput<'_>,
) -> Result<Vec<EvidenceCandidate>, EvidenceCompileError> {
    let mut candidates = Vec::new();

    push_metric_anomaly_candidates(input, &mut candidates)?;
    push_log_pattern_candidates(input, &mut candidates)?;
    push_change_event_candidates(input, &mut candidates)?;
    push_trace_exemplar_candidates(input, &mut candidates)?;
    push_dependency_edge_candidates(input, &mut candidates)?;
    push_previous_incident_candidates(input, &mut candidates)?;
    push_missing_data_candidates(input, &mut candidates)?;
    push_counter_evidence_candidates(input, &mut candidates)?;

    score_evidence_candidates(input, candidates)
}

pub fn compile_evidence(
    query: &EvidenceQuery,
    store: &HotContextStore,
    derived: &DerivedContext,
) -> Result<EvidenceCompilation, EvidenceCompileError> {
    let input = EvidenceCompilerInput {
        query,
        store,
        derived,
    };
    let candidates = generate_evidence_candidates(input)?;
    let suspected_causes = rank_suspected_causes_from_candidates(input, &candidates);
    let mut compilation = select_evidence_compilation(input, candidates, suspected_causes)?;

    compilation.next_checks =
        suggest_next_checks(input, &compilation.bundle, &compilation.suspected_causes);

    Ok(compilation)
}

pub fn load_expected_compilation(
    case: &FixtureCase,
) -> Result<EvidenceCompilation, EvidenceCompilationGoldError> {
    let bundle = parse_required_artifact(case, "evidence_bundle")?;
    let suspected_causes = parse_optional_artifact(case, "suspected_causes")?.unwrap_or_default();
    let next_checks = parse_optional_artifact(case, "next_checks")?.unwrap_or_default();

    Ok(EvidenceCompilation::from_bundle(
        bundle,
        suspected_causes,
        next_checks,
    ))
}

pub fn compare_compiled_evidence_for_case(
    case: &FixtureCase,
    actual: &EvidenceCompilation,
) -> Result<EvidenceCompilationComparison, EvidenceCompilationGoldError> {
    let expected = load_expected_compilation(case)?;

    Ok(compare_compiled_evidence(&expected, actual))
}

pub fn compare_compiled_evidence(
    expected: &EvidenceCompilation,
    actual: &EvidenceCompilation,
) -> EvidenceCompilationComparison {
    let mut comparison = EvidenceCompilationComparison::default();

    compare_bundle(&expected.bundle, &actual.bundle, &mut comparison);
    compare_suspected_causes(expected, actual, &mut comparison);
    compare_next_checks(expected, actual, &mut comparison);

    comparison
}

pub fn canonical_evidence_item_payload_json_bytes(
    item: &EvidenceItem,
) -> Result<Vec<u8>, EvidenceCompileError> {
    let payload = EvidenceItemTokenPayload {
        id: item.id.as_str(),
        claim: item.claim.as_str(),
        kind: item.kind,
        direction: item.direction,
        strength: item.strength,
        time_window: &item.time_window,
        entities: &item.entities,
        source_refs: &item.source_refs,
        freshness: item.freshness,
        missing_data: &item.missing_data,
        privacy_scope: item.privacy_scope.as_str(),
        confidence: &item.confidence,
        note: &item.note,
    };

    serde_json::to_vec(&payload).map_err(|source| EvidenceCompileError::TokenEstimate {
        item_id: item.id.clone(),
        source,
    })
}

pub fn estimate_evidence_item_tokens(item: &EvidenceItem) -> Result<u32, EvidenceCompileError> {
    let bytes = canonical_evidence_item_payload_json_bytes(item)?.len();
    let tokens = bytes.div_ceil(4);

    u32::try_from(tokens).map_err(|_| EvidenceCompileError::TokenCostOverflow {
        item_id: item.id.clone(),
        bytes,
    })
}

pub fn apply_compiler_token_estimates(
    bundle: &mut EvidenceBundle,
) -> Result<(), EvidenceCompileError> {
    let mut tokens_used = 0u32;

    for item in &mut bundle.items {
        let token_cost = estimate_evidence_item_tokens(item)?;
        item.token_cost = token_cost;
        tokens_used = tokens_used.saturating_add(token_cost);
    }

    bundle.budget.tokens_used = tokens_used;

    Ok(())
}

pub fn score_evidence_candidates(
    input: EvidenceCompilerInput<'_>,
    candidates: Vec<EvidenceCandidate>,
) -> Result<Vec<EvidenceCandidate>, EvidenceCompileError> {
    let mut scored = Vec::with_capacity(candidates.len());

    for mut candidate in candidates {
        apply_evidence_strength_score(input, &mut candidate);
        candidate.item.token_cost = estimate_evidence_item_tokens(&candidate.item)?;
        scored.push(candidate);
    }

    Ok(scored)
}

pub fn rank_suspected_causes_from_candidates(
    input: EvidenceCompilerInput<'_>,
    candidates: &[EvidenceCandidate],
) -> Vec<SuspectedCause> {
    let relationships = resolved_relationships(input.store);
    let mut suspects = BTreeMap::<String, SuspectDraft>::new();

    for candidate in candidates {
        match candidate.item.direction {
            EvidenceDirection::Supports => {
                for entity in causal_entities_for_candidate(candidate, &relationships) {
                    let multiplier = structural_causal_multiplier(
                        input,
                        candidate,
                        entity.as_str(),
                        &relationships,
                    );
                    let contribution =
                        candidate.item.strength.0 * support_weight(candidate) * multiplier;
                    if contribution <= 0.0 {
                        continue;
                    }

                    let suspect = suspects
                        .entry(entity.clone())
                        .or_insert_with(|| SuspectDraft::new(entity));
                    suspect.support_score += contribution;
                    suspect.supporting.push(candidate.candidate_id.clone());
                    suspect.add_reasons(reason_tokens_for_candidate(input, candidate));
                }
            }
            EvidenceDirection::Weakens | EvidenceDirection::Contradicts => {
                for entity in causal_entities_for_candidate(candidate, &relationships) {
                    let contribution = candidate.item.strength.0 * counter_weight(candidate);
                    if contribution <= 0.0 {
                        continue;
                    }

                    let suspect = suspects
                        .entry(entity.clone())
                        .or_insert_with(|| SuspectDraft::new(entity));
                    suspect.counter_score += contribution;
                    suspect.counter.push(candidate.candidate_id.clone());
                    suspect.add_reasons(reason_tokens_for_candidate(input, candidate));
                }
            }
            EvidenceDirection::Neutral => {}
        }

        if candidate.source == EvidenceCandidateSource::MissingData {
            add_under_determined_suspect(&mut suspects, candidate);
        }
    }

    roll_up_runtime_child_support(&relationships, &mut suspects);
    adjust_relationship_causal_support(&relationships, &mut suspects);

    let mut ranked = suspects
        .into_values()
        .filter(|suspect| suspect.is_material())
        .map(SuspectDraft::into_suspected_cause)
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        right
            .score
            .0
            .total_cmp(&left.score.0)
            .then_with(|| left.entity.cmp(&right.entity))
    });

    for (index, cause) in ranked.iter_mut().enumerate() {
        cause.rank = (index + 1) as u32;
    }

    ranked
}

pub fn select_evidence_compilation(
    input: EvidenceCompilerInput<'_>,
    candidates: Vec<EvidenceCandidate>,
    suspected_causes: Vec<SuspectedCause>,
) -> Result<EvidenceCompilation, EvidenceCompileError> {
    let generated_items = candidates.len();
    let required_counter_count = required_counter_evidence_count(input.query);
    let token_limit = selection_token_limit(input.query);
    let max_items = input.query.budget.max_items as usize;
    let mut selected_ids = BTreeSet::new();
    let mut selected_candidates = Vec::<EvidenceCandidate>::new();
    let mut selected_candidate_tokens = 0u32;

    if required_counter_count > 0 {
        let counters = sorted_counter_candidates(&candidates);
        if counters.len() < required_counter_count as usize {
            return Err(EvidenceCompileError::RequirementUnsatisfied {
                requirement: "counter_evidence",
                message: format!(
                    "required at least {required_counter_count} counter-evidence item(s), generated {}",
                    counters.len()
                ),
            });
        }

        for candidate in counters.into_iter().take(required_counter_count as usize) {
            if !try_select_candidate(
                candidate,
                max_items,
                token_limit,
                &mut selected_ids,
                &mut selected_candidates,
                &mut selected_candidate_tokens,
            ) {
                return Err(EvidenceCompileError::RequirementUnsatisfied {
                    requirement: "counter_evidence",
                    message: format!(
                        "required at least {required_counter_count} counter-evidence item(s), but the selection budget could not fit them"
                    ),
                });
            }
        }
    }

    if let Some(top_cause) = suspected_causes.first() {
        for candidate in sorted_candidates_for_cause(&candidates, top_cause.entity.as_str()) {
            if try_select_candidate(
                candidate,
                max_items,
                token_limit,
                &mut selected_ids,
                &mut selected_candidates,
                &mut selected_candidate_tokens,
            ) {
                break;
            }
        }
    }

    for candidate in sorted_selection_candidates(&candidates) {
        try_select_candidate(
            candidate,
            max_items,
            token_limit,
            &mut selected_ids,
            &mut selected_candidates,
            &mut selected_candidate_tokens,
        );
    }

    selected_candidates.sort_by(compare_candidate_selection_order);
    let selected_id_map = selected_ev_id_map(&selected_candidates);
    let items = selected_candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            let mut item = candidate.item.clone();
            item.id = format!("ev-{}", index + 1);
            item.token_cost = estimate_evidence_item_tokens(&item)?;
            Ok(item)
        })
        .collect::<Result<Vec<_>, EvidenceCompileError>>()?;
    let tokens_used = items
        .iter()
        .fold(0u32, |tokens, item| tokens.saturating_add(item.token_cost));

    let dropped_items = dropped_candidates(&candidates, &selected_ids, max_items, token_limit);
    let items_dropped = dropped_items.len() as u32;
    let suspected_causes = remap_suspected_cause_links(suspected_causes, &selected_id_map);

    let bundle = EvidenceBundle {
        question: input.query.intent.question.clone(),
        hypothesis: input.query.intent.hypothesis.clone(),
        time_window: input.query.time_window.clone(),
        budget: EvidenceBudget {
            max_items: input.query.budget.max_items,
            max_tokens: input.query.budget.max_tokens,
            tokens_used,
            items_dropped,
            note: None,
        },
        items,
    };

    Ok(EvidenceCompilation {
        bundle,
        suspected_causes,
        next_checks: Vec::new(),
        report: EvidenceCompilationReport {
            generated_items,
            selected_items: selected_ids.len(),
            dropped_items,
            requirement_failures: Vec::new(),
        },
    })
}

pub fn suggest_next_checks(
    input: EvidenceCompilerInput<'_>,
    bundle: &EvidenceBundle,
    suspected_causes: &[SuspectedCause],
) -> Vec<NextCheck> {
    let mut drafts = Vec::<NextCheckDraft>::new();

    if let Some(item) = bundle.items.iter().find(|item| is_missing_data_item(item)) {
        push_next_check_draft(
            &mut drafts,
            0,
            NextCheck {
                action: format!(
                    "Backfill or query {} for {} during {} to {}.",
                    missing_data_label(item),
                    entity_label(&item.entities),
                    input.query.time_window.start,
                    input.query.time_window.end
                ),
                rationale: "The selected evidence marks this signal as missing; recovering it can confirm or reject the current hypothesis.".to_string(),
                expected_signal: missing_data_expected_signal(item).to_string(),
            },
        );
    }

    if let Some(cause) = suspected_causes
        .iter()
        .find(|cause| cause.trap_note.is_some() || !cause.counter.is_empty())
        && let Some(item) = first_linked_item(bundle, &cause.counter)
    {
        push_next_check_draft(
            &mut drafts,
            1,
            NextCheck {
                action: format!(
                    "Validate counter-evidence {} before mitigating {}.",
                    item.id, cause.entity
                ),
                rationale: format!(
                    "{} has selected counter-evidence; checking it first reduces false-causality risk.",
                    cause.entity
                ),
                expected_signal: counter_check_expected_signal(item).to_string(),
            },
        );
    }

    if let Some(cause) = suspected_causes.first() {
        if cause.score.0 < 0.45 {
            push_next_check_draft(
                &mut drafts,
                2,
                NextCheck {
                    action: format!(
                        "Gather an independent signal for {} before treating it as causal.",
                        cause.entity
                    ),
                    rationale: format!(
                        "The top suspected-cause score is {:.2}; another signal should discriminate between hypotheses.",
                        cause.score.0
                    ),
                    expected_signal: "compare_windows".to_string(),
                },
            );
        }

        if let Some(item) =
            first_linked_item(bundle, &cause.supporting).or_else(|| strongest_support_item(bundle))
        {
            push_next_check_draft(
                &mut drafts,
                3,
                NextCheck {
                    action: format!(
                        "Confirm {} by re-checking {} evidence {}.",
                        cause.entity,
                        evidence_kind_label(item.kind),
                        item.id
                    ),
                    rationale: format!(
                        "The selected evidence is linked to the top suspected cause with score {:.2}.",
                        cause.score.0
                    ),
                    expected_signal: next_check_expected_signal(item).to_string(),
                },
            );
        }
    }

    if drafts.is_empty()
        && let Some(item) = bundle.items.first()
    {
        push_next_check_draft(
            &mut drafts,
            4,
            NextCheck {
                action: format!(
                    "Inspect the strongest selected evidence {} for {}.",
                    item.id,
                    entity_label(&item.entities)
                ),
                rationale:
                    "The compiler selected this item as high-value evidence under the token budget."
                        .to_string(),
                expected_signal: next_check_expected_signal(item).to_string(),
            },
        );
    }

    drafts.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.check.expected_signal.cmp(&right.check.expected_signal))
            .then_with(|| left.check.action.cmp(&right.check.action))
    });

    drafts
        .into_iter()
        .map(|draft| draft.check)
        .take(3)
        .collect()
}

pub fn insert_evidence_compilation(
    store: &mut HotContextStore,
    compilation: &EvidenceCompilation,
) -> Result<(), HotStoreError> {
    for item in &compilation.bundle.items {
        store.insert_record(StoredRecord {
            key: SourceKey::new(item.id.clone()),
            kind: StoredRecordKind::EvidenceItem,
            time_window: Some(item.time_window.clone()),
            entities: item.entities.clone(),
            payload: serde_json::to_value(item).expect("evidence item should serialize"),
        })?;
    }

    for cause in &compilation.suspected_causes {
        store.insert_record(StoredRecord {
            key: SourceKey::new(format!("suspected-cause:{}", cause.rank)),
            kind: StoredRecordKind::SuspectedCause,
            time_window: Some(compilation.bundle.time_window.clone()),
            entities: suspected_cause_record_entities(cause),
            payload: serde_json::to_value(cause).expect("suspected cause should serialize"),
        })?;
    }

    let next_check_entities = compilation_entities(compilation);
    for (index, check) in compilation.next_checks.iter().enumerate() {
        store.insert_record(StoredRecord {
            key: SourceKey::new(format!("next-check:{}", index + 1)),
            kind: StoredRecordKind::NextCheck,
            time_window: Some(compilation.bundle.time_window.clone()),
            entities: next_check_entities.clone(),
            payload: serde_json::to_value(check).expect("next check should serialize"),
        })?;
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct NextCheckDraft {
    priority: u8,
    check: NextCheck,
}

fn push_next_check_draft(drafts: &mut Vec<NextCheckDraft>, priority: u8, check: NextCheck) {
    if drafts.iter().any(|draft| {
        draft.check.expected_signal == check.expected_signal && draft.check.action == check.action
    }) {
        return;
    }

    drafts.push(NextCheckDraft { priority, check });
}

fn first_linked_item<'a>(bundle: &'a EvidenceBundle, ids: &[String]) -> Option<&'a EvidenceItem> {
    ids.iter()
        .find_map(|id| bundle.items.iter().find(|item| item.id == *id))
}

fn strongest_support_item(bundle: &EvidenceBundle) -> Option<&EvidenceItem> {
    bundle
        .items
        .iter()
        .filter(|item| item.direction == EvidenceDirection::Supports)
        .max_by(|left, right| {
            left.strength
                .0
                .total_cmp(&right.strength.0)
                .then_with(|| right.id.cmp(&left.id))
        })
}

fn is_missing_data_item(item: &EvidenceItem) -> bool {
    item.kind == EvidenceKind::MissingData
        || !item.missing_data.is_empty()
        || has_source_signal(item, SourceSignal::TelemetryGap)
}

fn missing_data_label(item: &EvidenceItem) -> String {
    item.missing_data
        .first()
        .cloned()
        .unwrap_or_else(|| "missing telemetry".to_string())
}

fn entity_label(entities: &[String]) -> String {
    if entities.is_empty() {
        "the selected entity".to_string()
    } else {
        entities.join(", ")
    }
}

fn missing_data_expected_signal(item: &EvidenceItem) -> &'static str {
    let text = item
        .missing_data
        .iter()
        .chain(std::iter::once(&item.claim))
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");

    if text.contains("service.instance.id") || text.contains("service.version") {
        "entity_resolution"
    } else if text.contains("log") {
        "log_cluster"
    } else if text.contains("change") || text.contains("deploy") || text.contains("config") {
        "change_event"
    } else {
        "metric_anomaly"
    }
}

fn counter_check_expected_signal(item: &EvidenceItem) -> &'static str {
    if has_source_signal(item, SourceSignal::Relationship) {
        "relationship"
    } else if has_source_signal(item, SourceSignal::Change)
        || item.kind == EvidenceKind::ChangeEvent
    {
        "change_event"
    } else if has_source_signal(item, SourceSignal::Metric) {
        "compare_windows"
    } else if has_source_signal(item, SourceSignal::Log) {
        "log_cluster"
    } else {
        "compare_windows"
    }
}

fn next_check_expected_signal(item: &EvidenceItem) -> &'static str {
    match item.kind {
        EvidenceKind::MetricAnomaly => "metric_anomaly",
        EvidenceKind::TraceExemplar => "trace",
        EvidenceKind::LogCluster => "log_cluster",
        EvidenceKind::ChangeEvent => "change_event",
        EvidenceKind::DependencyEdge => "relationship",
        EvidenceKind::ProfileHotspot => "profile_hotspot",
        EvidenceKind::PreviousIncident => "find_related_anomalies",
        EvidenceKind::CounterEvidence => counter_check_expected_signal(item),
        EvidenceKind::MissingData => missing_data_expected_signal(item),
    }
}

fn evidence_kind_label(kind: EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::MetricAnomaly => "metric anomaly",
        EvidenceKind::TraceExemplar => "trace exemplar",
        EvidenceKind::LogCluster => "log cluster",
        EvidenceKind::ChangeEvent => "change event",
        EvidenceKind::DependencyEdge => "dependency",
        EvidenceKind::ProfileHotspot => "profile hotspot",
        EvidenceKind::PreviousIncident => "previous incident",
        EvidenceKind::CounterEvidence => "counter-evidence",
        EvidenceKind::MissingData => "missing-data",
    }
}

fn suspected_cause_record_entities(cause: &SuspectedCause) -> Vec<String> {
    if cause.entity == "under-determined" {
        Vec::new()
    } else {
        vec![cause.entity.clone()]
    }
}

fn compilation_entities(compilation: &EvidenceCompilation) -> Vec<String> {
    dedupe_stable(
        compilation
            .bundle
            .items
            .iter()
            .flat_map(|item| item.entities.iter().cloned())
            .chain(
                compilation
                    .suspected_causes
                    .iter()
                    .filter(|cause| cause.entity != "under-determined")
                    .map(|cause| cause.entity.clone()),
            )
            .collect(),
    )
}

fn required_counter_evidence_count(query: &EvidenceQuery) -> u32 {
    if query.require_counter_evidence {
        query.budget.min_counter_evidence_items.unwrap_or(1).max(1)
    } else {
        query.budget.min_counter_evidence_items.unwrap_or(0)
    }
}

fn selection_token_limit(query: &EvidenceQuery) -> u32 {
    query
        .budget
        .max_tokens
        .saturating_sub(query.budget.reserve_tokens_for_raw_refs.unwrap_or(0))
}

fn sorted_counter_candidates(candidates: &[EvidenceCandidate]) -> Vec<&EvidenceCandidate> {
    let mut counters = candidates
        .iter()
        .filter(|candidate| is_counter_evidence_candidate(candidate))
        .collect::<Vec<_>>();
    counters.sort_by(|left, right| compare_candidate_selection_order(left, right));
    counters
}

fn sorted_candidates_for_cause<'a>(
    candidates: &'a [EvidenceCandidate],
    cause_entity: &str,
) -> Vec<&'a EvidenceCandidate> {
    let mut matches = candidates
        .iter()
        .filter(|candidate| {
            candidate.item.direction == EvidenceDirection::Supports
                && candidate_mentions_cause(candidate, cause_entity)
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| compare_candidate_selection_order(left, right));
    matches
}

fn sorted_selection_candidates(candidates: &[EvidenceCandidate]) -> Vec<&EvidenceCandidate> {
    let mut sorted = candidates.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| compare_candidate_selection_order(left, right));
    sorted
}

fn try_select_candidate(
    candidate: &EvidenceCandidate,
    max_items: usize,
    token_limit: u32,
    selected_ids: &mut BTreeSet<String>,
    selected_candidates: &mut Vec<EvidenceCandidate>,
    selected_tokens: &mut u32,
) -> bool {
    if selected_ids.contains(&candidate.candidate_id) {
        return true;
    }
    if selected_candidates.len() >= max_items {
        return false;
    }
    if candidate.item.token_cost > token_limit.saturating_sub(*selected_tokens) {
        return false;
    }

    selected_ids.insert(candidate.candidate_id.clone());
    selected_candidates.push(candidate.clone());
    *selected_tokens = selected_tokens.saturating_add(candidate.item.token_cost);

    true
}

fn selected_ev_id_map(selected_candidates: &[EvidenceCandidate]) -> BTreeMap<String, String> {
    selected_candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            (
                candidate.candidate_id.clone(),
                format!("ev-{}", index.saturating_add(1)),
            )
        })
        .collect()
}

fn dropped_candidates(
    candidates: &[EvidenceCandidate],
    selected_ids: &BTreeSet<String>,
    max_items: usize,
    token_limit: u32,
) -> Vec<DroppedEvidenceItem> {
    let selected_tokens = candidates
        .iter()
        .filter(|candidate| selected_ids.contains(&candidate.candidate_id))
        .fold(0u32, |tokens, candidate| {
            tokens.saturating_add(candidate.item.token_cost)
        });

    candidates
        .iter()
        .filter(|candidate| !selected_ids.contains(&candidate.candidate_id))
        .map(|candidate| DroppedEvidenceItem {
            id: candidate.candidate_id.clone(),
            reason: if selected_ids.len() >= max_items {
                "max_items".to_string()
            } else if candidate.item.token_cost > token_limit.saturating_sub(selected_tokens) {
                "max_tokens".to_string()
            } else {
                "lower_priority".to_string()
            },
            token_cost: candidate.item.token_cost,
        })
        .collect()
}

fn remap_suspected_cause_links(
    causes: Vec<SuspectedCause>,
    selected_id_map: &BTreeMap<String, String>,
) -> Vec<SuspectedCause> {
    causes
        .into_iter()
        .map(|mut cause| {
            cause.supporting = remap_evidence_ids(cause.supporting, selected_id_map);
            cause.counter = remap_evidence_ids(cause.counter, selected_id_map);
            cause
        })
        .collect()
}

fn remap_evidence_ids(ids: Vec<String>, selected_id_map: &BTreeMap<String, String>) -> Vec<String> {
    dedupe_stable(
        ids.into_iter()
            .filter_map(|id| selected_id_map.get(&id).cloned())
            .collect(),
    )
}

fn compare_candidate_selection_order(
    left: &EvidenceCandidate,
    right: &EvidenceCandidate,
) -> std::cmp::Ordering {
    candidate_selection_group(left)
        .cmp(&candidate_selection_group(right))
        .then_with(|| right.item.strength.0.total_cmp(&left.item.strength.0))
        .then_with(|| left.item.token_cost.cmp(&right.item.token_cost))
        .then_with(|| left.candidate_id.cmp(&right.candidate_id))
}

fn candidate_selection_group(candidate: &EvidenceCandidate) -> u8 {
    match (
        candidate.item.direction,
        candidate.source,
        candidate.item.kind,
    ) {
        (EvidenceDirection::Supports, EvidenceCandidateSource::ChangeEvent, _) => 0,
        (EvidenceDirection::Supports, EvidenceCandidateSource::MetricAnomaly, _) => 1,
        (EvidenceDirection::Supports, EvidenceCandidateSource::TraceExemplar, _) => 2,
        (EvidenceDirection::Supports, EvidenceCandidateSource::LogPattern, _) => 3,
        (_, _, EvidenceKind::CounterEvidence)
        | (EvidenceDirection::Weakens | EvidenceDirection::Contradicts, _, _) => 4,
        (_, EvidenceCandidateSource::MissingData, _) => 5,
        (_, EvidenceCandidateSource::PreviousIncident, _) => 6,
        (_, EvidenceCandidateSource::DependencyEdge, _) => 7,
        _ => 8,
    }
}

fn candidate_mentions_cause(candidate: &EvidenceCandidate, cause_entity: &str) -> bool {
    if cause_entity == "under-determined" {
        return candidate.source == EvidenceCandidateSource::MissingData;
    }

    candidate.item.entities.iter().any(|entity| {
        entity == cause_entity || entity.strip_suffix("(aggregate)") == Some(cause_entity)
    })
}

fn is_counter_evidence_candidate(candidate: &EvidenceCandidate) -> bool {
    candidate.item.kind == EvidenceKind::CounterEvidence
        || matches!(
            candidate.item.direction,
            EvidenceDirection::Weakens | EvidenceDirection::Contradicts
        )
}

#[derive(Debug, Clone)]
struct SuspectDraft {
    entity: String,
    support_score: f64,
    counter_score: f64,
    reasons: BTreeSet<String>,
    supporting: Vec<String>,
    counter: Vec<String>,
}

impl SuspectDraft {
    fn new(entity: String) -> Self {
        Self {
            entity,
            support_score: 0.0,
            counter_score: 0.0,
            reasons: BTreeSet::new(),
            supporting: Vec::new(),
            counter: Vec::new(),
        }
    }

    fn add_reasons(&mut self, reasons: Vec<String>) {
        self.reasons.extend(reasons);
    }

    fn is_material(&self) -> bool {
        self.entity == "under-determined"
            || self.support_score >= 0.20
            || self.counter_score >= 0.45
    }

    fn into_suspected_cause(mut self) -> SuspectedCause {
        self.supporting = dedupe_stable(self.supporting);
        self.counter = dedupe_stable(self.counter);

        let score = causal_suspicion_score(self.support_score, self.counter_score);
        let reasons = self.reasons.into_iter().collect::<Vec<_>>();
        let note = if self.entity == "under-determined" {
            Some("telemetry gaps make a confident concrete cause unsafe".to_string())
        } else if score.0 < 0.20 && !self.counter.is_empty() {
            Some(format!(
                "{} is retained as a false-causality risk, not a likely cause",
                self.entity
            ))
        } else {
            None
        };
        let trap_note = if self.entity != "under-determined"
            && self.counter_score >= self.support_score.max(0.20)
        {
            Some(format!(
                "{} has stronger counter-evidence than causal support; downgrade the suspect",
                self.entity
            ))
        } else {
            None
        };

        SuspectedCause {
            rank: 0,
            entity: self.entity.clone(),
            hypothesis: format!(
                "{} is a plausible cause based on {}.",
                self.entity,
                if reasons.is_empty() {
                    "available evidence".to_string()
                } else {
                    reasons.join(", ")
                }
            ),
            score,
            reasons,
            supporting: self.supporting,
            counter: self.counter,
            note,
            trap_note,
        }
    }
}

fn apply_evidence_strength_score(
    input: EvidenceCompilerInput<'_>,
    candidate: &mut EvidenceCandidate,
) {
    let source_ref_quality = source_ref_quality(input.store, &candidate.item);
    candidate
        .item
        .confidence
        .insert("source_ref_quality".to_string(), source_ref_quality);

    match candidate.source {
        EvidenceCandidateSource::MetricAnomaly => {
            apply_metric_anomaly_strength(input, candidate, source_ref_quality);
        }
        EvidenceCandidateSource::LogPattern => {
            apply_log_pattern_strength(input, candidate, source_ref_quality);
        }
        EvidenceCandidateSource::CounterEvidence
            if candidate.item.kind == EvidenceKind::ChangeEvent
                || has_source_signal(&candidate.item, SourceSignal::Change) =>
        {
            apply_change_strength(input, candidate, source_ref_quality);
        }
        EvidenceCandidateSource::TraceExemplar => {
            apply_trace_strength(candidate, source_ref_quality);
        }
        EvidenceCandidateSource::DependencyEdge => {
            apply_dependency_strength(input, candidate, source_ref_quality);
        }
        EvidenceCandidateSource::PreviousIncident => {
            apply_previous_incident_strength(input, candidate, source_ref_quality);
        }
        EvidenceCandidateSource::MissingData => {
            apply_missing_data_strength(candidate, source_ref_quality);
        }
        EvidenceCandidateSource::CounterEvidence => {
            apply_counter_evidence_strength(candidate, source_ref_quality);
        }
        EvidenceCandidateSource::ChangeEvent => {
            apply_change_strength(input, candidate, source_ref_quality);
        }
    }
}

fn apply_metric_anomaly_strength(
    input: EvidenceCompilerInput<'_>,
    candidate: &mut EvidenceCandidate,
    source_ref_quality: UnitInterval,
) {
    let detector = anomaly_window_for_candidate(input, candidate)
        .map(|window| window.detector_confidence)
        .unwrap_or(UnitInterval(0.50));
    let magnitude = anomaly_window_for_candidate(input, candidate)
        .map(anomaly_magnitude_strength)
        .unwrap_or(UnitInterval(0.50));
    let coverage = if has_source_signal(&candidate.item, SourceSignal::Metric)
        && has_source_signal(&candidate.item, SourceSignal::AnomalyWindow)
    {
        UnitInterval(1.0)
    } else {
        UnitInterval(0.70)
    };

    candidate
        .item
        .confidence
        .insert("detector".to_string(), detector);
    candidate
        .item
        .confidence
        .insert("magnitude".to_string(), magnitude);
    candidate
        .item
        .confidence
        .insert("coverage".to_string(), coverage);

    if detector.0 < 0.40 || note_contains(&candidate.item, "no anomaly") {
        candidate.item.direction = EvidenceDirection::Weakens;
        candidate.item.kind = EvidenceKind::CounterEvidence;
    }

    candidate.item.strength = weighted_unit_interval(&[
        (detector, 0.35),
        (magnitude, 0.25),
        (coverage, 0.20),
        (source_ref_quality, 0.20),
    ]);
}

fn apply_log_pattern_strength(
    input: EvidenceCompilerInput<'_>,
    candidate: &mut EvidenceCandidate,
    source_ref_quality: UnitInterval,
) {
    let pattern = log_pattern_for_candidate(input, candidate);
    let severity = pattern
        .map(|pattern| severity_strength(pattern.severity.as_str()))
        .unwrap_or(UnitInterval(0.65));
    let volume = pattern
        .map(|pattern| UnitInterval((pattern.count as f64 / 3.0).clamp(0.30, 1.0)))
        .unwrap_or(UnitInterval(0.50));
    let exemplar_quality = if candidate.item.source_refs.iter().any(|source_ref| {
        matches!(source_ref.signal, SourceSignal::Log) && !source_ref.r#ref.trim().is_empty()
    }) {
        UnitInterval(1.0)
    } else {
        UnitInterval(0.60)
    };

    candidate
        .item
        .confidence
        .insert("severity".to_string(), severity);
    candidate
        .item
        .confidence
        .insert("volume".to_string(), volume);
    candidate
        .item
        .confidence
        .insert("exemplar_quality".to_string(), exemplar_quality);

    candidate.item.strength = weighted_unit_interval(&[
        (severity, 0.35),
        (volume, 0.25),
        (exemplar_quality, 0.20),
        (source_ref_quality, 0.20),
    ]);
}

fn apply_change_strength(
    input: EvidenceCompilerInput<'_>,
    candidate: &mut EvidenceCandidate,
    source_ref_quality: UnitInterval,
) {
    let time_alignment = change_time_alignment(input, candidate);
    let entity_specificity = if candidate.item.entities.is_empty() {
        UnitInterval(0.25)
    } else {
        UnitInterval(1.0)
    };

    candidate
        .item
        .confidence
        .insert("time_alignment".to_string(), time_alignment);
    candidate
        .item
        .confidence
        .insert("entity_specificity".to_string(), entity_specificity);
    candidate
        .item
        .confidence
        .insert("change_proximity".to_string(), time_alignment);

    candidate.item.strength = weighted_unit_interval(&[
        (time_alignment, 0.45),
        (entity_specificity, 0.20),
        (source_ref_quality, 0.20),
        (
            if candidate.item.direction == EvidenceDirection::Weakens {
                UnitInterval(0.90)
            } else {
                UnitInterval(0.75)
            },
            0.15,
        ),
    ]);
}

fn apply_trace_strength(candidate: &mut EvidenceCandidate, source_ref_quality: UnitInterval) {
    let span_specificity = if candidate
        .item
        .source_refs
        .iter()
        .any(|source_ref| source_ref.r#ref.contains('/'))
    {
        UnitInterval(1.0)
    } else {
        UnitInterval(0.65)
    };
    let path_specificity =
        UnitInterval((candidate.item.entities.len() as f64 / 3.0).clamp(0.45, 1.0));

    candidate
        .item
        .confidence
        .insert("span_specificity".to_string(), span_specificity);
    candidate
        .item
        .confidence
        .insert("path_specificity".to_string(), path_specificity);

    candidate.item.strength = weighted_unit_interval(&[
        (span_specificity, 0.40),
        (path_specificity, 0.30),
        (source_ref_quality, 0.30),
    ]);
}

fn apply_dependency_strength(
    input: EvidenceCompilerInput<'_>,
    candidate: &mut EvidenceCandidate,
    source_ref_quality: UnitInterval,
) {
    let relationship_confidence = relationship_for_candidate(input, candidate)
        .map(|relationship| relationship.confidence)
        .unwrap_or(UnitInterval(0.65));

    candidate.item.confidence.insert(
        "relationship_confidence".to_string(),
        relationship_confidence,
    );

    candidate.item.strength =
        weighted_unit_interval(&[(relationship_confidence, 0.70), (source_ref_quality, 0.30)]);
}

fn apply_previous_incident_strength(
    input: EvidenceCompilerInput<'_>,
    candidate: &mut EvidenceCandidate,
    source_ref_quality: UnitInterval,
) {
    let similarity = prior_incident_similarity(input, candidate).unwrap_or(UnitInterval(0.70));
    let entity_specificity = if candidate.item.entities.is_empty() {
        UnitInterval(0.35)
    } else {
        UnitInterval(0.85)
    };

    candidate
        .item
        .confidence
        .insert("signature_similarity".to_string(), similarity);
    candidate
        .item
        .confidence
        .insert("entity_specificity".to_string(), entity_specificity);

    candidate.item.strength = weighted_unit_interval(&[
        (similarity, 0.45),
        (entity_specificity, 0.25),
        (source_ref_quality, 0.30),
    ]);
}

fn apply_missing_data_strength(
    candidate: &mut EvidenceCandidate,
    source_ref_quality: UnitInterval,
) {
    let gap_materiality = if candidate.item.missing_data.is_empty() {
        UnitInterval(0.55)
    } else {
        UnitInterval(0.90)
    };
    let validation_impact = if candidate.item.entities.is_empty() {
        UnitInterval(0.65)
    } else {
        UnitInterval(0.80)
    };

    candidate
        .item
        .confidence
        .insert("gap_materiality".to_string(), gap_materiality);
    candidate
        .item
        .confidence
        .insert("validation_impact".to_string(), validation_impact);

    candidate.item.strength = weighted_unit_interval(&[
        (gap_materiality, 0.45),
        (validation_impact, 0.25),
        (source_ref_quality, 0.30),
    ]);
}

fn apply_counter_evidence_strength(
    candidate: &mut EvidenceCandidate,
    source_ref_quality: UnitInterval,
) {
    let contradiction = if candidate.item.direction == EvidenceDirection::Contradicts {
        UnitInterval(0.95)
    } else if note_contains(&candidate.item, "flat") || note_contains(&candidate.item, "counter") {
        UnitInterval(0.90)
    } else {
        UnitInterval(0.75)
    };

    candidate
        .item
        .confidence
        .insert("contradiction_quality".to_string(), contradiction);

    candidate.item.strength =
        weighted_unit_interval(&[(contradiction, 0.65), (source_ref_quality, 0.35)]);
}

fn add_under_determined_suspect(
    suspects: &mut BTreeMap<String, SuspectDraft>,
    candidate: &EvidenceCandidate,
) {
    let suspect = suspects
        .entry("under-determined".to_string())
        .or_insert_with(|| SuspectDraft::new("under-determined".to_string()));
    suspect.support_score += candidate.item.strength.0 * 0.80;
    suspect.supporting.push(candidate.candidate_id.clone());
    suspect
        .reasons
        .insert("telemetry_gap_across_peak".to_string());
}

fn roll_up_runtime_child_support(
    relationships: &[ResolvedRelationship],
    suspects: &mut BTreeMap<String, SuspectDraft>,
) {
    let transfers = relationships
        .iter()
        .filter(|relationship| is_runtime_child_relationship(relationship))
        .filter_map(|relationship| {
            suspects
                .get(&relationship.dst)
                .cloned()
                .map(|child| (relationship.src.clone(), child))
        })
        .collect::<Vec<_>>();

    for (parent_entity, child) in transfers {
        let parent = suspects
            .entry(parent_entity.clone())
            .or_insert_with(|| SuspectDraft::new(parent_entity));
        parent.support_score += child.support_score * 0.80;
        parent.counter_score += child.counter_score * 0.80;
        parent.supporting.extend(child.supporting);
        parent.counter.extend(child.counter);
        parent.reasons.extend(child.reasons);
        parent.reasons.insert("runtime_child_anomaly".to_string());
    }
}

fn adjust_relationship_causal_support(
    relationships: &[ResolvedRelationship],
    suspects: &mut BTreeMap<String, SuspectDraft>,
) {
    for relationship in relationships {
        let src = canonical_suspect_entity(&relationship.src, relationships);
        let dst = canonical_suspect_entity(&relationship.dst, relationships);
        if src == dst {
            continue;
        }

        match relationship.relationship_type {
            RelationshipType::Retries => {
                add_structural_counter_from_support(
                    suspects,
                    dst.as_str(),
                    0.90,
                    "retry_fanout_trace",
                );
                transfer_causal_support(
                    suspects,
                    dst.as_str(),
                    src.as_str(),
                    0.55,
                    0.55,
                    "retry_fanout_trace",
                );
            }
            RelationshipType::FansOutTo => {}
            RelationshipType::ReadsFrom
            | RelationshipType::WritesTo
            | RelationshipType::DependsOn => {
                if relationship_role_contains(relationship, "fallback") {
                    add_structural_counter_from_support(
                        suspects,
                        dst.as_str(),
                        0.80,
                        "dependency_direction",
                    );
                    dampen_structural_support(suspects, dst.as_str(), 0.35);
                    continue;
                }
                if suspect_is_counter_dominated(suspects, dst.as_str()) {
                    continue;
                }
                if suspect_has_any_reason(
                    suspects,
                    src.as_str(),
                    &["oom_logs", "restart_aligned_errors", "sawtooth_rss"],
                ) {
                    continue;
                }
                transfer_causal_support(
                    suspects,
                    src.as_str(),
                    dst.as_str(),
                    1.20,
                    0.80,
                    "dependency_direction",
                );
            }
            RelationshipType::Calls
            | RelationshipType::RunsOn
            | RelationshipType::DeployedAs
            | RelationshipType::Emits
            | RelationshipType::SharesResourceWith => {}
        }
    }
}

fn suspect_is_counter_dominated(suspects: &BTreeMap<String, SuspectDraft>, entity: &str) -> bool {
    suspects
        .get(entity)
        .is_some_and(|suspect| suspect.counter_score >= suspect.support_score.max(0.20))
}

fn suspect_has_any_reason(
    suspects: &BTreeMap<String, SuspectDraft>,
    entity: &str,
    reasons: &[&str],
) -> bool {
    suspects.get(entity).is_some_and(|suspect| {
        reasons
            .iter()
            .any(|reason| suspect.reasons.contains(*reason))
    })
}

fn transfer_causal_support(
    suspects: &mut BTreeMap<String, SuspectDraft>,
    from_entity: &str,
    to_entity: &str,
    transfer_ratio: f64,
    dampen_ratio: f64,
    reason: &str,
) {
    let Some(source) = suspects.get(from_entity).cloned() else {
        return;
    };
    if source.support_score <= 0.0 {
        return;
    }

    let target = suspects
        .entry(to_entity.to_string())
        .or_insert_with(|| SuspectDraft::new(to_entity.to_string()));
    target.support_score += source.support_score * transfer_ratio;
    target.supporting.extend(source.supporting.clone());
    target.reasons.extend(source.reasons.iter().cloned());
    target.reasons.insert(reason.to_string());

    if let Some(source) = suspects.get_mut(from_entity) {
        source.support_score *= 1.0 - dampen_ratio;
        source.reasons.insert(reason.to_string());
    }
}

fn dampen_structural_support(
    suspects: &mut BTreeMap<String, SuspectDraft>,
    entity: &str,
    dampen_ratio: f64,
) {
    if let Some(suspect) = suspects.get_mut(entity) {
        suspect.support_score *= 1.0 - dampen_ratio;
    }
}

fn add_structural_counter_from_support(
    suspects: &mut BTreeMap<String, SuspectDraft>,
    entity: &str,
    counter_ratio: f64,
    reason: &str,
) {
    let Some(snapshot) = suspects.get(entity).cloned() else {
        return;
    };
    if snapshot.support_score <= 0.0 {
        return;
    }

    if let Some(suspect) = suspects.get_mut(entity) {
        suspect.counter_score += snapshot.support_score * counter_ratio;
        suspect.counter.extend(snapshot.supporting);
        suspect.reasons.insert(reason.to_string());
    }
}

fn relationship_role_contains(relationship: &ResolvedRelationship, needle: &str) -> bool {
    relationship
        .attributes
        .get("role")
        .and_then(Value::as_str)
        .is_some_and(|role| role.to_ascii_lowercase().contains(needle))
}

fn causal_suspicion_score(support_score: f64, counter_score: f64) -> UnitInterval {
    let net = support_score - counter_score * 1.15;
    if net <= 0.0 {
        return UnitInterval((0.05 + support_score * 0.05).clamp(0.0, 0.18));
    }

    UnitInterval((1.0 - (-net * 0.75).exp()).clamp(0.0, 1.0))
}

fn causal_entities_for_candidate(
    candidate: &EvidenceCandidate,
    relationships: &[ResolvedRelationship],
) -> Vec<String> {
    dedupe_stable(
        candidate
            .item
            .entities
            .iter()
            .map(|entity| canonical_suspect_entity(entity, relationships))
            .collect(),
    )
}

fn canonical_suspect_entity(entity: &str, relationships: &[ResolvedRelationship]) -> String {
    let entity = entity.strip_suffix("(aggregate)").unwrap_or(entity);
    if is_runtime_child_entity(entity)
        && let Some(parent) = relationships.iter().find(|relationship| {
            relationship.dst == entity && is_runtime_child_relationship(relationship)
        })
    {
        return parent.src.clone();
    }

    entity.to_string()
}

fn is_runtime_child_relationship(relationship: &ResolvedRelationship) -> bool {
    matches!(
        relationship.relationship_type,
        RelationshipType::RunsOn | RelationshipType::DeployedAs
    ) || is_runtime_child_entity(relationship.dst.as_str())
}

fn is_runtime_child_entity(entity: &str) -> bool {
    entity.starts_with("pod:")
        || entity.starts_with("instance:")
        || entity.starts_with("container:")
        || entity.starts_with("host:")
}

fn support_weight(candidate: &EvidenceCandidate) -> f64 {
    match candidate.source {
        EvidenceCandidateSource::MetricAnomaly => 0.60,
        EvidenceCandidateSource::LogPattern => 0.55,
        EvidenceCandidateSource::ChangeEvent => {
            confidence_value(candidate, "time_alignment").0 * 0.55
        }
        EvidenceCandidateSource::TraceExemplar => 0.45,
        EvidenceCandidateSource::DependencyEdge => 0.15,
        EvidenceCandidateSource::PreviousIncident => 0.55,
        EvidenceCandidateSource::MissingData => 0.0,
        EvidenceCandidateSource::CounterEvidence => 0.0,
    }
}

fn counter_weight(candidate: &EvidenceCandidate) -> f64 {
    match candidate.item.direction {
        EvidenceDirection::Contradicts => 1.15,
        EvidenceDirection::Weakens => 0.95,
        EvidenceDirection::Supports | EvidenceDirection::Neutral => 0.0,
    }
}

fn structural_causal_multiplier(
    input: EvidenceCompilerInput<'_>,
    candidate: &EvidenceCandidate,
    entity: &str,
    relationships: &[ResolvedRelationship],
) -> f64 {
    let mut multiplier: f64 = 1.0;

    if candidate_structurally_mentions_entity(candidate, entity, relationships) {
        multiplier += 0.08;
    }

    if candidate.item.source_refs.0.len() > 1 {
        multiplier += 0.04;
    }

    if !candidate.item.missing_data.is_empty() {
        multiplier -= 0.15;
    }

    match candidate.source {
        EvidenceCandidateSource::MetricAnomaly => {
            if let Some(window) = anomaly_window_for_candidate(input, candidate) {
                let signal = window.signal.to_ascii_lowercase();
                if window.start.is_some() && window.end.is_some() {
                    multiplier += 0.10;
                }
                if anomaly_magnitude_strength(window).0 >= 0.75 {
                    multiplier += 0.10;
                }
                if window.detector_confidence.0 < 0.40 {
                    multiplier -= 0.20;
                }
                if signal.contains("retry") {
                    multiplier += 0.25;
                }
                if signal.contains("hit_ratio") {
                    multiplier += 0.45;
                }
                if signal.contains("queue.depth") {
                    multiplier += 0.25;
                }
                if signal.contains("dependency.error_rate") {
                    multiplier += 0.20;
                }
                if signal.contains("request.rate") && !signal.contains("retry") {
                    multiplier -= 0.35;
                }
            }
        }
        EvidenceCandidateSource::LogPattern => {
            if confidence_value(candidate, "severity").0 >= 0.75 {
                multiplier += 0.08;
            }
            if confidence_value(candidate, "volume").0 >= 0.75 {
                multiplier += 0.06;
            }
            if confidence_value(candidate, "exemplar_quality").0 >= 0.85 {
                multiplier += 0.04;
            }
        }
        EvidenceCandidateSource::ChangeEvent => {
            let alignment = confidence_value(candidate, "time_alignment").0;
            if alignment >= 0.80 {
                multiplier += 0.12;
            } else if alignment >= 0.60 {
                multiplier += 0.05;
            } else if alignment < 0.35 {
                multiplier -= 0.20;
            }
            if change_record_for_candidate(input, candidate)
                .and_then(|record| str_field(&record.payload, "kind"))
                == Some("external_event")
            {
                multiplier += 0.20;
            }
        }
        EvidenceCandidateSource::TraceExemplar => {
            if confidence_value(candidate, "span_specificity").0 >= 0.85 {
                multiplier += 0.08;
            }
            if confidence_value(candidate, "path_specificity").0 >= 0.70 {
                multiplier += 0.06;
            }
        }
        EvidenceCandidateSource::DependencyEdge => {
            if let Some(relationship) = relationship_for_candidate(input, candidate) {
                multiplier += relationship_causal_bonus(entity, &relationship, relationships);
            }
            if confidence_value(candidate, "relationship_confidence").0 >= 0.80 {
                multiplier += 0.04;
            }
        }
        EvidenceCandidateSource::PreviousIncident => {
            if confidence_value(candidate, "signature_similarity").0 >= 0.80 {
                multiplier += 0.10;
            }
        }
        EvidenceCandidateSource::MissingData | EvidenceCandidateSource::CounterEvidence => {}
    }

    if candidate.item.direction != EvidenceDirection::Supports {
        multiplier -= 0.10;
    }

    multiplier.clamp(0.55, 1.35)
}

fn candidate_structurally_mentions_entity(
    candidate: &EvidenceCandidate,
    entity: &str,
    relationships: &[ResolvedRelationship],
) -> bool {
    candidate.item.entities.iter().any(|candidate_entity| {
        candidate_entity == entity
            || canonical_suspect_entity(candidate_entity, relationships) == entity
    })
}

fn relationship_causal_bonus(
    entity: &str,
    relationship: &ResolvedRelationship,
    relationships: &[ResolvedRelationship],
) -> f64 {
    let src = canonical_suspect_entity(&relationship.src, relationships);
    let dst = canonical_suspect_entity(&relationship.dst, relationships);
    match relationship.relationship_type {
        RelationshipType::ReadsFrom
        | RelationshipType::WritesTo
        | RelationshipType::DependsOn
        | RelationshipType::Retries
        | RelationshipType::FansOutTo => {
            if dst == entity {
                0.10
            } else if src == entity {
                0.03
            } else {
                0.0
            }
        }
        RelationshipType::Calls | RelationshipType::SharesResourceWith => {
            if src == entity || dst == entity {
                0.05
            } else {
                0.0
            }
        }
        RelationshipType::RunsOn | RelationshipType::DeployedAs | RelationshipType::Emits => 0.0,
    }
}

fn reason_tokens_for_candidate(
    input: EvidenceCompilerInput<'_>,
    candidate: &EvidenceCandidate,
) -> Vec<String> {
    match candidate.source {
        EvidenceCandidateSource::MetricAnomaly => metric_reason_tokens(input, candidate),
        EvidenceCandidateSource::LogPattern => log_reason_tokens(input, candidate),
        EvidenceCandidateSource::ChangeEvent => change_reason_tokens(input, candidate),
        EvidenceCandidateSource::TraceExemplar => trace_reason_tokens(candidate),
        EvidenceCandidateSource::DependencyEdge => vec!["dependency_direction".to_string()],
        EvidenceCandidateSource::PreviousIncident => vec!["prior_incident_match".to_string()],
        EvidenceCandidateSource::MissingData => vec!["telemetry_gap_across_peak".to_string()],
        EvidenceCandidateSource::CounterEvidence => counter_reason_tokens(candidate),
    }
}

fn metric_reason_tokens(
    input: EvidenceCompilerInput<'_>,
    candidate: &EvidenceCandidate,
) -> Vec<String> {
    let signal = anomaly_window_for_candidate(input, candidate)
        .map(|window| window.signal.as_str())
        .unwrap_or(candidate.item.claim.as_str())
        .to_ascii_lowercase();
    let mut reasons = Vec::new();

    if signal.contains("lock") {
        reasons.push("lock_metric".to_string());
    } else if signal.contains("hit_ratio") {
        reasons.push("hit_ratio_collapse".to_string());
    } else if signal.contains("retry") {
        reasons.push("retry_rate_tracks_load".to_string());
    } else if signal.contains("rss") || signal.contains("memory") {
        reasons.push("sawtooth_rss".to_string());
    } else if signal.contains("restart") {
        reasons.push("restart_aligned_errors".to_string());
    } else if signal.contains("queue.depth") {
        reasons.push("hot_partition".to_string());
    } else if signal.contains("request.rate") {
        reasons.push("tenant_ramp_time_aligned".to_string());
    } else if signal.contains("duration") || signal.contains("latency") {
        reasons.push("latency_spike".to_string());
    } else if signal.contains("error_rate") {
        reasons.push("error_rate_spike".to_string());
    } else {
        reasons.push("metric_anomaly".to_string());
    }

    if confidence_value(candidate, "magnitude").0 >= 0.75 {
        reasons.push("time_alignment".to_string());
    }

    dedupe_stable(reasons)
}

fn log_reason_tokens(
    input: EvidenceCompilerInput<'_>,
    candidate: &EvidenceCandidate,
) -> Vec<String> {
    let text = log_pattern_for_candidate(input, candidate)
        .map(|pattern| pattern.template.as_str())
        .unwrap_or(candidate.item.claim.as_str())
        .to_ascii_lowercase();

    if text.contains("nullpointer") || text.contains("exception") {
        vec!["error_signature".to_string()]
    } else if text.contains("deadline") {
        vec!["deadline_exceeded_signature".to_string()]
    } else if text.contains("42703") || text.contains("undefinedcolumn") || text.contains("column")
    {
        vec!["error_signature_42703".to_string()]
    } else if text.contains("oom") {
        vec!["oom_logs".to_string()]
    } else if text.contains("retry") || text.contains("no backoff") {
        vec!["retry_fanout_trace".to_string()]
    } else if text.contains("lock") {
        vec!["lock_metric".to_string()]
    } else if text.contains("queue depth") {
        vec!["hot_partition".to_string()]
    } else {
        vec!["log_cluster".to_string()]
    }
}

fn change_reason_tokens(
    input: EvidenceCompilerInput<'_>,
    candidate: &EvidenceCandidate,
) -> Vec<String> {
    let record = change_record_for_candidate(input, candidate);
    let kind = record
        .and_then(|record| str_field(&record.payload, "kind"))
        .unwrap_or_default();
    let summary = record
        .and_then(|record| str_field(&record.payload, "summary"))
        .unwrap_or_default()
        .to_ascii_lowercase();

    if candidate.item.direction == EvidenceDirection::Weakens {
        vec![
            "onset_precedes_change".to_string(),
            "change_proximity".to_string(),
        ]
    } else if kind == "config_change" {
        vec!["config_change_proximity".to_string()]
    } else if kind == "schema_migration" {
        vec!["exact_time_alignment".to_string()]
    } else if kind == "external_event" {
        vec!["provider_status_event".to_string()]
    } else if kind == "traffic_shift" || summary.contains("traffic") {
        vec!["tenant_ramp_time_aligned".to_string()]
    } else if summary.contains("oom") || summary.contains("restart") {
        vec!["restart_aligned_errors".to_string()]
    } else if kind == "deploy" {
        vec!["change_proximity".to_string(), "deploy_origin".to_string()]
    } else {
        vec!["change_proximity".to_string()]
    }
}

fn trace_reason_tokens(candidate: &EvidenceCandidate) -> Vec<String> {
    let mut reasons = vec!["trace_exemplar".to_string()];

    if confidence_value(candidate, "path_specificity").0 >= 0.70
        || candidate.item.entities.len() > 1
    {
        reasons.push("dependency_direction".to_string());
    }

    if confidence_value(candidate, "span_specificity").0 >= 0.85 {
        reasons.push("span_specificity".to_string());
    }

    dedupe_stable(reasons)
}

fn counter_reason_tokens(candidate: &EvidenceCandidate) -> Vec<String> {
    let mut reasons = vec!["counter_evidence".to_string()];

    if has_source_signal(&candidate.item, SourceSignal::Metric) {
        reasons.push("counter_metric".to_string());
    }
    if has_source_signal(&candidate.item, SourceSignal::Trace) {
        reasons.push("counter_trace".to_string());
    }
    if has_source_signal(&candidate.item, SourceSignal::Change) {
        reasons.push("onset_precedes_change".to_string());
    }
    if has_source_signal(&candidate.item, SourceSignal::Relationship) {
        reasons.push("dependency_direction".to_string());
    }

    dedupe_stable(reasons)
}

fn source_ref_quality(store: &HotContextStore, item: &EvidenceItem) -> UnitInterval {
    if item.source_refs.is_empty() {
        return UnitInterval(0.0);
    }

    let found = item
        .source_refs
        .iter()
        .filter(|source_ref| {
            matches!(
                store.resolve_source_ref(source_ref),
                SourceResolution::Found(_)
            )
        })
        .count();
    let mut signals = Vec::new();
    for source_ref in item.source_refs.iter() {
        if !signals.contains(&source_ref.signal) {
            signals.push(source_ref.signal);
        }
    }
    let signal_count = signals.len();
    let found_ratio = found as f64 / item.source_refs.0.len() as f64;
    let diversity_bonus = if signal_count > 1 { 0.05 } else { 0.0 };

    UnitInterval((found_ratio + diversity_bonus).clamp(0.0, 1.0))
}

fn weighted_unit_interval(parts: &[(UnitInterval, f64)]) -> UnitInterval {
    let (weighted_sum, weight_sum) =
        parts
            .iter()
            .fold((0.0, 0.0), |(weighted_sum, weight_sum), (value, weight)| {
                (weighted_sum + value.0 * weight, weight_sum + weight)
            });

    if weight_sum <= 0.0 {
        UnitInterval(0.0)
    } else {
        UnitInterval((weighted_sum / weight_sum).clamp(0.0, 1.0))
    }
}

fn anomaly_magnitude_strength(
    window: &crate::derived_context::DerivedAnomalyWindow,
) -> UnitInterval {
    let Some(baseline) = window.baseline else {
        return UnitInterval(
            if window
                .note
                .as_deref()
                .is_some_and(|note| note.to_ascii_lowercase().contains("uncertain"))
            {
                0.35
            } else {
                0.55
            },
        );
    };

    let mut magnitudes = Vec::new();
    if let Some(peak) = window.peak.or(window.peak_observed) {
        let denominator = baseline.abs().max(0.01);
        magnitudes.push((peak - baseline).abs() / denominator);
    }
    if let Some(trough) = window.trough {
        if baseline.abs() <= 1.0 {
            magnitudes.push(((baseline - trough).abs() / baseline.abs().max(0.01)) * 1.5);
        } else {
            magnitudes.push((baseline - trough).abs() / baseline.abs().max(1.0));
        }
    }

    let magnitude = magnitudes.into_iter().fold(0.0_f64, f64::max);
    UnitInterval((magnitude / (magnitude + 1.0)).clamp(0.0, 1.0))
}

fn severity_strength(severity: &str) -> UnitInterval {
    match severity.to_ascii_uppercase().as_str() {
        "FATAL" => UnitInterval(1.0),
        "ERROR" => UnitInterval(0.95),
        "WARN" | "WARNING" => UnitInterval(0.72),
        "INFO" => UnitInterval(0.45),
        _ => UnitInterval(0.55),
    }
}

fn change_time_alignment(
    input: EvidenceCompilerInput<'_>,
    candidate: &EvidenceCandidate,
) -> UnitInterval {
    let Some(record) = change_record_for_candidate(input, candidate) else {
        return UnitInterval(0.50);
    };
    let Some(event) = input
        .derived
        .timeline
        .iter()
        .find(|event| event.source_ref == record.key.as_str())
    else {
        return if record
            .time_window
            .as_ref()
            .is_some_and(|window| window_overlaps_query(window, input.query))
        {
            UnitInterval(0.65)
        } else {
            UnitInterval(0.10)
        };
    };

    match event.marker {
        TimelineMarker::Change | TimelineMarker::Trigger => UnitInterval(0.95),
        TimelineMarker::NonCausalChange => UnitInterval(0.95),
        TimelineMarker::Recovery => UnitInterval(0.45),
        TimelineMarker::Symptom | TimelineMarker::Propagation | TimelineMarker::Amplification => {
            UnitInterval(0.70)
        }
        TimelineMarker::DataGap => UnitInterval(0.35),
    }
}

fn confidence_value(candidate: &EvidenceCandidate, key: &str) -> UnitInterval {
    candidate
        .item
        .confidence
        .get(key)
        .copied()
        .unwrap_or(UnitInterval(0.0))
}

fn has_source_signal(item: &EvidenceItem, signal: SourceSignal) -> bool {
    item.source_refs
        .iter()
        .any(|source_ref| source_ref.signal == signal)
}

fn note_contains(item: &EvidenceItem, needle: &str) -> bool {
    item.note
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .contains(needle)
        || item.claim.to_ascii_lowercase().contains(needle)
}

fn window_overlaps_query(window: &TimeWindow, query: &EvidenceQuery) -> bool {
    window.start.as_str() <= query.time_window.end.as_str()
        && query.time_window.start.as_str() <= window.end.as_str()
}

fn anomaly_window_for_candidate<'a>(
    input: EvidenceCompilerInput<'a>,
    candidate: &EvidenceCandidate,
) -> Option<&'a crate::derived_context::DerivedAnomalyWindow> {
    let window_id = candidate
        .item
        .source_refs
        .iter()
        .find(|source_ref| source_ref.signal == SourceSignal::AnomalyWindow)
        .map(|source_ref| source_ref.r#ref.as_str())?;

    input
        .derived
        .anomaly_windows
        .iter()
        .find(|window| window.id == window_id)
}

fn log_pattern_for_candidate<'a>(
    input: EvidenceCompilerInput<'a>,
    candidate: &EvidenceCandidate,
) -> Option<&'a crate::derived_context::DerivedLogPattern> {
    let pattern_id = candidate
        .item
        .source_refs
        .iter()
        .find(|source_ref| source_ref.signal == SourceSignal::LogPattern)
        .map(|source_ref| source_ref.r#ref.as_str())?;

    input
        .derived
        .log_patterns
        .iter()
        .find(|pattern| pattern.id == pattern_id)
}

fn change_record_for_candidate<'a>(
    input: EvidenceCompilerInput<'a>,
    candidate: &EvidenceCandidate,
) -> Option<&'a StoredRecord> {
    candidate
        .item
        .source_refs
        .iter()
        .find(|source_ref| source_ref.signal == SourceSignal::Change)
        .and_then(
            |source_ref| match input.store.resolve_source_ref(source_ref) {
                SourceResolution::Found(record) if record.kind == StoredRecordKind::Change => {
                    Some(record)
                }
                _ => None,
            },
        )
}

fn relationship_for_candidate(
    input: EvidenceCompilerInput<'_>,
    candidate: &EvidenceCandidate,
) -> Option<ResolvedRelationship> {
    candidate
        .item
        .source_refs
        .iter()
        .find(|source_ref| source_ref.signal == SourceSignal::Relationship)
        .and_then(
            |source_ref| match input.store.resolve_source_ref(source_ref) {
                SourceResolution::Found(record)
                    if record.kind == StoredRecordKind::Relationship =>
                {
                    serde_json::from_value::<ResolvedRelationship>(record.payload.clone()).ok()
                }
                _ => None,
            },
        )
}

fn prior_incident_similarity(
    input: EvidenceCompilerInput<'_>,
    candidate: &EvidenceCandidate,
) -> Option<UnitInterval> {
    let prior_id = candidate
        .item
        .source_refs
        .iter()
        .find(|source_ref| source_ref.signal == SourceSignal::PriorIncident)
        .map(|source_ref| source_ref.r#ref.as_str())?;

    input
        .derived
        .related_anomalies
        .as_ref()?
        .related
        .iter()
        .find(|related| related.prior_incident.as_deref() == Some(prior_id))
        .and_then(|related| related.similarity)
}

fn resolved_relationships(store: &HotContextStore) -> Vec<ResolvedRelationship> {
    store
        .records()
        .iter()
        .filter(|record| record.kind == StoredRecordKind::Relationship)
        .filter_map(|record| {
            serde_json::from_value::<ResolvedRelationship>(record.payload.clone()).ok()
        })
        .collect()
}

fn push_metric_anomaly_candidates(
    input: EvidenceCompilerInput<'_>,
    candidates: &mut Vec<EvidenceCandidate>,
) -> Result<(), EvidenceCompileError> {
    for window in &input.derived.anomaly_windows {
        let mut source_refs = Vec::new();
        push_resolvable_source_ref(
            input.store,
            &mut source_refs,
            SourceSignal::Metric,
            &metric_series_ref(&window.signal, &window.entity),
        );
        push_resolvable_source_ref(
            input.store,
            &mut source_refs,
            SourceSignal::AnomalyWindow,
            &window.id,
        );
        push_inferred_source_refs(input.store, &mut source_refs, &window.source_refs);

        if source_refs.is_empty() {
            continue;
        }

        let mut confidence = BTreeMap::new();
        confidence.insert("detector".to_string(), window.detector_confidence);

        push_candidate(
            candidates,
            EvidenceCandidateSource::MetricAnomaly,
            EvidenceItemDraft {
                claim: format!(
                    "{} {} anomaly spans {} to {}.",
                    window.entity,
                    window.signal,
                    window
                        .start
                        .as_deref()
                        .unwrap_or(input.query.time_window.start.as_str()),
                    window
                        .end
                        .as_deref()
                        .unwrap_or(input.query.time_window.end.as_str())
                ),
                kind: EvidenceKind::MetricAnomaly,
                direction: EvidenceDirection::Supports,
                strength: window.detector_confidence,
                time_window: optional_window_or_query(
                    window.start.as_deref(),
                    window.end.as_deref(),
                    input.query,
                ),
                entities: vec![window.entity.clone()],
                source_refs,
                freshness: EvidenceFreshness::Changing,
                missing_data: Vec::new(),
                privacy_scope: privacy_scope(input.query),
                confidence,
                note: window.note.clone(),
            },
        )?;
    }

    Ok(())
}

fn push_log_pattern_candidates(
    input: EvidenceCompilerInput<'_>,
    candidates: &mut Vec<EvidenceCandidate>,
) -> Result<(), EvidenceCompileError> {
    for pattern in &input.derived.log_patterns {
        let mut source_refs = Vec::new();
        push_resolvable_source_ref(
            input.store,
            &mut source_refs,
            SourceSignal::LogPattern,
            &pattern.id,
        );
        for exemplar in &pattern.exemplars {
            push_resolvable_source_ref(input.store, &mut source_refs, SourceSignal::Log, exemplar);
        }
        push_inferred_source_refs(input.store, &mut source_refs, &pattern.source_refs);

        if source_refs.is_empty() {
            continue;
        }

        push_candidate(
            candidates,
            EvidenceCandidateSource::LogPattern,
            EvidenceItemDraft {
                claim: format!(
                    "{} log pattern for {} occurred {} time(s): {}",
                    pattern.severity, pattern.entity, pattern.count, pattern.template
                ),
                kind: EvidenceKind::LogCluster,
                direction: EvidenceDirection::Supports,
                strength: log_pattern_strength(pattern.severity.as_str(), pattern.count),
                time_window: TimeWindow {
                    start: pattern.first_seen.clone(),
                    end: pattern.last_seen.clone(),
                },
                entities: vec![pattern.entity.clone()],
                source_refs,
                freshness: if pattern.stability.contains("new") {
                    EvidenceFreshness::Changing
                } else {
                    EvidenceFreshness::Settled
                },
                missing_data: Vec::new(),
                privacy_scope: privacy_scope(input.query),
                confidence: BTreeMap::new(),
                note: Some(pattern.stability.clone()),
            },
        )?;
    }

    Ok(())
}

fn push_change_event_candidates(
    input: EvidenceCompilerInput<'_>,
    candidates: &mut Vec<EvidenceCandidate>,
) -> Result<(), EvidenceCompileError> {
    for record in input
        .store
        .raw_source_records()
        .filter(|record| record.kind == StoredRecordKind::Change)
    {
        let mut source_refs = Vec::new();
        push_resolvable_source_ref(
            input.store,
            &mut source_refs,
            SourceSignal::Change,
            record.key.as_str(),
        );

        let timeline_event = input
            .derived
            .timeline
            .iter()
            .find(|event| event.source_ref == record.key.as_str());
        if let Some(event) = timeline_event {
            push_inferred_source_refs(input.store, &mut source_refs, &event.source_refs);
        }

        if source_refs.is_empty() {
            continue;
        }

        let is_non_causal =
            timeline_event.is_some_and(|event| event.marker == TimelineMarker::NonCausalChange);
        let source = if is_non_causal {
            EvidenceCandidateSource::CounterEvidence
        } else {
            EvidenceCandidateSource::ChangeEvent
        };
        let kind = if is_non_causal {
            EvidenceKind::CounterEvidence
        } else {
            EvidenceKind::ChangeEvent
        };
        let direction = if is_non_causal {
            EvidenceDirection::Weakens
        } else {
            EvidenceDirection::Supports
        };
        let mut confidence = BTreeMap::new();
        confidence.insert("source_record".to_string(), UnitInterval(1.0));

        push_candidate(
            candidates,
            source,
            EvidenceItemDraft {
                claim: change_claim(record, is_non_causal),
                kind,
                direction,
                strength: if is_non_causal {
                    UnitInterval(0.78)
                } else {
                    UnitInterval(0.72)
                },
                time_window: record_window_or_query(record, input.query),
                entities: record_entities(record),
                source_refs,
                freshness: EvidenceFreshness::Settled,
                missing_data: Vec::new(),
                privacy_scope: privacy_scope(input.query),
                confidence,
                note: timeline_event.map(|event| event.text.clone()),
            },
        )?;
    }

    Ok(())
}

fn push_trace_exemplar_candidates(
    input: EvidenceCompilerInput<'_>,
    candidates: &mut Vec<EvidenceCandidate>,
) -> Result<(), EvidenceCompileError> {
    for record in input
        .store
        .raw_source_records()
        .filter(|record| record.kind == StoredRecordKind::Trace)
    {
        let error_span_refs = trace_error_span_refs(record);
        let exemplar_of = str_field(&record.payload, "exemplar_of");
        let is_exemplar = exemplar_of.is_some_and(|value| value != "baseline");

        if error_span_refs.is_empty() && !is_exemplar {
            continue;
        }

        let mut source_refs = Vec::new();
        if error_span_refs.is_empty() {
            push_resolvable_source_ref(
                input.store,
                &mut source_refs,
                SourceSignal::Trace,
                record.key.as_str(),
            );
        } else {
            for raw_ref in &error_span_refs {
                push_resolvable_source_ref(
                    input.store,
                    &mut source_refs,
                    SourceSignal::Trace,
                    raw_ref,
                );
            }
        }

        if source_refs.is_empty() {
            continue;
        }

        push_candidate(
            candidates,
            EvidenceCandidateSource::TraceExemplar,
            EvidenceItemDraft {
                claim: format!(
                    "Trace {} contains {} error span(s).",
                    record.key.as_str(),
                    error_span_refs.len().max(1)
                ),
                kind: EvidenceKind::TraceExemplar,
                direction: EvidenceDirection::Supports,
                strength: UnitInterval(0.78),
                time_window: record_window_or_query(record, input.query),
                entities: record_entities(record),
                source_refs,
                freshness: EvidenceFreshness::Settled,
                missing_data: Vec::new(),
                privacy_scope: privacy_scope(input.query),
                confidence: BTreeMap::new(),
                note: exemplar_of.map(str::to_string),
            },
        )?;
    }

    Ok(())
}

fn push_dependency_edge_candidates(
    input: EvidenceCompilerInput<'_>,
    candidates: &mut Vec<EvidenceCandidate>,
) -> Result<(), EvidenceCompileError> {
    for record in input
        .store
        .records()
        .iter()
        .filter(|record| record.kind == StoredRecordKind::Relationship)
    {
        let Ok(relationship) =
            serde_json::from_value::<ResolvedRelationship>(record.payload.clone())
        else {
            continue;
        };

        let mut source_refs = Vec::new();
        push_resolvable_source_ref(
            input.store,
            &mut source_refs,
            SourceSignal::Relationship,
            record.key.as_str(),
        );
        push_inferred_source_refs(input.store, &mut source_refs, &relationship.evidence);

        if source_refs.is_empty() {
            continue;
        }

        push_candidate(
            candidates,
            EvidenceCandidateSource::DependencyEdge,
            EvidenceItemDraft {
                claim: format!(
                    "{} {} {} relationship is source-backed.",
                    relationship.src,
                    relationship.relationship_type.as_str(),
                    relationship.dst
                ),
                kind: EvidenceKind::DependencyEdge,
                direction: EvidenceDirection::Neutral,
                strength: relationship.confidence,
                time_window: input.query.time_window.clone(),
                entities: dedupe_stable(vec![relationship.src.clone(), relationship.dst.clone()]),
                source_refs,
                freshness: EvidenceFreshness::Settled,
                missing_data: Vec::new(),
                privacy_scope: privacy_scope(input.query),
                confidence: BTreeMap::new(),
                note: None,
            },
        )?;
    }

    Ok(())
}

fn push_previous_incident_candidates(
    input: EvidenceCompilerInput<'_>,
    candidates: &mut Vec<EvidenceCandidate>,
) -> Result<(), EvidenceCompileError> {
    for record in input
        .store
        .raw_source_records()
        .filter(|record| record.kind == StoredRecordKind::PriorIncident)
    {
        let mut source_refs = Vec::new();
        push_resolvable_source_ref(
            input.store,
            &mut source_refs,
            SourceSignal::PriorIncident,
            record.key.as_str(),
        );

        if source_refs.is_empty() {
            continue;
        }

        let mut entities = record_entities(record);
        if let Some(primary_entity) = record
            .payload
            .pointer("/signature/primary_entity")
            .and_then(Value::as_str)
        {
            entities.push(primary_entity.to_string());
        }

        push_candidate(
            candidates,
            EvidenceCandidateSource::PreviousIncident,
            EvidenceItemDraft {
                claim: prior_incident_claim(record),
                kind: EvidenceKind::PreviousIncident,
                direction: EvidenceDirection::Supports,
                strength: UnitInterval(0.72),
                time_window: record_window_or_query(record, input.query),
                entities: dedupe_stable(entities),
                source_refs,
                freshness: EvidenceFreshness::Settled,
                missing_data: Vec::new(),
                privacy_scope: privacy_scope(input.query),
                confidence: BTreeMap::new(),
                note: str_field(&record.payload, "mitigation").map(str::to_string),
            },
        )?;
    }

    Ok(())
}

fn push_missing_data_candidates(
    input: EvidenceCompilerInput<'_>,
    candidates: &mut Vec<EvidenceCandidate>,
) -> Result<(), EvidenceCompileError> {
    for record in input
        .store
        .raw_source_records()
        .filter(|record| record.kind == StoredRecordKind::TelemetryGap)
    {
        let mut source_refs = Vec::new();
        push_resolvable_source_ref(
            input.store,
            &mut source_refs,
            SourceSignal::TelemetryGap,
            record.key.as_str(),
        );
        if let Some(cause) = str_field(&record.payload, "cause") {
            push_resolvable_source_ref(input.store, &mut source_refs, SourceSignal::Change, cause);
        }

        if source_refs.is_empty() {
            continue;
        }

        push_candidate(
            candidates,
            EvidenceCandidateSource::MissingData,
            EvidenceItemDraft {
                claim: missing_data_claim(record),
                kind: EvidenceKind::MissingData,
                direction: EvidenceDirection::Neutral,
                strength: UnitInterval(0.70),
                time_window: record_window_or_query(record, input.query),
                entities: record_entities(record),
                source_refs,
                freshness: EvidenceFreshness::Changing,
                missing_data: string_array_field(&record.payload, "affected_signals"),
                privacy_scope: privacy_scope(input.query),
                confidence: BTreeMap::new(),
                note: str_field(&record.payload, "note").map(str::to_string),
            },
        )?;
    }

    Ok(())
}

fn push_counter_evidence_candidates(
    input: EvidenceCompilerInput<'_>,
    candidates: &mut Vec<EvidenceCandidate>,
) -> Result<(), EvidenceCompileError> {
    let Some(comparison) = &input.derived.window_comparison else {
        return Ok(());
    };

    for delta in comparison.deltas.iter().filter(is_counter_evidence_delta) {
        let mut source_refs = Vec::new();
        push_resolvable_source_ref(
            input.store,
            &mut source_refs,
            SourceSignal::Metric,
            &metric_series_ref(&delta.signal, &delta.entity),
        );
        push_inferred_source_refs(input.store, &mut source_refs, &delta.source_refs);

        if source_refs.is_empty() {
            continue;
        }

        push_candidate(
            candidates,
            EvidenceCandidateSource::CounterEvidence,
            EvidenceItemDraft {
                claim: delta.note.clone().unwrap_or_else(|| {
                    format!(
                        "{} {} stayed comparatively flat.",
                        delta.entity, delta.signal
                    )
                }),
                kind: EvidenceKind::CounterEvidence,
                direction: EvidenceDirection::Weakens,
                strength: UnitInterval(0.75),
                time_window: comparison.anomalous.clone(),
                entities: vec![delta.entity.clone()],
                source_refs,
                freshness: EvidenceFreshness::Settled,
                missing_data: Vec::new(),
                privacy_scope: privacy_scope(input.query),
                confidence: BTreeMap::new(),
                note: delta.note.clone(),
            },
        )?;
    }

    Ok(())
}

struct EvidenceItemDraft {
    claim: String,
    kind: EvidenceKind,
    direction: EvidenceDirection,
    strength: UnitInterval,
    time_window: TimeWindow,
    entities: Vec<String>,
    source_refs: Vec<SourceRef>,
    freshness: EvidenceFreshness,
    missing_data: Vec<String>,
    privacy_scope: String,
    confidence: BTreeMap<String, UnitInterval>,
    note: Option<String>,
}

fn push_candidate(
    candidates: &mut Vec<EvidenceCandidate>,
    source: EvidenceCandidateSource,
    draft: EvidenceItemDraft,
) -> Result<(), EvidenceCompileError> {
    let candidate_id = format!("cand-{:03}", candidates.len() + 1);
    let mut item = EvidenceItem {
        id: candidate_id.clone(),
        claim: draft.claim,
        kind: draft.kind,
        direction: draft.direction,
        strength: clamp_unit_interval(draft.strength),
        time_window: draft.time_window,
        entities: dedupe_stable(draft.entities),
        source_refs: SourceRefs(draft.source_refs),
        freshness: draft.freshness,
        missing_data: dedupe_stable(draft.missing_data),
        token_cost: 0,
        privacy_scope: draft.privacy_scope,
        confidence: draft.confidence,
        note: draft.note,
    };

    item.token_cost = estimate_evidence_item_tokens(&item)?;
    candidates.push(EvidenceCandidate {
        candidate_id,
        item,
        source,
    });

    Ok(())
}

fn push_resolvable_source_ref(
    store: &HotContextStore,
    refs: &mut Vec<SourceRef>,
    signal: SourceSignal,
    raw_ref: &str,
) {
    if raw_ref.trim().is_empty() {
        return;
    }

    let source_ref = SourceRef {
        signal,
        r#ref: raw_ref.to_string(),
    };

    if matches!(
        store.resolve_source_ref(&source_ref),
        SourceResolution::Found(_)
    ) && !refs.iter().any(|existing| existing == &source_ref)
    {
        refs.push(source_ref);
    }
}

fn push_inferred_source_refs(
    store: &HotContextStore,
    refs: &mut Vec<SourceRef>,
    raw_refs: &[String],
) {
    for raw_ref in raw_refs {
        if let Some(signal) = infer_source_signal(raw_ref) {
            push_resolvable_source_ref(store, refs, signal, raw_ref);
        }
    }
}

fn infer_source_signal(raw_ref: &str) -> Option<SourceSignal> {
    if raw_ref.starts_with("aw-") {
        Some(SourceSignal::AnomalyWindow)
    } else if raw_ref.starts_with("change:") {
        Some(SourceSignal::Change)
    } else if raw_ref.starts_with("log-") {
        Some(SourceSignal::Log)
    } else if raw_ref.starts_with("lp-") {
        Some(SourceSignal::LogPattern)
    } else if raw_ref.starts_with("prior:") {
        Some(SourceSignal::PriorIncident)
    } else if raw_ref.starts_with("telemetry_gap:") {
        Some(SourceSignal::TelemetryGap)
    } else if raw_ref.starts_with("relationship:") {
        Some(SourceSignal::Relationship)
    } else if raw_ref.contains('@') {
        Some(SourceSignal::Metric)
    } else if raw_ref.starts_with("trace:") || raw_ref.contains('/') || raw_ref.starts_with("t-") {
        Some(SourceSignal::Trace)
    } else {
        None
    }
}

fn record_window_or_query(record: &StoredRecord, query: &EvidenceQuery) -> TimeWindow {
    record
        .time_window
        .clone()
        .unwrap_or_else(|| query.time_window.clone())
}

fn optional_window_or_query(
    start: Option<&str>,
    end: Option<&str>,
    query: &EvidenceQuery,
) -> TimeWindow {
    match (start, end) {
        (Some(start), Some(end)) => TimeWindow {
            start: start.to_string(),
            end: end.to_string(),
        },
        _ => query.time_window.clone(),
    }
}

fn record_entities(record: &StoredRecord) -> Vec<String> {
    record.entities.clone()
}

fn privacy_scope(query: &EvidenceQuery) -> String {
    query
        .privacy_scope
        .clone()
        .unwrap_or_else(|| "none".to_string())
}

fn log_pattern_strength(severity: &str, count: usize) -> UnitInterval {
    let severity_score = match severity {
        "ERROR" | "FATAL" => 0.82,
        "WARN" | "WARNING" => 0.68,
        _ => 0.55,
    };
    let count_bonus = if count >= 10 {
        0.08
    } else if count >= 3 {
        0.04
    } else {
        0.0
    };

    UnitInterval(severity_score + count_bonus)
}

fn change_claim(record: &StoredRecord, is_non_causal: bool) -> String {
    let summary = str_field(&record.payload, "summary").unwrap_or(record.key.as_str());
    let entity = str_field(&record.payload, "entity").unwrap_or("unknown entity");

    if is_non_causal {
        format!("{summary} is present on {entity}, but derived timing marks it non-causal.")
    } else {
        format!("{summary} changed {entity}.")
    }
}

fn trace_error_span_refs(record: &StoredRecord) -> Vec<String> {
    let Some(spans) = record.payload.get("spans").and_then(Value::as_array) else {
        return Vec::new();
    };

    spans
        .iter()
        .filter(|span| str_field(span, "status").is_some_and(|status| status == "ERROR"))
        .filter_map(|span| str_field(span, "span_id"))
        .map(|span_id| format!("{}/{}", record.key.as_str(), span_id))
        .collect()
}

fn prior_incident_claim(record: &StoredRecord) -> String {
    str_field(&record.payload, "summary")
        .or_else(|| str_field(&record.payload, "title"))
        .map(|summary| format!("Prior incident {} matches: {summary}", record.key.as_str()))
        .unwrap_or_else(|| format!("Prior incident {} may be relevant.", record.key.as_str()))
}

fn missing_data_claim(record: &StoredRecord) -> String {
    let affected = string_array_field(&record.payload, "affected_signals");
    if affected.is_empty() {
        format!(
            "Telemetry gap {} overlaps the investigation window.",
            record.key.as_str()
        )
    } else {
        format!(
            "Telemetry gap {} affects {}.",
            record.key.as_str(),
            affected.join(", ")
        )
    }
}

fn is_counter_evidence_delta(delta: &&WindowDelta) -> bool {
    let delta = *delta;
    let note_marks_counter = delta.note.as_ref().is_some_and(|note| {
        let lower = note.to_ascii_lowercase();
        lower.contains("flat") || lower.contains("counter")
    });
    let factor_is_flat = delta.factor.is_some_and(|factor| factor <= 1.20);
    let absolute_is_flat = (delta.to - delta.from).abs() <= delta.from.abs().max(1.0) * 0.10;

    note_marks_counter || factor_is_flat || absolute_is_flat
}

fn string_array_field(value: &Value, field: &str) -> Vec<String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn str_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
}

fn clamp_unit_interval(value: UnitInterval) -> UnitInterval {
    UnitInterval(value.0.clamp(0.0, 1.0))
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

fn parse_required_artifact<T>(
    case: &FixtureCase,
    artifact: &'static str,
) -> Result<T, EvidenceCompilationGoldError>
where
    T: for<'de> Deserialize<'de>,
{
    let Some(value) = case.expected.get(artifact) else {
        return Err(EvidenceCompilationGoldError::MissingArtifact {
            fixture_id: case.registry_entry.id.clone(),
            artifact,
        });
    };

    serde_json::from_value(value.clone()).map_err(|source| {
        EvidenceCompilationGoldError::ParseArtifact {
            fixture_id: case.registry_entry.id.clone(),
            artifact,
            source,
        }
    })
}

fn parse_optional_artifact<T>(
    case: &FixtureCase,
    artifact: &'static str,
) -> Result<Option<T>, EvidenceCompilationGoldError>
where
    T: for<'de> Deserialize<'de>,
{
    let Some(value) = case.expected.get(artifact) else {
        return Ok(None);
    };

    serde_json::from_value(value.clone())
        .map(Some)
        .map_err(|source| EvidenceCompilationGoldError::ParseArtifact {
            fixture_id: case.registry_entry.id.clone(),
            artifact,
            source,
        })
}

fn compare_bundle(
    expected: &EvidenceBundle,
    actual: &EvidenceBundle,
    comparison: &mut EvidenceCompilationComparison,
) {
    compare_exact(
        &mut comparison.bundle_mismatches,
        "bundle",
        "question",
        &expected.question,
        &actual.question,
    );
    compare_exact(
        &mut comparison.bundle_mismatches,
        "bundle",
        "hypothesis",
        &expected.hypothesis,
        &actual.hypothesis,
    );
    compare_exact(
        &mut comparison.bundle_mismatches,
        "bundle",
        "time_window",
        &expected.time_window,
        &actual.time_window,
    );

    compare_budget_structural(actual, comparison);

    compare_items(expected, actual, comparison);
}

fn compare_budget_structural(
    actual: &EvidenceBundle,
    comparison: &mut EvidenceCompilationComparison,
) {
    if actual.items.len() > actual.budget.max_items as usize {
        comparison
            .bundle_mismatches
            .push(EvidenceCompilationFieldMismatch {
                artifact: "bundle".to_string(),
                field: "budget.max_items".to_string(),
                expected: Value::String("selected items within max_items".to_string()),
                actual: Some(Value::from(actual.items.len())),
            });
    }

    if actual.budget.tokens_used > actual.budget.max_tokens {
        comparison
            .bundle_mismatches
            .push(EvidenceCompilationFieldMismatch {
                artifact: "bundle".to_string(),
                field: "budget.max_tokens".to_string(),
                expected: Value::String("tokens_used within max_tokens".to_string()),
                actual: Some(Value::from(actual.budget.tokens_used)),
            });
    }

    let actual_token_sum = actual
        .items
        .iter()
        .fold(0u32, |sum, item| sum.saturating_add(item.token_cost));
    if actual.budget.tokens_used != actual_token_sum {
        comparison
            .bundle_mismatches
            .push(EvidenceCompilationFieldMismatch {
                artifact: "bundle".to_string(),
                field: "budget.tokens_used".to_string(),
                expected: Value::from(actual_token_sum),
                actual: Some(Value::from(actual.budget.tokens_used)),
            });
    }
}

fn compare_items(
    expected: &EvidenceBundle,
    actual: &EvidenceBundle,
    comparison: &mut EvidenceCompilationComparison,
) {
    let mut used_actual = BTreeSet::<usize>::new();

    for (expected_index, expected_item) in expected.items.iter().enumerate() {
        let Some(actual_index) =
            best_structural_item_match(expected_item, &actual.items, &used_actual)
        else {
            if actual.items.is_empty() {
                comparison
                    .item_order_mismatches
                    .push(EvidenceItemOrderMismatch {
                        index: expected_index,
                        expected: Some(expected_item.id.clone()),
                        actual: None,
                    });
            }
            continue;
        };

        used_actual.insert(actual_index);
        compare_item(expected_item, &actual.items[actual_index], comparison);
    }
}

fn best_structural_item_match(
    expected: &EvidenceItem,
    actual_items: &[EvidenceItem],
    used_actual: &BTreeSet<usize>,
) -> Option<usize> {
    actual_items
        .iter()
        .enumerate()
        .filter(|(index, _)| !used_actual.contains(index))
        .filter(|(_, actual)| {
            (!expected_counter_evidence(expected) || expected_counter_evidence(actual))
                && (expected.kind != EvidenceKind::MissingData
                    || actual.kind == EvidenceKind::MissingData
                    || !actual.missing_data.is_empty())
        })
        .filter_map(|(index, actual)| {
            let score = structural_item_match_score(expected, actual);
            (score >= 70).then_some((score, index))
        })
        .max_by(|(left_score, left_index), (right_score, right_index)| {
            left_score
                .cmp(right_score)
                .then_with(|| right_index.cmp(left_index))
        })
        .map(|(_, index)| index)
}

fn structural_item_match_score(expected: &EvidenceItem, actual: &EvidenceItem) -> u32 {
    let mut score = 0u32;

    if expected.id == actual.id {
        score += 1_000;
    }
    if expected.kind == actual.kind {
        score += 90;
    }
    if expected.direction == actual.direction {
        score += 35;
    }
    if source_ref_identity_overlap(expected, actual) {
        score += 90;
    }
    if source_signal_overlap(expected, actual) {
        score += 65;
    }
    if entity_structural_overlap(&expected.entities, &actual.entities) {
        score += 45;
    }
    if !expected.missing_data.is_empty() && !actual.missing_data.is_empty() {
        score += 30;
    }
    if expected.time_window == actual.time_window {
        score += 20;
    }

    score
}

fn compare_item(
    expected: &EvidenceItem,
    actual: &EvidenceItem,
    comparison: &mut EvidenceCompilationComparison,
) {
    let artifact = format!("item:{}", expected.id);

    if expected_counter_evidence(expected) && !expected_counter_evidence(actual) {
        comparison
            .item_mismatches
            .push(EvidenceCompilationFieldMismatch {
                artifact: artifact.clone(),
                field: "counter_evidence".to_string(),
                expected: Value::String("counter evidence item or direction".to_string()),
                actual: Some(serde_json::to_value(actual.kind).unwrap_or(Value::Null)),
            });
    }

    if expected.kind == EvidenceKind::MissingData
        && actual.kind != EvidenceKind::MissingData
        && actual.missing_data.is_empty()
    {
        comparison
            .item_mismatches
            .push(EvidenceCompilationFieldMismatch {
                artifact: artifact.clone(),
                field: "missing_data".to_string(),
                expected: Value::String("missing-data evidence".to_string()),
                actual: Some(serde_json::to_value(&actual.missing_data).unwrap_or(Value::Null)),
            });
    }

    if !expected.source_refs.is_empty() && actual.source_refs.is_empty() {
        comparison
            .item_mismatches
            .push(EvidenceCompilationFieldMismatch {
                artifact: artifact.clone(),
                field: "source_refs".to_string(),
                expected: Value::String("source-backed item".to_string()),
                actual: Some(Value::Array(Vec::new())),
            });
    }
    compare_text_structural(
        &mut comparison.item_mismatches,
        &mut comparison.text_differences,
        &artifact,
        "claim",
        expected.claim.as_str(),
        actual.claim.as_str(),
    );
    compare_optional_text_structural(
        &mut comparison.item_mismatches,
        &mut comparison.text_differences,
        &artifact,
        "note",
        expected.note.as_deref(),
        actual.note.as_deref(),
    );
}

fn compare_suspected_causes(
    expected: &EvidenceCompilation,
    actual: &EvidenceCompilation,
    comparison: &mut EvidenceCompilationComparison,
) {
    let actual_by_entity = actual
        .suspected_causes
        .iter()
        .map(|cause| (cause.entity.as_str(), cause))
        .collect::<BTreeMap<_, _>>();
    let expected_entities = expected
        .suspected_causes
        .iter()
        .map(|cause| cause.entity.as_str())
        .collect::<BTreeSet<_>>();
    let mut seen_expected_entities = BTreeSet::<&str>::new();

    for expected in &expected.suspected_causes {
        if !seen_expected_entities.insert(expected.entity.as_str()) {
            continue;
        }

        let Some(actual) = actual_by_entity.get(expected.entity.as_str()).copied() else {
            if expected.rank == 1 && !actual_top_is_under_determined(&actual.suspected_causes) {
                comparison.missing_suspected_causes.push(expected.rank);
            }
            continue;
        };

        compare_suspected_cause(expected, actual, comparison);
    }

    for actual in &actual.suspected_causes {
        if !expected_entities.contains(actual.entity.as_str())
            && is_material_extra_cause(actual, expected.suspected_causes.len())
        {
            comparison.extra_suspected_causes.push(actual.rank);
        }
    }
}

fn compare_suspected_cause(
    expected: &SuspectedCause,
    actual: &SuspectedCause,
    comparison: &mut EvidenceCompilationComparison,
) {
    let artifact = format!("suspected_cause:{}", expected.entity);

    if (expected.rank == 1 && actual.rank != 1)
        || (expected.trap_note.is_some() && actual.rank == 1 && actual.entity != "under-determined")
    {
        comparison
            .suspected_cause_mismatches
            .push(EvidenceCompilationFieldMismatch {
                artifact: artifact.clone(),
                field: "rank".to_string(),
                expected: Value::from(expected.rank),
                actual: Some(Value::from(actual.rank)),
            });
    }

    if expected.trap_note.is_some() {
        compare_score_band(
            &mut comparison.suspected_cause_mismatches,
            &artifact,
            expected.score,
            actual.score,
        );
    }
    compare_reason_categories(
        &mut comparison.suspected_cause_mismatches,
        &artifact,
        "reasons",
        &expected.reasons,
        &actual.reasons,
    );
    if expected.rank == 1 {
        compare_link_presence(
            &mut comparison.suspected_cause_mismatches,
            &artifact,
            "supporting",
            &expected.supporting,
            &actual.supporting,
        );
    }
    if expected.trap_note.is_some() {
        compare_link_presence(
            &mut comparison.suspected_cause_mismatches,
            &artifact,
            "counter",
            &expected.counter,
            &actual.counter,
        );
    }
    compare_text_structural(
        &mut comparison.suspected_cause_mismatches,
        &mut comparison.text_differences,
        &artifact,
        "hypothesis",
        expected.hypothesis.as_str(),
        actual.hypothesis.as_str(),
    );
    compare_optional_text_structural(
        &mut comparison.suspected_cause_mismatches,
        &mut comparison.text_differences,
        &artifact,
        "note",
        expected.note.as_deref(),
        actual.note.as_deref(),
    );
    compare_optional_text_structural(
        &mut comparison.suspected_cause_mismatches,
        &mut comparison.text_differences,
        &artifact,
        "trap_note",
        expected.trap_note.as_deref(),
        actual.trap_note.as_deref(),
    );
}

fn compare_next_checks(
    expected: &EvidenceCompilation,
    actual: &EvidenceCompilation,
    comparison: &mut EvidenceCompilationComparison,
) {
    let mut used_actual = BTreeSet::<usize>::new();
    let expected_categories = expected
        .next_checks
        .iter()
        .map(|check| normalized_next_check_signal(&check.expected_signal))
        .collect::<BTreeSet<_>>();
    let actual_categories = actual
        .next_checks
        .iter()
        .map(|check| normalized_next_check_signal(&check.expected_signal))
        .collect::<BTreeSet<_>>();

    if !expected_categories.is_empty()
        && expected_categories
            .iter()
            .all(|category| !actual_categories.contains(category))
    {
        comparison.missing_next_checks.push(0);
    }

    for (expected_index, expected_check) in expected.next_checks.iter().enumerate() {
        let expected_category = normalized_next_check_signal(&expected_check.expected_signal);
        let Some(actual_index) =
            next_check_match_by_category(expected_category.as_str(), actual, &used_actual)
        else {
            continue;
        };

        used_actual.insert(actual_index);
        let actual_check = &actual.next_checks[actual_index];
        let artifact = format!("next_check:{}", expected_index + 1);
        compare_text_structural(
            &mut comparison.next_check_mismatches,
            &mut comparison.text_differences,
            &artifact,
            "action",
            expected_check.action.as_str(),
            actual_check.action.as_str(),
        );
        compare_text_structural(
            &mut comparison.next_check_mismatches,
            &mut comparison.text_differences,
            &artifact,
            "rationale",
            expected_check.rationale.as_str(),
            actual_check.rationale.as_str(),
        );
    }

    for (actual_index, actual_check) in actual.next_checks.iter().enumerate() {
        if used_actual.contains(&actual_index) {
            continue;
        }

        let category = normalized_next_check_signal(&actual_check.expected_signal);
        if actual.next_checks.len() > expected.next_checks.len()
            || !known_next_check_signals().contains(category.as_str())
        {
            comparison.extra_next_checks.push(actual_index);
        }
    }
}

fn next_check_match_by_category(
    expected_category: &str,
    actual: &EvidenceCompilation,
    used_actual: &BTreeSet<usize>,
) -> Option<usize> {
    actual
        .next_checks
        .iter()
        .enumerate()
        .find(|(index, actual_check)| {
            !used_actual.contains(index)
                && normalized_next_check_signal(&actual_check.expected_signal) == expected_category
        })
        .map(|(index, _)| index)
}

fn expected_counter_evidence(item: &EvidenceItem) -> bool {
    item.kind == EvidenceKind::CounterEvidence
        || matches!(
            item.direction,
            EvidenceDirection::Weakens | EvidenceDirection::Contradicts
        )
}

fn source_ref_identity_overlap(expected: &EvidenceItem, actual: &EvidenceItem) -> bool {
    expected.source_refs.iter().any(|expected_ref| {
        actual.source_refs.iter().any(|actual_ref| {
            expected_ref.signal == actual_ref.signal && expected_ref.r#ref == actual_ref.r#ref
        })
    })
}

fn source_signal_overlap(expected: &EvidenceItem, actual: &EvidenceItem) -> bool {
    let expected_signals = source_signal_set(expected);
    let actual_signals = source_signal_set(actual);

    expected_signals
        .iter()
        .any(|signal| actual_signals.contains(signal))
}

fn source_signal_set(item: &EvidenceItem) -> BTreeSet<String> {
    item.source_refs
        .iter()
        .map(|source_ref| format!("{:?}", source_ref.signal))
        .collect()
}

fn entity_structural_overlap(expected: &[String], actual: &[String]) -> bool {
    let expected_keys = expected
        .iter()
        .flat_map(|entity| entity_compare_keys(entity))
        .collect::<BTreeSet<_>>();

    actual
        .iter()
        .flat_map(|entity| entity_compare_keys(entity))
        .any(|entity| expected_keys.contains(&entity))
}

fn entity_compare_keys(entity: &str) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    let canonical = entity
        .strip_suffix("(aggregate)")
        .or_else(|| entity.strip_suffix("(aggregate-by-name)"))
        .unwrap_or(entity);

    keys.insert(canonical.to_string());

    if let Some((service, _variant)) = canonical.split_once('@') {
        keys.insert(service.to_string());
    }

    if let Some((service, _suffix)) = canonical.split_once('(') {
        keys.insert(service.to_string());
    }

    keys
}

fn is_material_extra_cause(cause: &SuspectedCause, expected_count: usize) -> bool {
    cause.rank == 1 && cause.entity != "under-determined" && cause.rank <= expected_count as u32
}

fn actual_top_is_under_determined(causes: &[SuspectedCause]) -> bool {
    causes
        .iter()
        .any(|cause| cause.rank == 1 && cause.entity == "under-determined")
}

fn compare_score_band(
    mismatches: &mut Vec<EvidenceCompilationFieldMismatch>,
    artifact: &str,
    expected: UnitInterval,
    actual: UnitInterval,
) {
    let expected_high = expected.0 >= 0.45;
    let expected_low = expected.0 < 0.20;
    let actual_high = actual.0 >= 0.45;
    let actual_low = actual.0 < 0.20;

    if (expected_high && actual_low) || (expected_low && actual_high) {
        mismatches.push(EvidenceCompilationFieldMismatch {
            artifact: artifact.to_string(),
            field: "score".to_string(),
            expected: Value::from(expected.0),
            actual: Some(Value::from(actual.0)),
        });
    }
}

fn compare_link_presence(
    mismatches: &mut Vec<EvidenceCompilationFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: &[String],
    actual: &[String],
) {
    if !expected.is_empty() && actual.is_empty() {
        mismatches.push(EvidenceCompilationFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: Value::String("one or more linked evidence ids".to_string()),
            actual: Some(Value::Array(Vec::new())),
        });
    }
}

fn compare_reason_categories(
    mismatches: &mut Vec<EvidenceCompilationFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: &[String],
    actual: &[String],
) {
    if expected.is_empty() {
        return;
    }

    if actual.is_empty() {
        mismatches.push(EvidenceCompilationFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: serde_json::to_value(string_set(expected)).unwrap_or(Value::Null),
            actual: Some(Value::Array(Vec::new())),
        });
        return;
    }

    let expected = string_set(expected);
    let known = known_reason_tokens();
    let unknown = actual
        .iter()
        .filter(|reason| !expected.contains(reason.as_str()) && !known.contains(reason.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    if !unknown.is_empty() {
        mismatches.push(EvidenceCompilationFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: serde_json::to_value(known).unwrap_or(Value::Null),
            actual: Some(serde_json::to_value(unknown).unwrap_or(Value::Null)),
        });
    }
}

fn known_reason_tokens() -> BTreeSet<&'static str> {
    [
        "catalog_latency_unchanged",
        "change_proximity",
        "config_change_proximity",
        "counter_evidence",
        "counter_metric",
        "counter_trace",
        "db_latency_flat",
        "db_spans_ok",
        "deadline_exceeded_signature",
        "dependency_direction",
        "deploy_origin",
        "error_rate_spike",
        "error_signature",
        "error_signature_42703",
        "errors_on_external_span_only",
        "exact_time_alignment",
        "hit_ratio_collapse",
        "hot_partition",
        "latency_flat",
        "latency_spike",
        "lock_metric",
        "log_cluster",
        "metric_anomaly",
        "oom_logs",
        "onset_precedes_change",
        "prior_incident_match",
        "provider_status_event",
        "restart_aligned_errors",
        "retry_fanout_trace",
        "retry_rate_tracks_load",
        "runtime_child_anomaly",
        "sawtooth_rss",
        "span_specificity",
        "telemetry_gap_across_peak",
        "tenant_ramp_time_aligned",
        "time_alignment",
        "trace_exemplar",
        "trace_shows_miss_fallback",
    ]
    .into_iter()
    .collect()
}

fn normalized_next_check_signal(signal: &str) -> String {
    match signal
        .trim()
        .to_ascii_lowercase()
        .replace('-', "_")
        .as_str()
    {
        "code_change" | "change" => "change_event".to_string(),
        "entity_resolution" => "entity_resolution".to_string(),
        other => other.to_string(),
    }
}

fn known_next_check_signals() -> BTreeSet<&'static str> {
    [
        "change_event",
        "compare_windows",
        "entity_resolution",
        "find_related_anomalies",
        "log_cluster",
        "metric_anomaly",
        "profile_hotspot",
        "relationship",
        "trace",
    ]
    .into_iter()
    .collect()
}

fn compare_exact<T>(
    mismatches: &mut Vec<EvidenceCompilationFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: &T,
    actual: &T,
) where
    T: Serialize + PartialEq,
{
    if expected != actual {
        mismatches.push(EvidenceCompilationFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: serde_json::to_value(expected).unwrap_or(Value::Null),
            actual: Some(serde_json::to_value(actual).unwrap_or(Value::Null)),
        });
    }
}

fn compare_text_structural(
    mismatches: &mut Vec<EvidenceCompilationFieldMismatch>,
    differences: &mut Vec<EvidenceCompilationFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: &str,
    actual: &str,
) {
    if expected.trim().is_empty() {
        return;
    }

    if actual.trim().is_empty() {
        mismatches.push(EvidenceCompilationFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: Value::String("non-empty text".to_string()),
            actual: Some(Value::String(actual.to_string())),
        });
    } else if expected != actual {
        differences.push(EvidenceCompilationFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: Value::String(expected.to_string()),
            actual: Some(Value::String(actual.to_string())),
        });
    }
}

fn compare_optional_text_structural(
    mismatches: &mut Vec<EvidenceCompilationFieldMismatch>,
    differences: &mut Vec<EvidenceCompilationFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: Option<&str>,
    actual: Option<&str>,
) {
    match (expected, actual) {
        (Some(expected), Some(actual)) => {
            compare_text_structural(mismatches, differences, artifact, field, expected, actual);
        }
        (Some(expected), None) if !expected.trim().is_empty() => {
            mismatches.push(EvidenceCompilationFieldMismatch {
                artifact: artifact.to_string(),
                field: field.to_string(),
                expected: Value::String("non-empty text".to_string()),
                actual: None,
            });
        }
        _ => {}
    }
}

fn string_set(values: &[String]) -> BTreeSet<String> {
    values.iter().cloned().collect()
}

impl fmt::Display for EvidenceCompilationGoldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvidenceCompilationGoldError::MissingArtifact {
                fixture_id,
                artifact,
            } => write!(
                formatter,
                "fixture `{fixture_id}` is missing expected artifact `{artifact}`"
            ),
            EvidenceCompilationGoldError::ParseArtifact {
                fixture_id,
                artifact,
                source,
            } => write!(
                formatter,
                "fixture `{fixture_id}` has invalid expected artifact `{artifact}`: {source}"
            ),
        }
    }
}

impl std::error::Error for EvidenceCompilationGoldError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EvidenceCompilationGoldError::ParseArtifact { source, .. } => Some(source),
            EvidenceCompilationGoldError::MissingArtifact { .. } => None,
        }
    }
}

impl fmt::Display for EvidenceCompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvidenceCompileError::TokenEstimate { item_id, source } => {
                write!(
                    formatter,
                    "failed to estimate token cost for {item_id}: {source}"
                )
            }
            EvidenceCompileError::TokenCostOverflow { item_id, bytes } => write!(
                formatter,
                "estimated token cost for {item_id} overflows u32 from {bytes} bytes"
            ),
            EvidenceCompileError::RequirementUnsatisfied {
                requirement,
                message,
            } => write!(
                formatter,
                "unsatisfied compiler requirement {requirement}: {message}"
            ),
        }
    }
}

impl std::error::Error for EvidenceCompileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EvidenceCompileError::TokenEstimate { source, .. } => Some(source),
            EvidenceCompileError::TokenCostOverflow { .. }
            | EvidenceCompileError::RequirementUnsatisfied { .. } => None,
        }
    }
}
