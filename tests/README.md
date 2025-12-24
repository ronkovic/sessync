# Integration Tests

This directory contains integration tests for sessync.

## Test Types

### Unit Tests (in `src/`)
Unit tests are located alongside the source code in each module. Run with:
```bash
cargo test
```

### Integration Tests (in `tests/`)
Integration tests test the system end-to-end with real external services.

## Running Integration Tests

### Prerequisites
1. A valid GCP service account key with BigQuery write permissions
2. A BigQuery dataset and table configured

### Environment Setup
```bash
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account-key.json
export SESSYNC_TEST_PROJECT=your-gcp-project-id
export SESSYNC_TEST_DATASET=your-dataset
export SESSYNC_TEST_TABLE=your-table
```

### Running E2E Tests
```bash
# Run integration tests (requires GCP credentials)
cargo test --test integration_test

# Run with verbose output
cargo test --test integration_test -- --nocapture
```

## Test Fixtures

### `fixtures/sample.jsonl`
Sample session log data for testing. Contains 3 log entries:
- User message
- Assistant response
- Follow-up user message

## Coverage

Run coverage with:
```bash
cargo llvm-cov --html
open target/llvm-cov/html/index.html
```

Current coverage targets:
- config.rs: 100%
- models.rs: 100%
- dedup.rs: 97%+
- parser.rs: 95%+
- uploader.rs: 95%+
- auth.rs: 57% (requires GCP for full coverage)
- main.rs: 42% (integration testing needed)
- **Overall: 87%+**
