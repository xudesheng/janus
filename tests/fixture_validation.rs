use janus::fixture_validation::{
    FixtureCorpus, FixtureSelector, IssueSeverity, validate_fixture_corpus,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn current_corpus_validates_successfully() {
    let report = validate_fixture_corpus(repo_root());

    assert_eq!(report.error_count(), 0, "{report}");
    assert_eq!(report.warning_count(), 4, "{report}");
    assert_eq!(report.coverage.fixture_count, 12);
    assert_eq!(report.coverage.false_causality_trap_count, 2);
}

#[test]
fn selectors_return_stable_registry_order() {
    let corpus = FixtureCorpus::load(repo_root()).unwrap();
    let selected = corpus.select(&FixtureSelector {
        capability: Some("compare_windows".to_string()),
        ..FixtureSelector::default()
    });
    let ids: Vec<&str> = selected
        .iter()
        .map(|case| case.registry_entry.id.as_str())
        .collect();

    assert_eq!(
        ids,
        vec![
            "deploy-bad-rollout",
            "dependency-db-degradation",
            "coincidental-deploy-trap",
            "config-change-timeout",
            "traffic-shift-hotspot"
        ]
    );
}

#[test]
fn duplicate_registry_ids_fail() {
    let corpus = TempCorpus::new("duplicate_registry_ids");
    write_minimal_corpus(corpus.path(), &["evidence-ir", "get_evidence_bundle"]);
    write_registry(
        corpus.path(),
        r#"[
          {
            "id": "minimal",
            "path": "scenarios/minimal",
            "failure_class": "deploy",
            "difficulty": "baseline",
            "false_causality_trap": false,
            "capabilities": ["evidence-ir", "get_evidence_bundle"],
            "title": "Minimal fixture"
          },
          {
            "id": "minimal",
            "path": "scenarios/minimal-copy",
            "failure_class": "deploy",
            "difficulty": "baseline",
            "false_causality_trap": false,
            "capabilities": ["evidence-ir", "get_evidence_bundle"],
            "title": "Minimal fixture copy"
          }
        ]"#,
    );

    let report = validate_fixture_corpus(corpus.path());

    assert_has_error(&report, "duplicate fixture id");
}

#[test]
fn unknown_capability_fails() {
    let corpus = TempCorpus::new("unknown_capability");
    write_minimal_corpus(corpus.path(), &["evidence-ir", "unknown-capability"]);

    let report = validate_fixture_corpus(corpus.path());

    assert_has_error(&report, "unknown capability `unknown-capability`");
}

#[test]
fn manifest_inputs_mismatch_fails() {
    let corpus = TempCorpus::new("inputs_mismatch");
    write_minimal_corpus(corpus.path(), &["evidence-ir", "get_evidence_bundle"]);
    write_scenario(
        corpus.path(),
        r#"{
          "id": "minimal",
          "title": "Minimal fixture",
          "version": 1,
          "schema_version": "fixtures/v1",
          "failure_class": "deploy",
          "difficulty": "baseline",
          "false_causality_trap": false,
          "summary": "Minimal scenario.",
          "question": "What happened?",
          "time_window": { "start": "2026-06-01T00:00:00Z", "end": "2026-06-01T00:05:00Z" },
          "ground_truth": { "primary_cause_entity": "service:minimal", "not_the_cause": [] },
          "capabilities": ["evidence-ir", "get_evidence_bundle"],
          "inputs": ["logs", "metrics"],
          "expected": ["evidence_bundle"]
        }"#,
    );

    let report = validate_fixture_corpus(corpus.path());

    assert_has_error(&report, "$.inputs");
}

#[test]
fn dangling_evidence_source_ref_fails() {
    let corpus = TempCorpus::new("dangling_source_ref");
    write_minimal_corpus(corpus.path(), &["evidence-ir", "get_evidence_bundle"]);
    write_expected(
        corpus.path(),
        minimal_expected_with_ref("log-missing").as_str(),
    );

    let report = validate_fixture_corpus(corpus.path());

    assert_has_error(&report, "unresolved source ref `log-missing`");
}

#[test]
fn dangling_timeline_source_ref_fails() {
    let corpus = TempCorpus::new("dangling_timeline_ref");
    write_minimal_corpus(
        corpus.path(),
        &["evidence-ir", "get_evidence_bundle", "build_timeline"],
    );
    write_scenario(
        corpus.path(),
        r#"{
          "id": "minimal",
          "title": "Minimal fixture",
          "version": 1,
          "schema_version": "fixtures/v1",
          "failure_class": "deploy",
          "difficulty": "baseline",
          "false_causality_trap": false,
          "summary": "Minimal scenario.",
          "question": "What happened?",
          "time_window": { "start": "2026-06-01T00:00:00Z", "end": "2026-06-01T00:05:00Z" },
          "ground_truth": { "primary_cause_entity": "service:minimal", "not_the_cause": [] },
          "capabilities": ["evidence-ir", "get_evidence_bundle", "build_timeline"],
          "inputs": ["logs"],
          "expected": ["evidence_bundle", "timeline"]
        }"#,
    );
    write_expected(
        corpus.path(),
        format!(
            r#"{{
              "evidence_bundle": {},
              "timeline": [
                {{ "t": "2026-06-01T00:01:00Z", "marker": "symptom", "entity": "service:minimal", "text": "missing", "source_ref": "log-missing" }}
              ]
            }}"#,
            minimal_bundle_with_ref("log-1")
        )
        .as_str(),
    );

    let report = validate_fixture_corpus(corpus.path());

    assert_has_error(&report, "$.timeline[0].source_ref");
}

