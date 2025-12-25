# sessync セットアップスクリプト (Windows) - 対話式
# 使い方: PowerShellで実行
#   iwr -useb https://raw.githubusercontent.com/ronkovic/sessync/main/scripts/setup.ps1 | iex
#   .\setup.ps1 -Version v0.1.0                    # バージョン指定
#   .\setup.ps1 -ProjectDir C:\path\to\project     # プロジェクトパス指定（非対話）

param(
    [string]$Version = "latest",
    [string]$ProjectDir = ""
)

$ErrorActionPreference = "Stop"
$Repo = "ronkovic/sessync"

function Write-Info { param($msg) Write-Host "[INFO] $msg" -ForegroundColor Green }
function Write-Warn { param($msg) Write-Host "[WARN] $msg" -ForegroundColor Yellow }
function Write-Err  { param($msg) Write-Host "[ERROR] $msg" -ForegroundColor Red; exit 1 }
function Write-Prompt { param($msg) Write-Host "[?] $msg" -ForegroundColor Cyan }

# プロジェクトディレクトリの選択（対話式）
function Select-ProjectDir {
    if ($ProjectDir -ne "") {
        # 引数で指定された場合
        if (-not (Test-Path $ProjectDir -PathType Container)) {
            Write-Err "Directory not found: $ProjectDir"
        }
        $script:ProjectDir = (Resolve-Path $ProjectDir).Path
        return
    }

    Write-Host ""
    Write-Prompt "sessyncをインストールするプロジェクトフォルダを入力してください"
    Write-Host "  (空白でEnter = 現在のディレクトリ: $(Get-Location))" -ForegroundColor Yellow
    Write-Host ""
    $inputDir = Read-Host ">"

    if ([string]::IsNullOrWhiteSpace($inputDir)) {
        $script:ProjectDir = (Get-Location).Path
        Write-Info "Using current directory: $script:ProjectDir"
    } else {
        # 環境変数展開
        $inputDir = [Environment]::ExpandEnvironmentVariables($inputDir)

        if (-not (Test-Path $inputDir -PathType Container)) {
            Write-Host ""
            Write-Prompt "ディレクトリが存在しません: $inputDir"
            $createDir = Read-Host "  作成しますか? [y/N]"
            if ($createDir -match "^[Yy]$") {
                New-Item -ItemType Directory -Force -Path $inputDir | Out-Null
                Write-Info "Created directory: $inputDir"
            } else {
                Write-Err "Directory not found: $inputDir"
            }
        }

        $script:ProjectDir = (Resolve-Path $inputDir).Path
        Write-Info "Target directory: $script:ProjectDir"
    }
}

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

