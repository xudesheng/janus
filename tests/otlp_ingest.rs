use janus::{
    evidence::{SourceRef, SourceSignal},
    hot_context_store::{SourceResolution, StoredRecord},
    otlp_ingest::{OtlpIngestResult, ingest_otlp_json_files, ingest_otlp_json_value},
};
use serde_json::{Value, json};
use std::{path::PathBuf, process::Command};

#[test]
fn sample_uses_true_otlp_json_envelopes() {
    let sample = sample_json();

    assert!(
        sample
            .pointer("/resourceSpans/0/scopeSpans/0/spans")
            .is_some()
    );
    assert!(
        sample
            .pointer("/resourceMetrics/0/scopeMetrics/0/metrics/0/gauge/dataPoints")
            .is_some()
    );
    assert!(
        sample
            .pointer("/resourceLogs/0/scopeLogs/0/logRecords")
            .is_some()
    );
}

#[test]
fn parses_otlp_trace_payload_with_resource_and_scope_spans() {
    let result = ingest_sample();

    assert!(!result.summary.has_errors(), "{:?}", result.summary.errors);
    assert_eq!(result.summary.accepted.traces, 1);
    assert_eq!(result.summary.accepted.spans, 2);

    let span = found(result.store.resolve_source_ref(&source_ref(
        SourceSignal::Trace,
        "4bf92f3577b34da6a3ce929d0e0e4736/00f067aa0ba902b7",
    )));

    assert_eq!(span.payload["trace_id"], "4bf92f3577b34da6a3ce929d0e0e4736");
    assert_eq!(span.payload["span_id"], "00f067aa0ba902b7");
    assert_eq!(span.payload["entity"], "service:checkout");
    assert_eq!(
        span.payload["_janus.provenance"]["envelope_path"],
        "$.resourceSpans[0].scopeSpans[0].spans[0]"
    );
}

#[test]
fn parses_metrics_into_accumulated_metric_series() {
    let result = ingest_sample();

    assert_eq!(result.summary.accepted.metric_points, 3);
    assert_eq!(result.summary.inserted_records, 9);
    assert_eq!(result.summary.updated_records, 1);
    assert_eq!(result.summary.duplicate_source_keys, 0);

    let metric = found(result.store.resolve_source_ref(&source_ref(
        SourceSignal::Metric,
        "http.server.error_rate@service:checkout",
    )));

    assert_eq!(metric.payload["name"], "http.server.error_rate");
    assert_eq!(metric.payload["entity"], "service:checkout");
    assert_eq!(metric.payload["points"].as_array().unwrap().len(), 2);
    assert!(
        metric.payload["points"][0]
            .get("_janus.provenance")
            .is_some()
    );
}

#[test]
fn generated_log_ids_are_deterministic_for_same_input() {
    let first = ingest_sample();
    let second = ingest_sample();
    let first_generated = generated_log_ref(&first);
    let second_generated = generated_log_ref(&second);

    assert_eq!(first.summary.explicit_log_ids, 1);
    assert_eq!(first.summary.generated_log_ids, 1);
    assert_eq!(first_generated, second_generated);
    assert!(matches!(
        first
            .store
            .resolve_source_ref(&source_ref(SourceSignal::Log, &first_generated)),
        SourceResolution::Found(_)
    ));
}

#[test]
fn resource_key_fallback_is_low_quality_but_resolvable() {
    let result = ingest_sample();
    let fallback_metric = result
        .summary
        .emitted_source_refs
        .iter()
        .find(|source_ref| source_ref.starts_with("process.cpu.utilization@resource:attrs:"))
        .expect("fallback metric source ref should exist");

    assert_eq!(result.summary.low_quality_entity_hints, 2);
    assert!(matches!(
        result
            .store
            .resolve_source_ref(&source_ref(SourceSignal::Metric, fallback_metric)),
        SourceResolution::Found(_)
    ));
}

