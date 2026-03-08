# GitHub Publication Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Prepare the Jarvis fork for clean publication at https://github.com/vovazlo97/jarvis — correct .gitignore, automated setup.ps1, comprehensive README.md, then git init + push.

**Architecture:** No git history to clean (repo is not yet initialized). We init fresh, craft .gitignore before staging anything, write all docs, then force-push to the existing GitHub remote.

**Tech Stack:** Rust/Cargo workspace, Tauri v2, Svelte/Vite frontend, Vosk STT, PowerShell setup script.

---

## Task 1: Update .gitignore

**Files:**
- Modify: `.gitignore`

**Step 1: Replace .gitignore with complete version**

Write the following content to `.gitignore` (replaces existing file):

```gitignore
# ── Logs ──────────────────────────────────────────────────────────────────────
logs/
*.log
npm-debug.log*
yarn-debug.log*
yarn-error.log*
pnpm-debug.log*

# ── Editor ────────────────────────────────────────────────────────────────────
.vscode/
!.vscode/extensions.json
.idea/
.DS_Store
*.suo
*.ntvs*
*.njsproj
*.sln
*.sw?

# ── Rust / Cargo ──────────────────────────────────────────────────────────────
/target/
**/target/

# Rust compilation artifacts
*.rlib
*.rmeta
*.rs.bk
*.crate

# ── Node / Frontend ───────────────────────────────────────────────────────────
frontend/node_modules/
frontend/dist/
node_modules/

# ── AI / STT Models (download via setup.ps1) ──────────────────────────────────
resources/vosk/
resources/models/
resources/vosk-api-*/

# ── Runtime binaries at repo root (present in lib/windows/amd64/) ─────────────
/libvosk.dll

# ── Internal / Dev files ──────────────────────────────────────────────────────
.claude/
.serena/
CLAUDE.md
tsk.md
__other/
list.py
tree.txt

# ── Misc ──────────────────────────────────────────────────────────────────────
*.psd
```

**Step 2: Verify no large paths would be committed**

```bash
# From project root — should show only source files, no target/ or node_modules
git check-ignore -v target/ frontend/node_modules/ resources/vosk/ resources/models/
```

Expected output: each path is listed as ignored.

---

## Task 2: Create setup.ps1

**Files:**
- Create: `setup.ps1`

**Step 1: Write setup.ps1**

```powershell
<#
.SYNOPSIS
    JARVIS Voice Assistant — one-command setup script.
    Downloads Vosk models and prepares the project for first build.
.EXAMPLE
    powershell -ExecutionPolicy Bypass -File setup.ps1
#>

$ErrorActionPreference = "Stop"

function Write-Header { param($msg) Write-Host "`n=== $msg ===" -ForegroundColor Cyan }
function Write-OK     { param($msg) Write-Host "  ✅ $msg" -ForegroundColor Green }
function Write-Fail   { param($msg) Write-Host "  ❌ $msg" -ForegroundColor Red; exit 1 }
function Write-Info   { param($msg) Write-Host "  ℹ  $msg" -ForegroundColor Yellow }

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
Write-Host "`n" -NoNewline
Write-Host "============================================" -ForegroundColor Green
Write-Host "  ✅  Setup complete! Start the app with:"  -ForegroundColor Green
Write-Host ""                                            -ForegroundColor Green
Write-Host "     cargo tauri dev"                        -ForegroundColor White
Write-Host ""                                            -ForegroundColor Green
Write-Host "  Run from: crates/jarvis-gui/"              -ForegroundColor Yellow
Write-Host "============================================" -ForegroundColor Green
```

**Step 2: Test script syntax (no execution, just parse)**

```powershell
powershell -NoProfile -Command "Get-Content setup.ps1 | Out-Null; Write-Host 'Syntax OK'"
```

Expected: `Syntax OK`

---

## Task 3: Write README.md

**Files:**
- Modify: `README.md` (full rewrite)

**Step 1: Write complete README.md**

```markdown
# JARVIS Voice Assistant

![JARVIS Voice Assistant](poster.jpg)

> **100% offline · Open source · No data collection**

A privacy-first voice assistant built with Rust and Tauri. Speak a command — JARVIS hears it, understands it, and acts. No cloud, no subscription, no telemetry.

