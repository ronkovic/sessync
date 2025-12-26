#!/bin/bash
set -e

# sessync セットアップスクリプト（対話式）
# 使い方:
#   curl -sSL https://raw.githubusercontent.com/ronkovic/sessync/main/scripts/setup.sh | bash
#   curl -sSL ... | bash -s -- -v v0.1.0           # バージョン指定
#   curl -sSL ... | bash -s -- -p /path/to/project # プロジェクトパス指定（非対話）

REPO="ronkovic/sessync"
VERSION="latest"
PROJECT_DIR=""
TEMP_FILE=""

# 色付き出力
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC} $1"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }
prompt() { echo -e "${CYAN}[?]${NC} $1"; }

# クリーンアップ
cleanup() {
  if [ -n "$TEMP_FILE" ] && [ -f "$TEMP_FILE" ]; then
    rm -f "$TEMP_FILE"
  fi
}
trap cleanup EXIT

# 引数パース
parse_args() {
  while [ $# -gt 0 ]; do
    case "$1" in
      -v|--version)
        VERSION="$2"
        shift 2
        ;;
      -p|--project)
        PROJECT_DIR="$2"
        shift 2
        ;;
      -h|--help)
        echo "Usage: setup.sh [-v VERSION] [-p PROJECT_DIR]"
        echo ""
        echo "Options:"
        echo "  -v, --version VERSION   Specify version (default: latest)"
        echo "  -p, --project PATH      Project directory (skip interactive prompt)"
        echo "  -h, --help              Show this help"
        exit 0
        ;;
      *)
        # 位置引数としてバージョンを受け付ける（後方互換性）
        if [ "$VERSION" = "latest" ]; then
          VERSION="$1"
        fi
        shift
        ;;
    esac
  done
}

# プロジェクトディレクトリの選択（対話式）
select_project_dir() {
  if [ -n "$PROJECT_DIR" ]; then
    # 引数で指定された場合
    if [ ! -d "$PROJECT_DIR" ]; then
      error "Directory not found: $PROJECT_DIR"
    fi
    PROJECT_DIR=$(cd "$PROJECT_DIR" && pwd)
    return
  fi

  echo ""
  prompt "sessyncをインストールするプロジェクトフォルダを入力してください"
  echo -e "  ${YELLOW}(空白でEnter = 現在のディレクトリ: $(pwd))${NC}"
  echo ""
  read -r -p "> " input_dir < /dev/tty

  if [ -z "$input_dir" ]; then
    PROJECT_DIR="$(pwd)"
    info "Using current directory: $PROJECT_DIR"
  else
    # チルダ展開
    input_dir="${input_dir/#\~/$HOME}"

    if [ ! -d "$input_dir" ]; then
      echo ""
      prompt "ディレクトリが存在しません: $input_dir"
      read -r -p "  作成しますか? [y/N] " create_dir < /dev/tty
      if [[ "$create_dir" =~ ^[Yy]$ ]]; then
        mkdir -p "$input_dir"
        info "Created directory: $input_dir"
      else
        error "Directory not found: $input_dir"
      fi
    fi

    PROJECT_DIR=$(cd "$input_dir" && pwd)
    info "Target directory: $PROJECT_DIR"
  fi
}

# プラットフォーム検出
detect_platform() {
  local os arch
  os=$(uname -s | tr '[:upper:]' '[:lower:]')
  arch=$(uname -m)

  case "$os" in
    linux)
      case "$arch" in
        x86_64) PLATFORM="linux-x86_64" ;;
        *) error "Unsupported Linux architecture: $arch (only x86_64 supported)" ;;
      esac
      ;;
    darwin)
      case "$arch" in
        x86_64) PLATFORM="darwin-x86_64" ;;
        arm64)  PLATFORM="darwin-arm64" ;;
        *) error "Unsupported macOS architecture: $arch" ;;
      esac
      ;;
    *)
      error "Unsupported OS: $os (only Linux/macOS supported)"
      ;;
  esac

  info "Platform: $PLATFORM"
}

