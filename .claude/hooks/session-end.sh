#!/bin/bash

# SessionEnd フック: セッションログを保存・整理する
# 標準入力からJSONを受け取り、セッションログを .claude/session-logs/ に保存

set -euo pipefail

# プロジェクトルートを取得（スクリプトの2階層上）
PROJECT_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
LOG_DIR="${PROJECT_ROOT}/.claude/session-logs"
INDEX_FILE="${LOG_DIR}/index.txt"

# 標準入力からJSONを読み込む
INPUT=$(cat)

# jqでパース
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // "unknown"')
TRANSCRIPT_PATH=$(echo "$INPUT" | jq -r '.transcript_path // ""')
REASON=$(echo "$INPUT" | jq -r '.reason // "unknown"')
CWD=$(echo "$INPUT" | jq -r '.cwd // ""')

# transcript_pathのチルダ展開
TRANSCRIPT_PATH="${TRANSCRIPT_PATH/#\~/$HOME}"

# タイムスタンプ取得
TIMESTAMP=$(date "+%Y-%m-%d_%H-%M-%S")

# ログファイルが存在するか確認
if [ ! -f "$TRANSCRIPT_PATH" ]; then
  echo "Warning: Transcript file not found: $TRANSCRIPT_PATH" >&2
  exit 0
fi

# プロジェクトパスをディレクトリ名に変換
PROJECT_NAME=$(echo "$CWD" | sed 's/\//-/g')

# 新しいファイル名形式
DEST_FILE="${LOG_DIR}/${PROJECT_NAME}_${SESSION_ID}.jsonl"

# 差分追記処理
if [ -f "$DEST_FILE" ]; then
  # 既存ファイルから全てのuuid/messageIdを抽出してJSON配列に変換
  EXISTING_IDS_JSON=$(jq -r 'if .uuid then .uuid elif .messageId then .messageId else empty end' "$DEST_FILE" | jq -R -s 'split("\n") | map(select(length > 0))')

  # 新しいログから既存IDにないエントリだけをフィルタリングして一時ファイルに保存
  TEMP_NEW=$(mktemp)
  jq --argjson existing_ids "$EXISTING_IDS_JSON" \
    'select(
      ((.uuid // .messageId) as $id | ($existing_ids | index($id)) == null)
    )' "$TRANSCRIPT_PATH" > "$TEMP_NEW"

  # 既存ファイルと新エントリを結合して重複除去
  TEMP_COMBINED=$(mktemp)
  cat "$DEST_FILE" "$TEMP_NEW" > "$TEMP_COMBINED"

  # 重複除去（最後のエントリを保持）してDEST_FILEに保存（JSONL形式を維持）
  jq -c -s 'reverse | unique_by(.uuid // .messageId) | reverse | .[]' "$TEMP_COMBINED" > "$DEST_FILE"

  # 一時ファイルを削除
  rm "$TEMP_NEW" "$TEMP_COMBINED"

  echo "Session log updated (diff appended, deduplicated): $DEST_FILE" >&2
  echo "${TIMESTAMP} | ${SESSION_ID} | ${REASON} (diff) | ${DEST_FILE}" >> "$INDEX_FILE"
else
  # 新規ファイル作成時も重複除去（JSONL形式を維持）
  jq -c -s 'reverse | unique_by(.uuid // .messageId) | reverse | .[]' "$TRANSCRIPT_PATH" > "$DEST_FILE"

  echo "Session log saved (new file, deduplicated): $DEST_FILE" >&2
  echo "${TIMESTAMP} | ${SESSION_ID} | ${REASON} (new) | ${DEST_FILE}" >> "$INDEX_FILE"
fi

# エージェントログの処理
if [ -n "$CWD" ] && [ -n "$SESSION_ID" ]; then
  # プロジェクトディレクトリを特定（既にPROJECT_NAMEは設定済み）
  PROJECTS_DIR="$HOME/.claude/projects/$PROJECT_NAME"

  # agent-*.jsonl ファイルを検索
  for AGENT_LOG in "$PROJECTS_DIR"/agent-*.jsonl; do
    # ファイルが存在するか確認（グロブが展開されなかった場合をスキップ）
    if [ ! -f "$AGENT_LOG" ]; then
      continue
    fi

    # エージェントIDを抽出（例: agent-a2a9065.jsonl → a2a9065）
    AGENT_ID=$(basename "$AGENT_LOG" .jsonl | sed 's/agent-//')

    # 保存先ファイル名
    AGENT_DEST_FILE="${LOG_DIR}/${PROJECT_NAME}_agent-${AGENT_ID}.jsonl"

    # 重複除去して保存（メインログと同じロジック）
    if [ -f "$AGENT_DEST_FILE" ]; then
      # 既存ファイルがある場合：差分追記
      EXISTING_IDS_JSON=$(jq -r 'if .uuid then .uuid elif .messageId then .messageId else empty end' "$AGENT_DEST_FILE" | jq -R -s 'split("\n") | map(select(length > 0))')

      TEMP_NEW=$(mktemp)
      jq --argjson existing_ids "$EXISTING_IDS_JSON" \
        'select(
          ((.uuid // .messageId) as $id | ($existing_ids | index($id)) == null)
        )' "$AGENT_LOG" > "$TEMP_NEW"

      TEMP_COMBINED=$(mktemp)
      cat "$AGENT_DEST_FILE" "$TEMP_NEW" > "$TEMP_COMBINED"

      jq -c -s 'reverse | unique_by(.uuid // .messageId) | reverse | .[]' "$TEMP_COMBINED" > "$AGENT_DEST_FILE"

      rm "$TEMP_NEW" "$TEMP_COMBINED"

      echo "Agent log updated (diff appended, deduplicated): $AGENT_DEST_FILE" >&2
    else
      # 新規ファイル作成
      jq -c -s 'reverse | unique_by(.uuid // .messageId) | reverse | .[]' "$AGENT_LOG" > "$AGENT_DEST_FILE"

      echo "Agent log saved (new file, deduplicated): $AGENT_DEST_FILE" >&2
    fi

    # index.txtに記録
    echo "${TIMESTAMP} | agent-${AGENT_ID} | ${REASON} | ${AGENT_DEST_FILE}" >> "$INDEX_FILE"
  done
fi

exit 0
