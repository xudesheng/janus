use janus::{
    fixture_simulator::{
        SimulatedSignal, format_dry_run_plan, format_jsonl_plan, plan_fixture_replay,
    },
    fixture_validation::{FixtureCase, FixtureCorpus, FixtureSelector},
    hot_context_store::StoredRecordKind,
};
use serde_json::Value;
use std::{path::Path, process::Command};

#[test]
fn every_registered_fixture_produces_a_non_empty_replay_plan() {
    let corpus = FixtureCorpus::load(repo_root()).unwrap();

    for case in &corpus.cases {
        let plan = plan_fixture_replay(case).unwrap_or_else(|error| {
            panic!(
                "failed to plan replay for {}: {error}",
                case.registry_entry.id
            )
        });

        assert!(
            !plan.events().is_empty(),
            "fixture {} should produce replay events",
            case.registry_entry.id
        );

        for (index, event) in plan.events().iter().enumerate() {
            assert_eq!(event.sequence, index as u64);
            assert_eq!(event.scenario_id, case.registry_entry.id);
        }
    }
}

#[test]
fn replay_plan_is_deterministic_across_runs() {
    let case = fixture_case("deploy-bad-rollout");
    let first = plan_fixture_replay(&case).unwrap();
    let second = plan_fixture_replay(&case).unwrap();

    assert_eq!(event_signature(&first), event_signature(&second));
    assert_eq!(format_dry_run_plan(&first), format_dry_run_plan(&second));
}

#[test]
fn resources_are_emitted_before_timed_records() {
    let case = fixture_case("deploy-bad-rollout");
    let plan = plan_fixture_replay(&case).unwrap();
    let first_timed = plan
        .events()
        .iter()
        .position(|event| event.simulated_time.is_some())
        .expect("fixture should have timed records");

    for event in &plan.events()[..first_timed] {
        assert_eq!(event.simulated_time, None);
    }

    let resource_count = case.input["resources"].as_array().unwrap().len();
    let leading_resources = plan
        .events()
        .iter()
        .take(resource_count)
        .filter(|event| event.signal == SimulatedSignal::Resource)
        .count();

    assert_eq!(leading_resources, resource_count);
}

#[test]
fn metrics_are_replayed_as_ordered_metric_point_events() {
    let case = fixture_case("deploy-bad-rollout");
    let plan = plan_fixture_replay(&case).unwrap();
    let metric = metric_by_key(&case.input, "http.server.error_rate", "service:checkout");
    let points = metric["points"].as_array().unwrap();
    let events: Vec<_> = plan
        .events()
        .iter()
        .filter(|event| event.source_key == "http.server.error_rate@service:checkout")
        .collect();

    assert_eq!(events.len(), points.len());

    for (event, point) in events.iter().zip(points) {
        assert_eq!(event.signal, SimulatedSignal::MetricPoint);
        assert_eq!(event.record_kind, StoredRecordKind::MetricSeries);
        assert_eq!(event.payload["name"], "http.server.error_rate");
        assert_eq!(event.payload["entity"], "service:checkout");
        assert_eq!(event.payload["point"], *point);
    }
}

#[test]
fn trace_event_precedes_span_event_at_the_same_timestamp() {
    let case = fixture_case("deploy-bad-rollout");
    let plan = plan_fixture_replay(&case).unwrap();
    let trace_index = plan
        .events()
        .iter()
        .position(|event| event.source_key == "t-0001")
        .unwrap();
    let span_index = plan
        .events()
        .iter()
        .position(|event| event.source_key == "t-0001/s-1")
        .unwrap();

    assert!(trace_index < span_index);
    assert_eq!(
        plan.events()[trace_index].simulated_time,
        plan.events()[span_index].simulated_time
    );
}

#[test]
fn dry_run_output_renders_event_order_without_store_mutation() {
    let case = fixture_case("deploy-bad-rollout");
    let plan = plan_fixture_replay(&case).unwrap();
    let output = format_dry_run_plan(&plan);

    assert!(output.starts_with("fixture deploy-bad-rollout replay plan: "));
    assert!(output.contains("0000 preload resource resource res:api-gateway"));
    assert!(output.contains("trace trace t-0001"));
    assert!(output.contains("metric_point metric_series http.server.error_rate@service:checkout"));
}

#[test]
fn jsonl_output_renders_one_event_per_line() {
    let case = fixture_case("deploy-bad-rollout");
    let plan = plan_fixture_replay(&case).unwrap();
    let output = format_jsonl_plan(&plan).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    let first: Value = serde_json::from_str(lines[0]).unwrap();

    assert_eq!(lines.len(), plan.events().len());
    assert_eq!(first["sequence"], 0);
    assert_eq!(first["signal"], "resource");
    assert_eq!(first["record_kind"], "resource");
    assert_eq!(first["source_key"], "res:api-gateway");
}

#[test]
fn simulate_fixture_cli_dry_run_succeeds_for_one_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_simulate_fixture"))
        .args(["--fixture", "deploy-bad-rollout", "--dry-run"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "simulate_fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("fixture deploy-bad-rollout replay plan: "));
    assert!(stdout.contains("metric_point metric_series http.server.error_rate@service:checkout"));
}

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn fixture_case(id: &str) -> FixtureCase {
    let corpus = FixtureCorpus::load(repo_root()).unwrap();
    let selected = corpus.select(&FixtureSelector {
        fixture_id: Some(id.to_string()),
        ..FixtureSelector::default()
    });

    selected.into_iter().next().unwrap().clone()
}

fn metric_by_key<'a>(input: &'a Value, name: &str, entity: &str) -> &'a Value {
    input["metrics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|metric| metric["name"] == name && metric["entity"] == entity)
        .unwrap()
}

fn event_signature(plan: &janus::fixture_simulator::FixtureReplayPlan) -> Vec<String> {
    plan.events()
        .iter()
        .map(|event| {
            format!(
                "{}|{:?}|{}|{}|{}",
                event.sequence,
                event.simulated_time,
                event.signal,
                event.record_kind,
                event.source_key
            )
        })
        .collect()
}