# 最新バージョン取得
get_latest_version() {
  local version
  version=$(curl -sf "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null | \
    grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')

  if [ -z "$version" ]; then
    error "Failed to fetch latest version from GitHub API"
  fi

  echo "$version"
}

# バイナリダウンロード
download_binary() {
  local url
  url="https://github.com/$REPO/releases/download/$VERSION/sessync-$PLATFORM.tar.gz"

  info "Downloading: $url"

  TEMP_FILE=$(mktemp)
  if ! curl -sfL "$url" -o "$TEMP_FILE"; then
    error "Failed to download binary. Check if version $VERSION exists."
  fi

  tar -xzf "$TEMP_FILE" -C .claude/sessync/
  chmod +x .claude/sessync/sessync

  info "Binary: .claude/sessync/sessync"
}

# config.json (対話式設定)
setup_config_json() {
  if [ -f .claude/sessync/config.json ]; then
    warn "Exists: .claude/sessync/config.json (skipped)"
    return
  fi

  echo ""
  echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${CYAN}  BigQuery設定${NC}"
  echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""

  # project_name
  local project_basename
  project_basename=$(basename "$PROJECT_DIR")
  prompt "プロジェクト名 (default: $project_basename)"
  read -r -p "> " cfg_project_name < /dev/tty
  cfg_project_name="${cfg_project_name:-$project_basename}"

  # project_id
  prompt "GCPプロジェクトID (default: $cfg_project_name)"
  read -r -p "> " cfg_project_id < /dev/tty
  cfg_project_id="${cfg_project_id:-$cfg_project_name}"

  # dataset
  prompt "BigQueryデータセット名 (default: claude_sessions)"
  read -r -p "> " cfg_dataset < /dev/tty
  cfg_dataset="${cfg_dataset:-claude_sessions}"

  # table
  prompt "BigQueryテーブル名 (default: session_logs)"
  read -r -p "> " cfg_table < /dev/tty
  cfg_table="${cfg_table:-session_logs}"

  # location
  prompt "BigQueryロケーション (default: US)"
  read -r -p "> " cfg_location < /dev/tty
  cfg_location="${cfg_location:-US}"

  # developer_id
  local default_dev_id
  default_dev_id=$(whoami)
  prompt "開発者ID (default: $default_dev_id)"
  read -r -p "> " cfg_developer_id < /dev/tty
  cfg_developer_id="${cfg_developer_id:-$default_dev_id}"

  # user_email
  local default_email=""
  if command -v git &>/dev/null; then
    default_email=$(git config --global user.email 2>/dev/null || echo "")
  fi
  if [ -n "$default_email" ]; then
    prompt "メールアドレス (default: $default_email)"
  else
    prompt "メールアドレス"
  fi
  read -r -p "> " cfg_user_email < /dev/tty
  cfg_user_email="${cfg_user_email:-$default_email}"

  # service_account_key_path
  prompt "サービスアカウントキーパス (default: ./.claude/sessync/service-account-key.json)"
  read -r -p "> " cfg_key_path < /dev/tty
  cfg_key_path="${cfg_key_path:-./.claude/sessync/service-account-key.json}"

  # config.json を生成
  cat > .claude/sessync/config.json << EOF
{
  "project_id": "$cfg_project_id",
  "dataset": "$cfg_dataset",
  "table": "$cfg_table",
  "location": "$cfg_location",
  "upload_batch_size": 500,
  "enable_auto_upload": true,
  "enable_deduplication": true,
  "developer_id": "$cfg_developer_id",
  "user_email": "$cfg_user_email",
  "project_name": "$cfg_project_name",
  "service_account_key_path": "$cfg_key_path"
}
EOF

  info "Created: .claude/sessync/config.json"
  echo ""
}

# settings.json (マージ)
setup_settings_json() {
  local settings_file=".claude/settings.json"

  if [ ! -f "$settings_file" ]; then
    cat > "$settings_file" << 'SETTINGS_EOF'
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
SETTINGS_EOF
    info "Created: $settings_file"
    return
  fi

  if grep -q "sessync" "$settings_file" 2>/dev/null; then
    warn "Exists: sessync hook already in $settings_file"
    return
  fi

  local python_cmd=""
  if command -v python3 &>/dev/null; then
    python_cmd="python3"
  elif command -v python &>/dev/null; then
    python_cmd="python"
  fi

  if [ -n "$python_cmd" ]; then
    $python_cmd << 'PYTHON_EOF'
import json

hook = {
    "hooks": [{
        "type": "command",
        "command": "./.claude/sessync/sessync --auto",
        "timeout": 60
    }]
}

with open(".claude/settings.json", "r") as f:
    data = json.load(f)

if "hooks" not in data:
    data["hooks"] = {}
if "SessionEnd" not in data["hooks"]:
    data["hooks"]["SessionEnd"] = []

data["hooks"]["SessionEnd"].append(hook)

with open(".claude/settings.json", "w") as f:
    json.dump(data, f, indent=2)

print("[INFO] Merged: SessionEnd hook added")
PYTHON_EOF
  else
    warn "Python not found. Please add SessionEnd hook manually:"
    warn "  See: https://github.com/$REPO#sessionend-hook"
  fi
}

# save-session.md (常に上書き)
setup_save_session() {
  curl -sfL "https://raw.githubusercontent.com/$REPO/main/examples/claude-commands/save-session.md" \
    -o .claude/commands/save-session.md
  info "Updated: .claude/commands/save-session.md"
}

# .gitignore (差分追記)
setup_gitignore() {
  local entries=(
    "# sessync"
    ".claude/sessync/service-account-key.json"
    ".claude/sessync/config.json"
    ".claude/sessync/upload-state.json"
    ".claude/sessync/sessync"
    ".claude/sessync/sessync.exe"
  )

  local added=0
  for entry in "${entries[@]}"; do
    if ! grep -qF "$entry" .gitignore 2>/dev/null; then
      echo "$entry" >> .gitignore
      ((added++)) || true
    fi
  done

  if [ $added -gt 0 ]; then
    info "Updated: .gitignore (+$added entries)"
  else
    info "Exists: .gitignore (all entries present)"
  fi
}

# サービスアカウントキーのコピー（対話式）
setup_service_account_key() {
  local target_key_path=".claude/sessync/service-account-key.json"

  if [ -f "$target_key_path" ]; then
    info "Service account key already exists: $target_key_path"
    return
  fi

  echo ""
  echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${CYAN}  サービスアカウントキーの配置${NC}"
  echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""

  prompt "サービスアカウントキーのパスを入力してください"
  echo -e "  ${YELLOW}(空白でEnter = 後で手動でコピー)${NC}"
  echo ""
  read -r -p "> " key_path < /dev/tty

  if [ -z "$key_path" ]; then
    warn "スキップしました。後でキーファイルをコピーしてください:"
    echo "  cp /path/to/key.json $PROJECT_DIR/$target_key_path"
    return
  fi

  # チルダ展開
  key_path="${key_path/#\~/$HOME}"

  if [ ! -f "$key_path" ]; then
    warn "ファイルが見つかりません: $key_path"
    warn "後でキーファイルをコピーしてください:"
    echo "  cp /path/to/key.json $PROJECT_DIR/$target_key_path"
    return
  fi

  cp "$key_path" "$target_key_path"
  chmod 600 "$target_key_path"
  info "Copied: $target_key_path (permissions: 600)"
  echo ""
}

main() {
  echo "========================================"
  echo "  sessync Setup Script"
  echo "========================================"

  # 引数パース
  parse_args "$@"

  # プロジェクトディレクトリ選択（対話式）
  select_project_dir

  # プロジェクトディレクトリに移動
  cd "$PROJECT_DIR" || error "Failed to change directory: $PROJECT_DIR"

  echo ""
  echo -e "Installing to: ${CYAN}$PROJECT_DIR${NC}"
  echo ""

  detect_platform

  if [ "$VERSION" = "latest" ]; then
    VERSION=$(get_latest_version)
  fi
  info "Version: $VERSION"

  mkdir -p .claude/sessync .claude/commands

  download_binary
  setup_config_json
  setup_settings_json
  setup_save_session
  setup_gitignore
  setup_service_account_key

  echo ""
  echo -e "${GREEN}✅ sessync installed!${NC}"
  echo ""
  echo "Installed to: $PROJECT_DIR"
  echo ""

  if [ ! -f ".claude/sessync/service-account-key.json" ]; then
    echo "Next steps:"
    echo "  1. Add your service account key:"
    echo "     cp /path/to/key.json $PROJECT_DIR/.claude/sessync/service-account-key.json"
    echo "  2. (Optional) Edit $PROJECT_DIR/.claude/sessync/config.json if needed"
    echo "  3. Test: cd $PROJECT_DIR && ./.claude/sessync/sessync --dry-run"
  else
    echo "Next steps:"
    echo "  1. (Optional) Edit $PROJECT_DIR/.claude/sessync/config.json if needed"
    echo "  2. Test: cd $PROJECT_DIR && ./.claude/sessync/sessync --dry-run"
  fi
  echo ""
}

main "$@"
