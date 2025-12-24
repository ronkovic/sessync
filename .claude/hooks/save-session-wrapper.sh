#!/bin/bash
# 現在のセッションログを手動保存するラッパースクリプト

set -euo pipefail

CWD="$(pwd)"
# プロジェクトパスをディレクトリ名に変換
PROJECT_NAME=$(echo "$CWD" | sed 's/\//-/g')
PROJECTS_DIR="$HOME/.claude/projects/$PROJECT_NAME"

# 最新のセッションログファイルを検索（agent-を除く）
LATEST_LOG=$(ls -t "$PROJECTS_DIR"/*.jsonl 2>/dev/null | grep -v agent- | head -1)

if [ -z "$LATEST_LOG" ]; then
  echo "Error: No session log found for this project" >&2
  exit 1
fi

# セッションIDをファイル名から抽出
SESSION_ID=$(basename "$LATEST_LOG" .jsonl)
TIMESTAMP=$(date "+%Y-%m-%d_%H-%M-%S")

# JSONを生成してsession-end.shに渡す
cat <<EOF | "$(dirname "$0")/session-end.sh"
{
  "session_id": "$SESSION_ID",
  "transcript_path": "$LATEST_LOG",
  "cwd": "$CWD",
  "reason": "manual"
}
EOF

echo "Session log saved manually at $TIMESTAMP"
