<#
.SYNOPSIS
    JARVIS Voice Assistant — one-command setup script.
    Downloads Vosk models and prepares the project for first build.
.EXAMPLE
    powershell -ExecutionPolicy Bypass -File setup.ps1
#>

$ErrorActionPreference = "Stop"

function Write-Header { param($msg) Write-Host "`n=== $msg ===" -ForegroundColor Cyan }
function Write-OK     { param($msg) Write-Host "  OK $msg" -ForegroundColor Green }
function Write-Fail   { param($msg) Write-Host "  FAIL $msg" -ForegroundColor Red; exit 1 }
function Write-Info   { param($msg) Write-Host "  INFO $msg" -ForegroundColor Yellow }

# ── 1. Check Rust ──────────────────────────────────────────────────────────────
Write-Header "Checking Rust"
if (Get-Command rustup -ErrorAction SilentlyContinue) {
    $rustVer = (rustc --version)
    Write-OK "Rust found: $rustVer"
} else {
    Write-Fail "Rust not found. Install from https://rustup.rs/ then re-run this script."
}

# ── 2. Check Node.js ──────────────────────────────────────────────────────────
Write-Header "Checking Node.js"
if (Get-Command node -ErrorAction SilentlyContinue) {
    $nodeVer = (node --version)
    $major = [int]($nodeVer -replace 'v(\d+)\..*','$1')
    if ($major -lt 18) {
        Write-Fail "Node.js $nodeVer found, but v18+ is required. Update from https://nodejs.org/"
    }
    Write-OK "Node.js found: $nodeVer"
} else {
    Write-Fail "Node.js not found. Install v18+ from https://nodejs.org/ then re-run this script."
}

# ── 3. Check / install Tauri CLI ───────────────────────────────────────────────
Write-Header "Checking Tauri CLI"
if (Get-Command cargo-tauri -ErrorAction SilentlyContinue) {
    Write-OK "Tauri CLI already installed"
} else {
    Write-Info "Installing Tauri CLI (this may take a few minutes)..."
    cargo install tauri-cli --version "^2"
    Write-OK "Tauri CLI installed"
}

# ── 4. Download Vosk models ───────────────────────────────────────────────────
Write-Header "Downloading Vosk speech models"

$voskDir = Join-Path $PSScriptRoot "resources\vosk"
New-Item -ItemType Directory -Force -Path $voskDir | Out-Null

$models = @(
    @{
        Name = "vosk-model-small-ru-0.22"
        Url  = "https://alphacephei.com/vosk/models/vosk-model-small-ru-0.22.zip"
        Desc = "Russian (small, ~40 MB)"
    },
    @{
        Name = "vosk-model-en-us-0.22-lgraph"
        Url  = "https://alphacephei.com/vosk/models/vosk-model-en-us-0.22-lgraph.zip"
        Desc = "English (lgraph, ~128 MB)"
    },
    @{
        Name = "vosk-model-small-uk-v3-nano"
        Url  = "https://alphacephei.com/vosk/models/vosk-model-small-uk-v3-nano.zip"
        Desc = "Ukrainian (nano)"
    }
)

foreach ($m in $models) {
    $destDir = Join-Path $voskDir $m.Name
    if (Test-Path $destDir) {
        Write-OK "$($m.Name) already present, skipping"
        continue
    }
    Write-Info "Downloading $($m.Desc)..."
    $zip = Join-Path $env:TEMP "$($m.Name).zip"
    Invoke-WebRequest -Uri $m.Url -OutFile $zip -UseBasicParsing
    Write-Info "Extracting..."
    Expand-Archive -Path $zip -DestinationPath $voskDir -Force
    Remove-Item $zip
    Write-OK "$($m.Name) ready"
}

# ── 5. Install frontend dependencies ──────────────────────────────────────────
Write-Header "Installing frontend dependencies"
$frontendDir = Join-Path $PSScriptRoot "frontend"
Push-Location $frontendDir
npm install
Pop-Location
Write-OK "Frontend dependencies installed"

# ── Done ──────────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "============================================" -ForegroundColor Green
Write-Host "  Setup complete! Start the app with:"      -ForegroundColor Green
Write-Host ""
Write-Host "     cd crates/jarvis-gui"                  -ForegroundColor White
Write-Host "     cargo tauri dev"                       -ForegroundColor White
Write-Host ""
Write-Host "  First build takes 5-15 min (Rust compiles deps)" -ForegroundColor Yellow
Write-Host "============================================" -ForegroundColor Green