# config.json (対話式設定)
function Setup-ConfigJson {
    if (Test-Path ".claude\sessync\config.json") {
        Write-Warn "Exists: .claude\sessync\config.json (skipped)"
        return
    }

    Write-Host ""
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
    Write-Host "  BigQuery設定" -ForegroundColor Cyan
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
    Write-Host ""

    # project_name
    $projectBasename = Split-Path $ProjectDir -Leaf
    Write-Prompt "プロジェクト名 (default: $projectBasename)"
    $cfgProjectName = Read-Host ">"
    if ([string]::IsNullOrWhiteSpace($cfgProjectName)) { $cfgProjectName = $projectBasename }

    # project_id
    Write-Prompt "GCPプロジェクトID (default: $cfgProjectName)"
    $cfgProjectId = Read-Host ">"
    if ([string]::IsNullOrWhiteSpace($cfgProjectId)) { $cfgProjectId = $cfgProjectName }

    # dataset
    Write-Prompt "BigQueryデータセット名 (default: claude_sessions)"
    $cfgDataset = Read-Host ">"
    if ([string]::IsNullOrWhiteSpace($cfgDataset)) { $cfgDataset = "claude_sessions" }

    # table
    Write-Prompt "BigQueryテーブル名 (default: session_logs)"
    $cfgTable = Read-Host ">"
    if ([string]::IsNullOrWhiteSpace($cfgTable)) { $cfgTable = "session_logs" }

    # location
    Write-Prompt "BigQueryロケーション (default: US)"
    $cfgLocation = Read-Host ">"
    if ([string]::IsNullOrWhiteSpace($cfgLocation)) { $cfgLocation = "US" }

    # developer_id
    $defaultDevId = $env:USERNAME
    Write-Prompt "開発者ID (default: $defaultDevId)"
    $cfgDeveloperId = Read-Host ">"
    if ([string]::IsNullOrWhiteSpace($cfgDeveloperId)) { $cfgDeveloperId = $defaultDevId }

    # user_email
    $defaultEmail = ""
    try { $defaultEmail = git config --global user.email 2>$null } catch {}
    if ($defaultEmail) {
        Write-Prompt "メールアドレス (default: $defaultEmail)"
    } else {
        Write-Prompt "メールアドレス"
    }
    $cfgUserEmail = Read-Host ">"
    if ([string]::IsNullOrWhiteSpace($cfgUserEmail)) { $cfgUserEmail = $defaultEmail }

    # service_account_key_path
    Write-Prompt "サービスアカウントキーパス (default: .\.claude\sessync\service-account-key.json)"
    $cfgKeyPath = Read-Host ">"
    if ([string]::IsNullOrWhiteSpace($cfgKeyPath)) { $cfgKeyPath = ".\.claude\sessync\service-account-key.json" }

    # config.json を生成
    $config = @{
        project_id = $cfgProjectId
        dataset = $cfgDataset
        table = $cfgTable
        location = $cfgLocation
        upload_batch_size = 500
        enable_auto_upload = $true
        enable_deduplication = $true
        developer_id = $cfgDeveloperId
        user_email = $cfgUserEmail
        project_name = $cfgProjectName
        service_account_key_path = $cfgKeyPath
    }

    $config | ConvertTo-Json -Depth 10 | Set-Content ".claude\sessync\config.json" -Encoding UTF8
    Write-Info "Created: .claude\sessync\config.json"
    Write-Host ""
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

# サービスアカウントキーのコピー（対話式）
function Setup-ServiceAccountKey {
    $targetKeyPath = ".claude\sessync\service-account-key.json"

    if (Test-Path $targetKeyPath) {
        Write-Info "Service account key already exists: $targetKeyPath"
        return
    }

    Write-Host ""
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
    Write-Host "  サービスアカウントキーの配置" -ForegroundColor Cyan
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
    Write-Host ""

    Write-Prompt "サービスアカウントキーのパスを入力してください"
    Write-Host "  (空白でEnter = 後で手動でコピー)" -ForegroundColor Yellow
    Write-Host ""
    $keyPath = Read-Host ">"

    if ([string]::IsNullOrWhiteSpace($keyPath)) {
        Write-Warn "スキップしました。後でキーファイルをコピーしてください:"
        Write-Host "  Copy-Item C:\path\to\key.json $ProjectDir\$targetKeyPath"
        return
    }

    # 環境変数展開
    $keyPath = [Environment]::ExpandEnvironmentVariables($keyPath)

    if (-not (Test-Path $keyPath -PathType Leaf)) {
        Write-Warn "ファイルが見つかりません: $keyPath"
        Write-Warn "後でキーファイルをコピーしてください:"
        Write-Host "  Copy-Item C:\path\to\key.json $ProjectDir\$targetKeyPath"
        return
    }

    Copy-Item $keyPath $targetKeyPath -Force
    # Windowsでのファイル権限設定（所有者のみアクセス可能）
    $acl = Get-Acl $targetKeyPath
    $acl.SetAccessRuleProtection($true, $false)
    $owner = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
    $rule = New-Object System.Security.AccessControl.FileSystemAccessRule($owner, "FullControl", "Allow")
    $acl.SetAccessRule($rule)
    Set-Acl $targetKeyPath $acl

    Write-Info "Copied: $targetKeyPath (permissions: owner only)"
    Write-Host ""
}

# メイン処理
Write-Host "========================================"
Write-Host "  sessync Setup Script (Windows)"
Write-Host "========================================"

# プロジェクトディレクトリ選択（対話式）
Select-ProjectDir

# プロジェクトディレクトリに移動
Set-Location $ProjectDir

Write-Host ""
Write-Host "Installing to: $ProjectDir" -ForegroundColor Cyan
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
Setup-ServiceAccountKey

Write-Host ""
Write-Host "✅ sessync installed!" -ForegroundColor Green
Write-Host ""
Write-Host "Installed to: $ProjectDir"
Write-Host ""

if (-not (Test-Path ".claude\sessync\service-account-key.json")) {
    Write-Host "Next steps:"
    Write-Host "  1. Add your service account key:"
    Write-Host "     Copy-Item C:\path\to\key.json $ProjectDir\.claude\sessync\service-account-key.json"
    Write-Host "  2. (Optional) Edit $ProjectDir\.claude\sessync\config.json if needed"
    Write-Host "  3. Test: cd $ProjectDir; .\.claude\sessync\sessync.exe --dry-run"
} else {
    Write-Host "Next steps:"
    Write-Host "  1. (Optional) Edit $ProjectDir\.claude\sessync\config.json if needed"
    Write-Host "  2. Test: cd $ProjectDir; .\.claude\sessync\sessync.exe --dry-run"
}
Write-Host ""
