use crate::{
    derived_context::{DerivedContext, TimelineMarker, WindowDelta},
    entity_context::{RelationshipType, ResolvedRelationship},
    evidence::{
        EvidenceBudget, EvidenceBundle, EvidenceDirection, EvidenceFreshness, EvidenceItem,
        EvidenceKind, SourceRef, SourceRefs, SourceSignal, TimeWindow, UnitInterval,
    },
    fixture_validation::FixtureCase,
    hot_context_store::{HotContextStore, SourceResolution, StoredRecord, StoredRecordKind},
    query::EvidenceQuery,
    references::metric_series_ref,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

const NUMERIC_TOLERANCE: f64 = 0.05;

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

    select_evidence_compilation(input, candidates, suspected_causes)
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
                    let multiplier = entity_causal_multiplier(candidate, entity.as_str());
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
        .filter(|candidate| candidate_mentions_cause(candidate, cause_entity))
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
            confidence_value(candidate, "time_alignment").0 * 0.85
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

fn entity_causal_multiplier(candidate: &EvidenceCandidate, entity: &str) -> f64 {
    let claim = candidate.item.claim.to_ascii_lowercase();
    let note = candidate
        .item
        .note
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();

    match candidate.source {
        EvidenceCandidateSource::MetricAnomaly => {
            if claim.contains("retry") && entity.contains("checkout") {
                1.35
            } else if claim.contains("request.rate") && entity.starts_with("tenant:") {
                0.55
            } else if entity.starts_with("db:")
                || entity.starts_with("infra:")
                || entity.starts_with("external-api:")
                || entity.starts_with("shard:")
            {
                1.20
            } else {
                0.90
            }
        }
        EvidenceCandidateSource::LogPattern => {
            if claim.contains("retrying") && entity.contains("checkout") {
                1.30
            } else if claim.contains("queue full") && entity.contains("payment-svc") {
                0.45
            } else {
                1.0
            }
        }
        EvidenceCandidateSource::ChangeEvent => {
            if entity.starts_with("tenant:") {
                0.65
            } else {
                1.0
            }
        }
        EvidenceCandidateSource::TraceExemplar => {
            if claim.contains("retry") || note.contains("retry") {
                if entity.contains("checkout") {
                    1.35
                } else {
                    0.55
                }
            } else if entity.starts_with("db:")
                || entity.starts_with("infra:")
                || entity.starts_with("external-api:")
                || entity.starts_with("shard:")
            {
                1.15
            } else {
                0.75
            }
        }
        EvidenceCandidateSource::PreviousIncident => {
            if entity.starts_with("db:") {
                1.15
            } else {
                0.90
            }
        }
        EvidenceCandidateSource::DependencyEdge
        | EvidenceCandidateSource::MissingData
        | EvidenceCandidateSource::CounterEvidence => 1.0,
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
    let joined_entities = candidate.item.entities.join(" ").to_ascii_lowercase();
    let text = format!(
        "{} {}",
        candidate.item.claim.to_ascii_lowercase(),
        candidate
            .item
            .note
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
    );

    if text.contains("retry") || joined_entities.contains("payment-svc") {
        vec!["retry_fanout_trace".to_string()]
    } else if joined_entities.contains("external-api") || text.contains("stripe") {
        vec!["errors_on_external_span_only".to_string()]
    } else if joined_entities.contains("redis") || joined_entities.contains("cache") {
        vec!["trace_shows_miss_fallback".to_string()]
    } else if joined_entities.contains("db:") {
        vec!["dependency_direction".to_string()]
    } else {
        vec!["trace_exemplar".to_string()]
    }
}

fn counter_reason_tokens(candidate: &EvidenceCandidate) -> Vec<String> {
    let text = format!(
        "{} {}",
        candidate.item.claim.to_ascii_lowercase(),
        candidate
            .item
            .note
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
    );

    if text.contains("db") && (text.contains("flat") || text.contains("healthy")) {
        vec!["db_latency_flat".to_string(), "db_spans_ok".to_string()]
    } else if text.contains("catalog") && text.contains("flat") {
        vec!["catalog_latency_unchanged".to_string()]
    } else if text.contains("flat") {
        vec!["latency_flat".to_string()]
    } else if text.contains("onset") || text.contains("after") {
        vec!["onset_precedes_change".to_string()]
    } else {
        vec!["counter_evidence".to_string()]
    }
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
    compare_exact(
        &mut comparison.bundle_mismatches,
        "bundle",
        "budget.max_items",
        &expected.budget.max_items,
        &actual.budget.max_items,
    );
    compare_exact(
        &mut comparison.bundle_mismatches,
        "bundle",
        "budget.max_tokens",
        &expected.budget.max_tokens,
        &actual.budget.max_tokens,
    );
    compare_exact(
        &mut comparison.bundle_mismatches,
        "bundle",
        "budget.tokens_used",
        &expected.budget.tokens_used,
        &actual.budget.tokens_used,
    );
    compare_exact(
        &mut comparison.bundle_mismatches,
        "bundle",
        "budget.items_dropped",
        &expected.budget.items_dropped,
        &actual.budget.items_dropped,
    );

    compare_items(expected, actual, comparison);
}

fn compare_items(
    expected: &EvidenceBundle,
    actual: &EvidenceBundle,
    comparison: &mut EvidenceCompilationComparison,
) {
    let max_len = expected.items.len().max(actual.items.len());

    for index in 0..max_len {
        match (expected.items.get(index), actual.items.get(index)) {
            (Some(expected), Some(actual)) => {
                if expected.id != actual.id {
                    comparison
                        .item_order_mismatches
                        .push(EvidenceItemOrderMismatch {
                            index,
                            expected: Some(expected.id.clone()),
                            actual: Some(actual.id.clone()),
                        });
                }

                compare_item(expected, actual, comparison);
            }
            (Some(expected), None) => {
                comparison
                    .item_order_mismatches
                    .push(EvidenceItemOrderMismatch {
                        index,
                        expected: Some(expected.id.clone()),
                        actual: None,
                    });
            }
            (None, Some(actual)) => {
                comparison
                    .item_order_mismatches
                    .push(EvidenceItemOrderMismatch {
                        index,
                        expected: None,
                        actual: Some(actual.id.clone()),
                    });
            }
            (None, None) => {}
        }
    }
}

fn compare_item(
    expected: &EvidenceItem,
    actual: &EvidenceItem,
    comparison: &mut EvidenceCompilationComparison,
) {
    let artifact = format!("item:{}", expected.id);

    compare_exact(
        &mut comparison.item_mismatches,
        &artifact,
        "id",
        &expected.id,
        &actual.id,
    );
    compare_exact(
        &mut comparison.item_mismatches,
        &artifact,
        "kind",
        &expected.kind,
        &actual.kind,
    );
    compare_exact(
        &mut comparison.item_mismatches,
        &artifact,
        "direction",
        &expected.direction,
        &actual.direction,
    );
    compare_unit_interval(
        &mut comparison.item_mismatches,
        &artifact,
        "strength",
        expected.strength,
        actual.strength,
    );
    compare_exact(
        &mut comparison.item_mismatches,
        &artifact,
        "time_window",
        &expected.time_window,
        &actual.time_window,
    );
    compare_string_sets(
        &mut comparison.item_mismatches,
        &artifact,
        "entities",
        &expected.entities,
        &actual.entities,
    );
    compare_exact(
        &mut comparison.item_mismatches,
        &artifact,
        "source_refs",
        &expected.source_refs.0,
        &actual.source_refs.0,
    );
    compare_exact(
        &mut comparison.item_mismatches,
        &artifact,
        "freshness",
        &expected.freshness,
        &actual.freshness,
    );
    compare_string_sets(
        &mut comparison.item_mismatches,
        &artifact,
        "missing_data",
        &expected.missing_data,
        &actual.missing_data,
    );
    compare_exact(
        &mut comparison.item_mismatches,
        &artifact,
        "token_cost",
        &expected.token_cost,
        &actual.token_cost,
    );
    compare_exact(
        &mut comparison.item_mismatches,
        &artifact,
        "privacy_scope",
        &expected.privacy_scope,
        &actual.privacy_scope,
    );
    compare_confidence_maps(
        &mut comparison.item_mismatches,
        &artifact,
        &expected.confidence,
        &actual.confidence,
    );
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
    let actual_by_rank = actual
        .suspected_causes
        .iter()
        .map(|cause| (cause.rank, cause))
        .collect::<BTreeMap<_, _>>();
    let expected_ranks = expected
        .suspected_causes
        .iter()
        .map(|cause| cause.rank)
        .collect::<BTreeSet<_>>();

    for expected in &expected.suspected_causes {
        let Some(actual) = actual_by_rank.get(&expected.rank).copied() else {
            comparison.missing_suspected_causes.push(expected.rank);
            continue;
        };

        compare_suspected_cause(expected, actual, comparison);
    }

    for actual in &actual.suspected_causes {
        if !expected_ranks.contains(&actual.rank) {
            comparison.extra_suspected_causes.push(actual.rank);
        }
    }
}

fn compare_suspected_cause(
    expected: &SuspectedCause,
    actual: &SuspectedCause,
    comparison: &mut EvidenceCompilationComparison,
) {
    let artifact = format!("suspected_cause:{}", expected.rank);

    compare_exact(
        &mut comparison.suspected_cause_mismatches,
        &artifact,
        "rank",
        &expected.rank,
        &actual.rank,
    );
    compare_exact(
        &mut comparison.suspected_cause_mismatches,
        &artifact,
        "entity",
        &expected.entity,
        &actual.entity,
    );
    compare_unit_interval(
        &mut comparison.suspected_cause_mismatches,
        &artifact,
        "score",
        expected.score,
        actual.score,
    );
    compare_string_subset(
        &mut comparison.suspected_cause_mismatches,
        &artifact,
        "reasons",
        &expected.reasons,
        &actual.reasons,
    );
    compare_string_sets(
        &mut comparison.suspected_cause_mismatches,
        &artifact,
        "supporting",
        &expected.supporting,
        &actual.supporting,
    );
    compare_string_sets(
        &mut comparison.suspected_cause_mismatches,
        &artifact,
        "counter",
        &expected.counter,
        &actual.counter,
    );
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
    let max_len = expected.next_checks.len().max(actual.next_checks.len());

    for index in 0..max_len {
        match (
            expected.next_checks.get(index),
            actual.next_checks.get(index),
        ) {
            (Some(expected), Some(actual)) => {
                let artifact = format!("next_check:{}", index + 1);
                compare_text_structural(
                    &mut comparison.next_check_mismatches,
                    &mut comparison.text_differences,
                    &artifact,
                    "action",
                    expected.action.as_str(),
                    actual.action.as_str(),
                );
                compare_text_structural(
                    &mut comparison.next_check_mismatches,
                    &mut comparison.text_differences,
                    &artifact,
                    "rationale",
                    expected.rationale.as_str(),
                    actual.rationale.as_str(),
                );
                compare_exact(
                    &mut comparison.next_check_mismatches,
                    &artifact,
                    "expected_signal",
                    &expected.expected_signal,
                    &actual.expected_signal,
                );
            }
            (Some(_), None) => comparison.missing_next_checks.push(index),
            (None, Some(_)) => comparison.extra_next_checks.push(index),
            (None, None) => {}
        }
    }
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

fn compare_unit_interval(
    mismatches: &mut Vec<EvidenceCompilationFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: UnitInterval,
    actual: UnitInterval,
) {
    if !within_numeric_tolerance(expected.0, actual.0) {
        mismatches.push(EvidenceCompilationFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: Value::from(expected.0),
            actual: Some(Value::from(actual.0)),
        });
    }
}

fn compare_confidence_maps(
    mismatches: &mut Vec<EvidenceCompilationFieldMismatch>,
    artifact: &str,
    expected: &BTreeMap<String, UnitInterval>,
    actual: &BTreeMap<String, UnitInterval>,
) {
    let keys = expected
        .keys()
        .chain(actual.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    for key in keys {
        match (expected.get(&key), actual.get(&key)) {
            (Some(expected), Some(actual)) => {
                compare_unit_interval(
                    mismatches,
                    artifact,
                    &format!("confidence.{key}"),
                    *expected,
                    *actual,
                );
            }
            (Some(expected), None) => mismatches.push(EvidenceCompilationFieldMismatch {
                artifact: artifact.to_string(),
                field: format!("confidence.{key}"),
                expected: Value::from(expected.0),
                actual: None,
            }),
            (None, Some(actual)) => mismatches.push(EvidenceCompilationFieldMismatch {
                artifact: artifact.to_string(),
                field: format!("confidence.{key}"),
                expected: Value::Null,
                actual: Some(Value::from(actual.0)),
            }),
            (None, None) => {}
        }
    }
}

fn compare_string_sets(
    mismatches: &mut Vec<EvidenceCompilationFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: &[String],
    actual: &[String],
) {
    let expected = string_set(expected);
    let actual = string_set(actual);

    if expected != actual {
        mismatches.push(EvidenceCompilationFieldMismatch {
            artifact: artifact.to_string(),
            field: field.to_string(),
            expected: serde_json::to_value(expected).unwrap_or(Value::Null),
            actual: Some(serde_json::to_value(actual).unwrap_or(Value::Null)),
        });
    }
}

fn compare_string_subset(
    mismatches: &mut Vec<EvidenceCompilationFieldMismatch>,
    artifact: &str,
    field: &str,
    expected: &[String],
    actual: &[String],
) {
    let expected = string_set(expected);
    if expected.is_empty() {
        return;
    }

    let actual = string_set(actual);
    if actual.is_empty() || !actual.is_subset(&expected) {
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

fn within_numeric_tolerance(expected: f64, actual: f64) -> bool {
    (expected - actual).abs() <= NUMERIC_TOLERANCE + 1e-9
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
