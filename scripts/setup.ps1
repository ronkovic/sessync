# sessync セットアップスクリプト (Windows)
# 使い方: PowerShellで実行
#   iwr -useb https://raw.githubusercontent.com/ronkovic/sessync/main/scripts/setup.ps1 | iex
#   .\setup.ps1 -Version v0.1.0  # バージョン指定

param(
    [string]$Version = "latest"
)

$ErrorActionPreference = "Stop"
$Repo = "ronkovic/sessync"

function Write-Info { param($msg) Write-Host "[INFO] $msg" -ForegroundColor Green }
function Write-Warn { param($msg) Write-Host "[WARN] $msg" -ForegroundColor Yellow }
function Write-Err  { param($msg) Write-Host "[ERROR] $msg" -ForegroundColor Red; exit 1 }

# 最新バージョン取得
function Get-LatestVersion {
    try {
        $release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest" -UseBasicParsing
        return $release.tag_name
    } catch {
        Write-Err "Failed to fetch latest version from GitHub API"
    }
}

# バイナリダウンロード
function Install-Binary {
    param($ver)

    $url = "https://github.com/$Repo/releases/download/$ver/sessync-windows-x86_64.zip"
    $temp = "$env:TEMP\sessync-$([guid]::NewGuid()).zip"

    Write-Info "Downloading: $url"

    try {
        Invoke-WebRequest $url -OutFile $temp -UseBasicParsing
    } catch {
        Write-Err "Failed to download binary. Check if version $ver exists."
    }

    # 展開先を確認してから展開
    Expand-Archive $temp -DestinationPath ".claude\sessync" -Force

    # sessync.exeが直接展開されたか確認
    if (-not (Test-Path ".claude\sessync\sessync.exe")) {
        # サブディレクトリに展開された場合
        $exePath = Get-ChildItem -Path ".claude\sessync" -Filter "sessync.exe" -Recurse | Select-Object -First 1
        if ($exePath) {
            Move-Item $exePath.FullName ".claude\sessync\sessync.exe" -Force
        }
    }

    Remove-Item $temp -ErrorAction SilentlyContinue

    Write-Info "Binary: .claude\sessync\sessync.exe"
}

# config.json (新規のみ)
function Setup-ConfigJson {
    if (-not (Test-Path ".claude\sessync\config.json")) {
        Invoke-WebRequest "https://raw.githubusercontent.com/$Repo/main/examples/config.json.example" `
            -OutFile ".claude\sessync\config.json" -UseBasicParsing
        Write-Info "Created: .claude\sessync\config.json (要編集)"
    } else {
        Write-Warn "Exists: .claude\sessync\config.json (skipped)"
    }
}

# settings.json (マージ)
function Setup-SettingsJson {
    $file = ".claude\settings.json"
    $hook = @{
        hooks = @(@{
            type = "command"
            command = ".\.claude\sessync\sessync.exe --auto"
            timeout = 60
        })
    }

    if (-not (Test-Path $file)) {
        @{ hooks = @{ SessionEnd = @($hook) } } | ConvertTo-Json -Depth 10 | Set-Content $file -Encoding UTF8
        Write-Info "Created: $file"
        return
    }

    # 既存ファイルを読み込み
    $content = Get-Content $file -Raw
    if ($content -match "sessync") {
        Write-Warn "Exists: sessync hook already in $file"
        return
    }

    try {
        $settings = $content | ConvertFrom-Json

        # hooksプロパティがなければ作成
        if (-not $settings.hooks) {
            $settings | Add-Member -NotePropertyName "hooks" -NotePropertyValue @{} -Force
        }

        # SessionEndプロパティがなければ作成
        if (-not $settings.hooks.SessionEnd) {
            $settings.hooks | Add-Member -NotePropertyName "SessionEnd" -NotePropertyValue @() -Force
        }

        # フックを追加
        $settings.hooks.SessionEnd = @($settings.hooks.SessionEnd) + $hook

        $settings | ConvertTo-Json -Depth 10 | Set-Content $file -Encoding UTF8
        Write-Info "Merged: SessionEnd hook added to $file"
    } catch {
        Write-Warn "Failed to parse $file. Please add SessionEnd hook manually."
    }
}

# save-session.md (常に上書き)
function Setup-SaveSession {
    Invoke-WebRequest "https://raw.githubusercontent.com/$Repo/main/.claude/commands/save-session.md" `
        -OutFile ".claude\commands\save-session.md" -UseBasicParsing
    Write-Info "Updated: .claude\commands\save-session.md"
}

# .gitignore (差分追記)
function Setup-Gitignore {
    $entries = @(
        "# sessync"
        ".claude/sessync/service-account-key.json"
        ".claude/sessync/config.json"
        ".claude/sessync/upload-state.json"
        ".claude/sessync/sessync"
        ".claude/sessync/sessync.exe"
    )

    $added = 0
    $current = if (Test-Path .gitignore) { Get-Content .gitignore } else { @() }

    foreach ($entry in $entries) {
        if ($entry -notin $current) {
            Add-Content .gitignore $entry
            $added++
        }
    }

    if ($added -gt 0) {
        Write-Info "Updated: .gitignore (+$added entries)"
    } else {
        Write-Info "Exists: .gitignore (all entries present)"
    }
}

# メイン処理
Write-Host "========================================"
Write-Host "  sessync Setup Script (Windows)"
Write-Host "========================================"
Write-Host ""

if ($Version -eq "latest") {
    $Version = Get-LatestVersion
}
Write-Info "Version: $Version"

# ディレクトリ作成
New-Item -ItemType Directory -Force -Path ".claude\sessync" | Out-Null
New-Item -ItemType Directory -Force -Path ".claude\commands" | Out-Null

Install-Binary -ver $Version
Setup-ConfigJson
Setup-SettingsJson
Setup-SaveSession
Setup-Gitignore

Write-Host ""
Write-Host "✅ sessync installed!" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:"
Write-Host "  1. Edit .claude\sessync\config.json with your BigQuery settings"
Write-Host "  2. Add your service account key:"
Write-Host "     Copy-Item C:\path\to\key.json .claude\sessync\service-account-key.json"
Write-Host "  3. Test: .\.claude\sessync\sessync.exe --dry-run"
Write-Host ""
