#!/bin/bash
set -e

# sessync セットアップスクリプト
# 使い方: 対象プロジェクトのルートから実行
#   curl -sSL https://raw.githubusercontent.com/ronkovic/sessync/main/scripts/setup.sh | bash
#   curl -sSL ... | bash -s v0.1.0  # バージョン指定

REPO="ronkovic/sessync"
VERSION="${1:-latest}"
TEMP_FILE=""

# 色付き出力
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC} $1"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# クリーンアップ
cleanup() {
  if [ -n "$TEMP_FILE" ] && [ -f "$TEMP_FILE" ]; then
    rm -f "$TEMP_FILE"
  fi
}
trap cleanup EXIT

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

# config.json (新規のみ)
setup_config_json() {
  if [ ! -f .claude/sessync/config.json ]; then
    curl -sfL "https://raw.githubusercontent.com/$REPO/main/examples/config.json.example" \
      -o .claude/sessync/config.json
    info "Created: .claude/sessync/config.json (要編集)"
  else
    warn "Exists: .claude/sessync/config.json (skipped)"
  fi
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
  curl -sfL "https://raw.githubusercontent.com/$REPO/main/.claude/commands/save-session.md" \
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

main() {
  echo "========================================"
  echo "  sessync Setup Script"
  echo "========================================"
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

  echo ""
  echo -e "${GREEN}✅ sessync installed!${NC}"
  echo ""
  echo "Next steps:"
  echo "  1. Edit .claude/sessync/config.json with your BigQuery settings"
  echo "  2. Add your service account key:"
  echo "     cp /path/to/key.json .claude/sessync/service-account-key.json"
  echo "  3. Test: ./.claude/sessync/sessync --dry-run"
  echo ""
}

main