**This fork** extends the original [Priler/jarvis](https://github.com/Priler/jarvis) with a full **Commands & Scripts system** — a low-code GUI for building personal voice-activated workflows without touching Rust code.

---

## What this fork adds

| Feature | Description |
|---|---|
| **Commands GUI** | Create, edit and delete voice commands through a visual interface |
| **Scripts Engine** | Chain multiple commands into sequential or parallel workflows |
| **Voice-activated workflows** | Trigger any script with a spoken phrase or regex pattern |
| **Scripting without code** | Build automations (open app → wait → play music) entirely from the GUI |
| **Custom voice responses** | Assign specific audio responses per command or script |
| **Sound Manager** | Browse, import and preview voice pack sounds from Settings |

**Example use case:** Say *"work mode"* → JARVIS opens Chrome with YouTube, waits 2 seconds, then starts your Spotify playlist — all configured through the GUI.

---

## System Requirements

| Requirement | Minimum | Recommended |
|---|---|---|
| **OS** | Windows 10 x64 | Windows 11 x64 |
| **RAM** | 4 GB | 8 GB |
| **CPU** | Any x64 (2 cores) | 4+ cores |
| **Microphone** | Any | USB/headset mic |
| **Rust** | 1.75+ | latest stable |
| **Node.js** | 18+ | 20+ LTS |
| **Disk** | 2 GB free | 4 GB free |

> **Linux / macOS:** Not tested. Contributions welcome.

---

## Installation

### Option A — Automatic (recommended)

One script downloads models and installs all dependencies:

```powershell
git clone https://github.com/vovazlo97/jarvis.git
cd jarvis

# Run setup (downloads ~200 MB of Vosk language models)
powershell -ExecutionPolicy Bypass -File setup.ps1

# Start the app
cd crates/jarvis-gui
cargo tauri dev
```

### Option B — Manual step by step

**1. Install Rust**

Go to https://rustup.rs/ and follow instructions, or run:

```powershell
winget install Rustlang.Rustup
```

Verify:
```powershell
rustc --version   # should print rustc 1.75.0 or later
cargo --version
```

**2. Install Node.js**

Download v18+ from https://nodejs.org/ (LTS recommended).

Verify:
```powershell
node --version    # should print v18.x.x or later
npm --version
```

**3. Install Tauri CLI**

```powershell
cargo install tauri-cli --version "^2"
```

**4. Clone the repository**

```powershell
git clone https://github.com/vovazlo97/jarvis.git
cd jarvis
```

**5. Download Vosk language models**

Create directory `resources/vosk/` and download models manually:

| Model | Language | Size | URL |
|---|---|---|---|
| `vosk-model-small-ru-0.22` | Russian | ~40 MB | https://alphacephei.com/vosk/models/vosk-model-small-ru-0.22.zip |
| `vosk-model-en-us-0.22-lgraph` | English | ~128 MB | https://alphacephei.com/vosk/models/vosk-model-en-us-0.22-lgraph.zip |
| `vosk-model-small-uk-v3-nano` | Ukrainian | ~10 MB | https://alphacephei.com/vosk/models/vosk-model-small-uk-v3-nano.zip |

Extract each zip so the folder structure looks like:
```
resources/
  vosk/
    vosk-model-small-ru-0.22/
    vosk-model-en-us-0.22-lgraph/
    vosk-model-small-uk-v3-nano/
```

**6. Install frontend dependencies**

```powershell
cd frontend
npm install
cd ..
```

**7. Start the application**

```powershell
cd crates/jarvis-gui
cargo tauri dev
```

> First build takes 5–15 minutes — Rust compiles all dependencies including embedding models.
> Subsequent builds are incremental and much faster.

---

## Project Structure

```
jarvis/
├── crates/
│   ├── jarvis-core/        # Core library: audio, STT, commands, scripts, intent
│   ├── jarvis-app/         # Voice engine binary (wake word → STT → routing)
│   ├── jarvis-gui/         # Tauri desktop app (GUI + Tauri commands)
│   └── jarvis-cli/         # CLI tool for testing
├── frontend/
│   └── src/
│       └── routes/
│           ├── commands/   # Commands GUI (pack list + editor)
│           └── scripts/    # Scripts GUI (script list + step editor)
├── resources/
│   ├── commands/           # Command packs (TOML files, one folder per pack)
│   │   ├── browser/command.toml
│   │   ├── games/command.toml
│   │   └── ...
│   ├── scripts/            # Script files (created via GUI, stored as TOML)
│   ├── sound/              # Voice packs and audio feedback
│   ├── vosk/               # Vosk STT models (downloaded by setup.ps1)
│   └── keywords/           # Wake word detection files
├── lib/
│   └── windows/amd64/      # Runtime DLLs (Vosk, PvRecorder)
├── setup.ps1               # One-command setup script
└── Cargo.toml              # Workspace manifest
```

---

## Commands System

A **command** is a single voice-triggered action — open an app, launch a URL, run a shell command, or control the system.

### Structure

Commands are stored in TOML files under `resources/commands/<pack-name>/command.toml`.

**Example — open YouTube:**

```toml
[[commands]]
id = "open_youtube"
type = "exe"
exe_path = "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe"
exe_args = ["https://youtube.com"]

phrases.ru = ["открой ютуб", "запусти ютуб", "включи ютуб"]
phrases.en = ["open youtube", "launch youtube", "go to youtube"]
patterns   = ["ютуб[а-яё]*|youtube"]

sounds.ru = ["ok1", "ok2", "ok3"]
```

### Command types

| Type | What it does | Key fields |
|---|---|---|
| `exe` | Launch an executable or open a URL | `exe_path`, `exe_args` |
| `cli` | Run a shell/PowerShell command | `cli_cmd`, `cli_args` |

### Adding a command via GUI

1. Open JARVIS → **Commands** tab
2. Select a pack (or create a new one)
3. Click **Add command**
4. Fill in: ID, type, executable path, voice phrases
5. Click **Save** — command is immediately active

### Voice matching

When you speak, JARVIS tries to match your phrase using (in order):
1. **Regex patterns** — exact regex match (`patterns` field)
2. **Intent classifier** — embedding-based semantic matching (≥ 88% confidence)
3. **Fuzzy matching** — Levenshtein-based fallback (≥ 75% similarity)

If nothing matches → plays `not_found` audio.

---

## Scripts System

A **script** is a sequence of commands triggered by a single voice phrase. Use scripts to automate multi-step workflows.

### Structure

Scripts are stored as TOML files in `resources/scripts/` (created via GUI).

**Example — "Work Mode":**

```toml
id          = "work_mode"
name        = "Work Mode"
description = "Opens browser and starts music for productive work"
mode        = "sequential"

phrases_ru = ["режим работы", "рабочий режим", "включи рабочий режим"]
phrases_en = ["work mode", "enable work mode"]
patterns   = ["режим.?работ"]

[[steps]]
step_type  = "command_ref"
pack       = "browser"
command_id = "open_youtube"

[[steps]]
step_type = "delay"
delay_ms  = 2000

[[steps]]
step_type        = "spotify"
spotify_action   = "play_track"
spotify_track_id = "4uLU6hMCjMI75M1A2tKUQC"

[[steps]]
step_type = "custom"
cli_cmd   = "powershell"
cli_args  = ["-Command", "Start-Process notepad"]
label     = "Open Notepad"
```

### Execution modes

| Mode | Behavior |
|---|---|
| `sequential` | Steps run one by one, respecting `delay` pauses |
| `parallel` | All steps start simultaneously in separate threads |

### Step types

| Type | Description | Key fields |
|---|---|---|
| `command_ref` | Execute an existing command from a pack | `pack`, `command_id` |
| `delay` | Wait N milliseconds | `delay_ms` |
| `custom` | Run any CLI/PowerShell command | `cli_cmd`, `cli_args` |
| `spotify` | Control Spotify | `spotify_action`, `spotify_track_id` |

### Adding a script via GUI

1. Open JARVIS → **Scripts** tab
2. Click **New script**
3. Set ID, name, execution mode
4. Add voice trigger phrases (Russian/English) or regex patterns
5. Add steps using the step editor
6. Click **Save** — script is immediately active (no restart needed)

---

## Commands vs Scripts

| | Command | Script |
|---|---|---|
| **Purpose** | Single action | Multi-step workflow |
| **Storage** | `resources/commands/<pack>/command.toml` | `resources/scripts/<id>.toml` |
| **Steps** | One (exe / cli) | Many (any combination) |
| **Order** | — | Sequential or Parallel |
| **Best for** | "Open Chrome", "Volume up" | "Work mode", "Gaming setup" |

---

## Adding Custom Workflows

**Scenario:** You want to say *"start gaming"* and have JARVIS:
1. Open Steam
2. Wait 3 seconds
3. Start your gaming Spotify playlist

**Step 1 — Create a command pack for Steam** (if not exists):

Create `resources/commands/games/command.toml`:
```toml
[[commands]]
id       = "open_steam"
type     = "exe"
exe_path = "C:\\Program Files (x86)\\Steam\\steam.exe"
exe_args = []
phrases.ru = ["открой стим", "запусти стим"]
phrases.en = ["open steam", "launch steam"]
sounds.ru  = ["ok1", "ok2"]
```

**Step 2 — Create a script in the GUI:**

- ID: `gaming_mode`
- Phrases: `["игровой режим", "режим игры"]`
- Steps:
  - `command_ref` → pack: `games`, command: `open_steam`
  - `delay` → 3000 ms
  - `spotify` → play_track → your playlist track ID

**Step 3 — Say the phrase.** Done.

---

## Troubleshooting

### Build fails: `linker error` or `libvosk not found`

The linker needs `lib/windows/amd64/` to be present. Verify the folder exists and contains `libvosk.dll` and `libvosk.lib`.

```powershell
ls lib/windows/amd64/
```

### Build fails: `cargo-tauri not found`

```powershell
cargo install tauri-cli --version "^2"
```

### GUI doesn't open / blank window

Make sure frontend dependencies are installed:
```powershell
cd frontend
npm install
cd ..
```
Then rerun `cargo tauri dev` from `crates/jarvis-gui/`.

### Voice not recognized

1. Check your microphone is set as the default recording device in Windows Sound settings
2. Verify Vosk models are in `resources/vosk/` with correct folder names
3. Check app logs printed to the terminal where you ran `cargo tauri dev`

### Models not found at startup

Run setup again:
```powershell
powershell -ExecutionPolicy Bypass -File setup.ps1
```

### First build is very slow

Normal — Rust compiles all dependencies from scratch. This takes 5–15 minutes once. Subsequent builds are incremental (< 1 min).

### Embedding models download during build

The first `cargo build` fetches embedding models (~100 MB) via `fastembed`. This is automatic — just let it finish.

---

## Neural Networks Used

| Purpose | Library |
|---|---|
| Speech-to-Text | [Vosk](https://github.com/alphacep/vosk-api) via [vosk-rs](https://github.com/Bear-03/vosk-rs) |
| Intent Classification | [fastembed](https://github.com/Anush008/fastembed-rs) (all-MiniLM-L6-v2) |
| Wake Word | [Rustpotter](https://github.com/GiviMAD/rustpotter) *(WIP)* |

---

## Development

```powershell
# Check all crates compile
cargo check --workspace

# Run core unit tests
cargo test -p jarvis-core

# Build release binary
cargo tauri build --cwd crates/jarvis-gui

# Check Rust lints
cargo clippy --workspace
```

Logs are printed to the terminal. Set `RUST_LOG=debug` for verbose output:
```powershell
$env:RUST_LOG="debug"; cargo tauri dev
```

---

## Credits

- Original project: [Priler/jarvis](https://github.com/Priler/jarvis) by Abraham Tugalov
- This fork: [@vovazlo97](https://github.com/vovazlo97)

## License

[Attribution-NonCommercial-ShareAlike 4.0 International](https://creativecommons.org/licenses/by-nc-sa/4.0/)
See `LICENSE.txt` for full terms.
```

---

## Task 4: Initialize git and push to GitHub

**Files:** none (git operations only)

**Step 1: Initialize git repo**

```bash
cd D:/Jarvis/jarvis2/jarvis-master/jarvis-master
git init
git branch -M main
```

**Step 2: Verify .gitignore is working — check what would be staged**

```bash
# Should show only source files, NOT target/ node_modules/ resources/vosk/ etc.
git status --short | head -40
```

Verify these paths do NOT appear:
- `target/`
- `frontend/node_modules/`
- `resources/vosk/`
- `resources/models/`
- `CLAUDE.md`
- `.claude/`

**Step 3: Stage all source files**

```bash
git add .
git status --short | wc -l   # count staged files — should be a small number
```

**Step 4: Create initial commit**

```bash
git commit -m "feat: initial public release — Commands & Scripts system

- Full GUI Commands editor (packs, CRUD, voice phrases)
- Scripts Engine: multi-step voice-activated workflows
- Sequential and parallel execution modes
- Spotify, Shell, Delay, CommandRef step types
- Voice response customization per command/script
- Sound Manager in Settings
- Automated setup.ps1 for one-command install
- Comprehensive README with manual + auto setup paths

Fork of Priler/jarvis — adds scripting and workflow automation."
```

**Step 5: Add remote and push**

```bash
git remote add origin https://github.com/vovazlo97/jarvis.git
git push --force origin main
```

> `--force` is needed because the remote has an old version. This overwrites it.

**Step 6: Verify on GitHub**

Open https://github.com/vovazlo97/jarvis in browser and confirm:
- README renders correctly with poster.jpg
- `setup.ps1` is visible
- No `target/`, `node_modules/`, `resources/vosk/` folders present
