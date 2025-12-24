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
mkdir -p .claude/sessync
cp examples/config.json.example .claude/sessync/config.json
```

Edit `.claude/sessync/config.json` with your settings:

```json
{
  "project_id": "your-gcp-project-id",
  "dataset": "claude_sessions",
  "table": "session_logs_v2",
  "location": "US",
  "upload_batch_size": 500,
  "enable_auto_upload": true,
  "enable_deduplication": true,
  "developer_id": "your-developer-id",
  "user_email": "your.email@example.com",
  "project_name": "your-project-name",
  "service_account_key_path": "./.claude/sessync/service-account-key.json"
}
```

### 3. Add Service Account Key

Place your GCP service account JSON key in the project directory:

```bash
cp /path/to/your-service-account-key.json ./.claude/sessync/service-account-key.json
chmod 600 ./.claude/sessync/service-account-key.json
```

**Note**: Service account key is project-local for multi-team support (different projects can use different BigQuery destinations).

### 4. Build and Deploy

```bash
cargo build --release
cp ./target/release/sessync ./.claude/sessync/sessync
chmod +x ./.claude/sessync/sessync
```

## Usage

### Dry Run (Test without uploading)

```bash
./.claude/sessync/sessync --dry-run
```

### Manual Upload (Current Project Only)

```bash
./.claude/sessync/sessync
```

### Upload All Projects

```bash
./.claude/sessync/sessync --all-projects
```

### With Custom Config Path

```bash
./.claude/sessync/sessync --config /path/to/config.json
```

### From Claude Code

Use the `/save-session` command within Claude Code to upload the current session to BigQuery.

## Features

- **Project Isolation**: Each project has its own config, service account key, and upload state
- **Multi-Team Support**: Different projects can upload to different BigQuery destinations
- **Deduplication**: Tracks uploaded UUIDs per-project to prevent duplicates
- **Batch Upload**: Configurable batch size for efficient uploads
- **Service Account Auth**: Secure authentication using GCP service accounts
- **Team Collaboration**: Adds developer_id, hostname, user_email metadata
- **Incremental**: Only uploads new records since last run
- **Dry Run**: Test mode to preview uploads without sending data
- **Automatic Upload**: SessionEnd hook for automatic uploads

## Log File Location

Claude Code stores session logs in:

```
~/.claude/projects/{project-name}/
```

Where `{project-name}` is the working directory path with `/` replaced by `-`.

By default, the uploader scans the current project's log directory. Use `--all-projects` to scan all projects.

## Upload State

Upload state (deduplication tracking) is stored per-project at:

```
./.claude/sessync/upload-state.json
```

This file tracks which UUIDs have been uploaded to prevent duplicates. Each project has its own state file to support uploading to different BigQuery destinations.

## Project Structure

```
your-project/
└── .claude/
    └── sessync/
        ├── config.json              ← BigQuery settings (per-project)
        ├── service-account-key.json ← GCP credentials (per-project)
        ├── upload-state.json        ← Dedup state (auto-generated)
        └── sessync                  ← Binary
```

## Troubleshooting

### "Failed to authenticate with service account"

- Verify the service account key path is correct
- Ensure the key file has proper permissions (600)
- Check that the service account has BigQuery Data Editor role

### "No log files to process"

- Verify Claude Code session logs are being created
- Check that logs exist at `~/.claude/projects/{project-name}/`
- Ensure you're running from the correct project directory

### "Failed to upload to BigQuery"

- Verify the dataset and table exist in BigQuery
- Check that the service account has insert permissions
- Ensure the table schema matches the SessionLogOutput structure

## Integration with Claude Code

### Automatic Upload (SessionEnd Hook)

Add to `.claude/settings.json`:

```json
{
  "hooks": {
    "SessionEnd": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "./.claude/sessync/sessync --auto",
            "timeout": 60
          }
        ]
      }
    ]
  }
}
```

### Manual Upload (Custom Command)

Create `.claude/commands/save-session.md`:

```markdown
---
description: Upload current session logs to BigQuery
allowed-tools: Bash
---

!`./.claude/sessync/sessync`
```

Then use `/save-session` within Claude Code to upload mid-session.
