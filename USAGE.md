# BigQuery Uploader - Usage Guide

## Prerequisites

1. **Rust toolchain** - Install from https://rustup.rs/
2. **GCP Service Account** - With BigQuery permissions
3. **Service Account Key** - JSON key file for authentication

## Setup

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Configure BigQuery

Copy the example config and customize it:

```bash
mkdir -p .claude/bigquery
cp examples/config.json.example .claude/bigquery/config.json
```

Edit `.claude/bigquery/config.json` with your settings:

```json
{
  "project_id": "your-gcp-project-id",
  "dataset": "claude_sessions",
  "table": "session_logs",
  "location": "US",
  "upload_batch_size": 500,
  "enable_auto_upload": true,
  "enable_deduplication": true,
  "developer_id": "your-developer-id",
  "user_email": "your.email@example.com",
  "project_name": "your-project-name",
  "service_account_key_path": "~/.claude/bigquery/service-account-key.json"
}
```

### 3. Add Service Account Key

Place your GCP service account JSON key at the path specified in config:

```bash
cp /path/to/your-service-account-key.json ~/.claude/bigquery/service-account-key.json
chmod 600 ~/.claude/bigquery/service-account-key.json
```

## Building

```bash
cargo build --release
```

The binary will be available at: `target/release/upload-to-bigquery`

## Usage

### Dry Run (Test without uploading)

```bash
cargo run -- --dry-run
```

### Manual Upload

```bash
cargo run
```

### With Custom Config Path

```bash
cargo run -- --config /path/to/config.json
```

### Automatic Mode (from hook)

```bash
cargo run -- --auto
```

## Features

- **Deduplication**: Tracks uploaded UUIDs to prevent duplicates
- **Batch Upload**: Configurable batch size for efficient uploads
- **Service Account Auth**: Secure authentication using GCP service accounts
- **Team Collaboration**: Adds developer_id, hostname, user_email metadata
- **Incremental**: Only uploads new records since last run
- **Dry Run**: Test mode to preview uploads without sending data

## Log File Location

The uploader scans for `.jsonl` files in:

```
~/.claude/session-logs/
```

## Upload State

Upload state (deduplication tracking) is stored at:

```
~/.upload_state.json
```

This file tracks which UUIDs have been uploaded to prevent duplicates.

## Troubleshooting

### "Failed to authenticate with service account"

- Verify the service account key path is correct
- Ensure the key file has proper permissions (600)
- Check that the service account has BigQuery Data Editor role

### "No log files to process"

- Verify Claude Code session logs are being created
- Check that logs exist at `~/.claude/session-logs/`

### "Failed to upload to BigQuery"

- Verify the dataset and table exist in BigQuery
- Check that the service account has insert permissions
- Ensure the table schema matches the SessionLogOutput structure

## Integration with Claude Code

To automatically upload after each session, add to your `.claude/hooks/session-end.sh`:

```bash
#!/bin/bash
/path/to/upload-to-bigquery --auto
```