#[test]
fn low_quality_entity_hint_count_tracks_stored_records_not_envelopes() {
    let input = json!({
        "resourceSpans": [
            {
                "resource": low_quality_resource(),
                "scopeSpans": [
                    {
                        "spans": [
                            {
                                "traceId": "4bf92f3577b34da6a3ce929d0e0e4736",
                                "spanId": "00f067aa0ba902b7",
                                "startTimeUnixNano": "1780312980000000000",
                                "endTimeUnixNano": "1780312980900000000"
                            }
                        ]
                    }
                ]
            }
        ],
        "resourceMetrics": [
            {
                "resource": low_quality_resource(),
                "scopeMetrics": [
                    {
                        "metrics": [
                            {
                                "name": "worker.queue.depth",
                                "gauge": {
                                    "dataPoints": [
                                        {
                                            "timeUnixNano": "1780312980000000000",
                                            "asInt": "3"
                                        }
                                    ]
                                }
                            }
                        ]
                    }
                ]
            }
        ],
        "resourceLogs": [
            {
                "resource": low_quality_resource(),
                "scopeLogs": [
                    {
                        "logRecords": [
                            {
                                "timeUnixNano": "1780312980000000000",
                                "traceId": "4bf92f3577b34da6a3ce929d0e0e4736",
                                "spanId": "00f067aa0ba902b7",
                                "body": {
                                    "stringValue": "worker queue depth rising"
                                }
                            }
                        ]
                    }
                ]
            }
        ]
    });

    let result = ingest_otlp_json_value("multi-signal-low-quality.otlp.json", &input);

    assert!(!result.summary.has_errors(), "{:?}", result.summary.errors);
    assert_eq!(result.summary.accepted.resources, 1);
    assert_eq!(result.summary.accepted.spans, 1);
    assert_eq!(result.summary.accepted.metric_points, 1);
    assert_eq!(result.summary.accepted.logs, 1);
    assert_eq!(result.summary.low_quality_entity_hints, 4);
    assert_eq!(result.summary.missing_entity_hints, 0);
}

#[test]
fn every_emitted_source_ref_resolves_after_ingest() {
    let result = ingest_sample();

    assert_eq!(
        result.summary.source_refs_resolved,
        result.summary.emitted_source_refs.len()
    );
}

#[test]
fn malformed_trace_ids_produce_structured_errors() {
    let bad = json!({
        "resourceSpans": [
            {
                "resource": {
                    "attributes": [
                        {
                            "key": "service.name",
                            "value": { "stringValue": "checkout" }
                        }
                    ]
                },
                "scopeSpans": [
                    {
                        "spans": [
                            {
                                "traceId": "not-hex",
                                "spanId": "00f067aa0ba902b7",
                                "startTimeUnixNano": "1780312980000000000",
                                "endTimeUnixNano": "1780312980900000000"
                            }
                        ]
                    }
                ]
            }
        ]
    });

    let result = ingest_otlp_json_value("bad.otlp.json", &bad);

    assert_eq!(result.summary.accepted.spans, 0);
    assert_eq!(result.summary.rejected.spans, 1);
    assert!(result.summary.has_errors());
    assert_eq!(
        result.summary.errors[0].path,
        "$.resourceSpans[0].scopeSpans[0].spans[0].traceId"
    );
    assert!(result.summary.errors[0].message.contains("32 hex"));
}

#[test]
fn ingest_otlp_cli_smoke_test_succeeds_for_sample() {
    let output = Command::new(env!("CARGO_BIN_EXE_ingest_otlp"))
        .args([
            "--input",
            sample_path().to_str().unwrap(),
            "--ref",
            "http.server.error_rate@service:checkout",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "ingest_otlp failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("otlp ingest summary"));
    assert!(stdout.contains("records updated: 1"));
    assert!(stdout.contains("generated log ids: 1"));
    assert!(stdout.contains("ref http.server.error_rate@service:checkout: found metric_series"));
}

fn ingest_sample() -> OtlpIngestResult {
    ingest_otlp_json_files(&[sample_path()]).unwrap()
}

fn sample_json() -> Value {
    let bytes = std::fs::read(sample_path()).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn sample_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/otel/deploy-bad-rollout.otlp.json")
}

fn low_quality_resource() -> Value {
    json!({
        "attributes": [
            {
                "key": "host.name",
                "value": { "stringValue": "worker-a" }
            }
        ]
    })
}

fn source_ref(signal: SourceSignal, raw_ref: &str) -> SourceRef {
    SourceRef {
        signal,
        r#ref: raw_ref.to_string(),
    }
}

fn found(resolution: SourceResolution<'_>) -> &StoredRecord {
    match resolution {
        SourceResolution::Found(record) => record,
        other => panic!("expected found record, got {other:?}"),
    }
}

fn generated_log_ref(result: &OtlpIngestResult) -> String {
    result
        .summary
        .emitted_source_refs
        .iter()
        .find(|source_ref| source_ref.starts_with("log:"))
        .unwrap()
        .clone()
}
