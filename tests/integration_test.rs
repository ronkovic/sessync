//! Integration tests for sessync
//!
//! These tests verify end-to-end functionality.
//! Some tests require GCP credentials to run.

use std::fs;
use std::path::PathBuf;

/// Get the path to test fixtures
fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

#[test]
fn test_fixture_file_exists() {
    let sample = fixtures_path().join("sample.jsonl");
    assert!(sample.exists(), "sample.jsonl fixture should exist");
}

#[test]
fn test_fixture_file_valid_jsonl() {
    let sample = fixtures_path().join("sample.jsonl");
    let content = fs::read_to_string(&sample).expect("Failed to read sample.jsonl");

    let mut valid_lines = 0;
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(line);
        assert!(parsed.is_ok(), "Each line should be valid JSON: {}", line);

        let json = parsed.unwrap();
        assert!(json.get("uuid").is_some(), "Each entry should have uuid");
        assert!(
            json.get("timestamp").is_some(),
            "Each entry should have timestamp"
        );
        assert!(
            json.get("sessionId").is_some(),
            "Each entry should have sessionId"
        );
        assert!(json.get("type").is_some(), "Each entry should have type");
        assert!(
            json.get("message").is_some(),
            "Each entry should have message"
        );

        valid_lines += 1;
    }

    assert_eq!(valid_lines, 3, "sample.jsonl should have 3 entries");
}

/// Integration test that requires GCP credentials
/// Run with: cargo test --test integration_test -- --ignored
#[test]
#[ignore]
fn test_bigquery_upload_e2e() {
    // This test requires:
    // - GOOGLE_APPLICATION_CREDENTIALS env var set
    // - SESSYNC_TEST_PROJECT, SESSYNC_TEST_DATASET, SESSYNC_TEST_TABLE env vars set

    let project = std::env::var("SESSYNC_TEST_PROJECT")
        .expect("SESSYNC_TEST_PROJECT env var required for E2E test");
    let dataset = std::env::var("SESSYNC_TEST_DATASET")
        .expect("SESSYNC_TEST_DATASET env var required for E2E test");
    let table = std::env::var("SESSYNC_TEST_TABLE")
        .expect("SESSYNC_TEST_TABLE env var required for E2E test");

    println!("E2E test configuration:");
    println!("  Project: {}", project);
    println!("  Dataset: {}", dataset);
    println!("  Table: {}", table);

    // TODO: Implement actual E2E test when ready
    // 1. Create temporary config
    // 2. Run sessync with --dry-run on sample.jsonl
    // 3. Verify output
}