#[test]
fn missing_false_causality_counter_evidence_fails() {
    let corpus = TempCorpus::new("missing_false_causality_counter");
    write_minimal_corpus(corpus.path(), &["evidence-ir", "get_evidence_bundle"]);
    write_scenario(
        corpus.path(),
        r#"{
          "id": "minimal",
          "title": "Minimal fixture",
          "version": 1,
          "schema_version": "fixtures/v1",
          "failure_class": "deploy",
          "difficulty": "baseline",
          "false_causality_trap": true,
          "summary": "Minimal scenario.",
          "question": "What happened?",
          "time_window": { "start": "2026-06-01T00:00:00Z", "end": "2026-06-01T00:05:00Z" },
          "ground_truth": { "primary_cause_entity": "service:minimal", "not_the_cause": ["service:innocent"] },
          "capabilities": ["evidence-ir", "get_evidence_bundle"],
          "inputs": ["logs"],
          "expected": ["evidence_bundle"]
        }"#,
    );

    let report = validate_fixture_corpus(corpus.path());

    assert_has_error(&report, "false-causality trap requires counter-evidence");
}

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn assert_has_error(report: &janus::fixture_validation::FixtureValidationReport, needle: &str) {
    assert!(
        report.issues.iter().any(|issue| {
            issue.severity == IssueSeverity::Error
                && (issue.message.contains(needle) || issue.json_path.contains(needle))
        }),
        "expected error containing {needle:?}, got:\n{report}"
    );
}

struct TempCorpus {
    path: PathBuf,
}

impl TempCorpus {
    fn new(test_name: &str) -> Self {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("janus_{test_name}_{suffix}"));
        fs::create_dir_all(path.join("fixtures/scenarios/minimal")).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempCorpus {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn write_minimal_corpus(root: &Path, capabilities: &[&str]) {
    write_registry(root, registry_fixture_entry(capabilities).as_str());
    write_scenario(root, minimal_scenario(capabilities).as_str());
    write_input(root, minimal_input());
    write_expected(root, minimal_expected_with_ref("log-1").as_str());
}

fn write_registry(root: &Path, fixtures_array: &str) {
    write_file(
        root,
        "fixtures/registry.json",
        format!(
            r#"{{
              "schema_version": "fixtures/v1",
              "description": "test registry",
              "capabilities": [
                "evidence-ir",
                "get_evidence_bundle",
                "build_timeline",
                "rank_suspected_causes",
                "false-causality-guard"
              ],
              "failure_classes": ["deploy", "missing-data"],
              "fixtures": {fixtures_array},
              "proposed": []
            }}"#
        )
        .as_str(),
    );
}

fn registry_fixture_entry(capabilities: &[&str]) -> String {
    format!(
        r#"[{{
          "id": "minimal",
          "path": "scenarios/minimal",
          "failure_class": "deploy",
          "difficulty": "baseline",
          "false_causality_trap": false,
          "capabilities": {},
          "title": "Minimal fixture"
        }}]"#,
        json_string_array(capabilities)
    )
}

fn minimal_scenario(capabilities: &[&str]) -> String {
    format!(
        r#"{{
          "id": "minimal",
          "title": "Minimal fixture",
          "version": 1,
          "schema_version": "fixtures/v1",
          "failure_class": "deploy",
          "difficulty": "baseline",
          "false_causality_trap": false,
          "summary": "Minimal scenario.",
          "question": "What happened?",
          "time_window": {{ "start": "2026-06-01T00:00:00Z", "end": "2026-06-01T00:05:00Z" }},
          "ground_truth": {{ "primary_cause_entity": "service:minimal", "not_the_cause": [] }},
          "capabilities": {},
          "inputs": ["logs"],
          "expected": ["evidence_bundle"]
        }}"#,
        json_string_array(capabilities)
    )
}

fn minimal_input() -> &'static str {
    r#"{
      "logs": [
        { "id": "log-1", "t": "2026-06-01T00:01:00Z", "entity": "service:minimal", "severity": "ERROR", "body": "failed", "attributes": {} }
      ]
    }"#
}

fn minimal_expected_with_ref(source_ref: &str) -> String {
    format!(
        r#"{{ "evidence_bundle": {} }}"#,
        minimal_bundle_with_ref(source_ref)
    )
}

fn minimal_bundle_with_ref(source_ref: &str) -> String {
    format!(
        r#"{{
          "question": "What happened?",
          "time_window": {{ "start": "2026-06-01T00:00:00Z", "end": "2026-06-01T00:05:00Z" }},
          "budget": {{ "max_items": 1, "max_tokens": 100, "tokens_used": 20, "items_dropped": 0 }},
          "items": [
            {{
              "id": "ev-1",
              "claim": "A minimal log exists.",
              "kind": "log_cluster",
              "direction": "supports",
              "strength": 0.8,
              "time_window": {{ "start": "2026-06-01T00:01:00Z", "end": "2026-06-01T00:01:00Z" }},
              "entities": ["service:minimal"],
              "source_refs": [{{ "signal": "log", "ref": "{source_ref}" }}],
              "freshness": "settled",
              "missing_data": [],
              "token_cost": 20,
              "privacy_scope": "none"
            }}
          ]
        }}"#
    )
}

fn write_scenario(root: &Path, body: &str) {
    write_file(root, "fixtures/scenarios/minimal/scenario.json", body);
}

fn write_input(root: &Path, body: &str) {
    write_file(root, "fixtures/scenarios/minimal/input.json", body);
}

fn write_expected(root: &Path, body: &str) {
    write_file(root, "fixtures/scenarios/minimal/expected.json", body);
}

fn write_file(root: &Path, relative: &str, body: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

fn json_string_array(values: &[&str]) -> String {
    serde_json::to_string(values).unwrap()
}
