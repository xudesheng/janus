use janus::{
    fixtures::load_bundle_by_scenario_id,
    query::{
        EvidenceQuery, EvidenceQueryBudget, EvidenceQueryIntent, FreshnessPreference,
        get_evidence_bundle,
    },
};
use std::{env, io, process};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let scenario_id = env::args().nth(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: emit_bundle <scenario-id>",
        )
    })?;

    let seed_bundle = load_bundle_by_scenario_id(&scenario_id)?;
    let query = EvidenceQuery {
        intent: EvidenceQueryIntent {
            question: seed_bundle.question.clone(),
            hypothesis: seed_bundle.hypothesis.clone(),
        },
        time_window: seed_bundle.time_window,
        budget: EvidenceQueryBudget {
            max_items: seed_bundle.items.len() as u32,
            max_tokens: seed_bundle.budget.tokens_used,
            min_counter_evidence_items: None,
            reserve_tokens_for_raw_refs: None,
        },
        scenario_id: Some(scenario_id),
        entities: Vec::new(),
        require_counter_evidence: false,
        require_raw_refs: true,
        freshness: FreshnessPreference::Any,
        privacy_scope: None,
    };

    let bundle = get_evidence_bundle(query)?;
    println!("{}", serde_json::to_string_pretty(&bundle)?);

    Ok(())
}
