# JARVIS Voice Assistant

![JARVIS Voice Assistant](poster.jpg)

> **100% offline · Open source · No data collection**

A privacy-first voice assistant built with Rust and Tauri. Speak a command — JARVIS hears it, understands it, and acts. No cloud, no subscription, no telemetry.

**This fork** extends the original [Priler/jarvis](https://github.com/Priler/jarvis) with a full **Commands & Scripts system** — a low-code GUI for building personal voice-activated workflows without writing any Rust code.

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

**Example:** Say *"work mode"* → JARVIS opens Chrome with YouTube, waits 2 seconds, then starts your Spotify playlist — all configured through the GUI, no code needed.

---

## Commands & Scripts Visual Guide

### Command Editor

![JARVIS Command Editor](CommandEditor.jpg)

Manage single voice-triggered actions entirely through the GUI. Supports `EXE APP`, `CHROME / URL`, `CLI / POWERSHELL`, and `VOICE ONLY` action types with per-language trigger phrases and custom response sounds.

### Script Editor & Step Builder

![JARVIS Script Editor](ScriptEditor.jpg)

Automate multi-step workflows. Run steps one by one (`Sequential`) or all at once (`Parallel`). Add delays, cross-command references, Spotify controls, and custom shell scripts via the visual builder.

### Sound Manager

![JARVIS Sound Manager](SoundManager.jpg)

Manage all audio feedback from Settings — browse, preview, import WAV files, and filter sounds by category.

---

## System Requirements

| Requirement | Minimum | Recommended |
|---|---|---|
| **OS** | Windows 10 x64 | Windows 11 x64 |
| **RAM** | 4 GB | 8 GB |
| **CPU** | Any x64 (2+ cores) | 4+ cores |
| **Microphone** | Any | USB or headset mic |
| **Rust** | 1.75+ | latest stable |
| **Node.js** | 18+ | 20 LTS |
| **Disk space** | 2 GB free | 4 GB free |

> **Linux / macOS:** Not tested. Contributions welcome.

---

## System Prerequisites (Windows)

Before installing Rust, install the **Microsoft C++ Build Tools** — required by Rust's MSVC toolchain:

1. Download from [https://visualstudio.microsoft.com/visual-cpp-build-tools/](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
2. Select the **"Desktop development with C++"** workload
3. Click **Install** (~5–8 GB)

> Skip this step if you already have **Visual Studio 2019/2022** with the C++ workload.

Without the C++ Build Tools, `cargo build -p jarvis-app` will fail with a linker error.

---

## Installation

### Option A — Automatic (recommended)

```powershell
# 1. Clone the repository
git clone https://github.com/vovazlo97/jarvis.git
cd jarvis

# 2. Run setup (downloads Vosk models ~200 MB, installs npm packages)
powershell -ExecutionPolicy Bypass -File setup.ps1

# 3. Build both binaries at once
cargo build -p jarvis-app -p jarvis-gui

# 4. Terminal 1 — start the backend voice engine
.\target\debug\jarvis-app.exe

# 5. Terminal 2 — start the GUI
cd crates/jarvis-gui
cargo tauri dev
```

> **Two terminals are required.** `jarvis-app` (voice engine) and `jarvis-gui` (configuration window) are separate processes that communicate via IPC.

> First build takes **5–15 minutes** — Rust compiles all dependencies from scratch. Subsequent builds are under 1 minute.

---

### Option B — Manual step by step

**Step 1 — Install Rust**

```powershell
winget install Rustlang.Rustup
```

Restart your terminal, then verify:

```powershell
rustc --version   # rustc 1.75.0 or later
cargo --version
```

---

**Step 2 — Install Node.js**

Download v18+ (LTS) from https://nodejs.org/ and verify:

```powershell
node --version    # v18.x.x or later
npm --version
```

---

**Step 3 — Install Tauri CLI**

```powershell
cargo install tauri-cli --version "^2"
```

---

**Step 4 — Clone the repository**

```powershell
git clone https://github.com/vovazlo97/jarvis.git
cd jarvis
```

---

**Step 5 — Download Vosk speech models**

Create the `resources/vosk/` directory and extract each zip so the structure matches exactly:

```
resources/
  vosk/
    vosk-model-small-ru-0.22/
    vosk-model-en-us-0.22-lgraph/
    vosk-model-small-uk-v3-nano/
```

| Model folder | Language | Size | Download |
|---|---|---|---|
| `vosk-model-small-ru-0.22` | Russian | ~40 MB | https://alphacephei.com/vosk/models/vosk-model-small-ru-0.22.zip |
| `vosk-model-en-us-0.22-lgraph` | English | ~128 MB | https://alphacephei.com/vosk/models/vosk-model-en-us-0.22-lgraph.zip |
| `vosk-model-small-uk-v3-nano` | Ukrainian | ~10 MB | https://alphacephei.com/vosk/models/vosk-model-small-uk-v3-nano.zip |

---

**Step 6 — Install frontend dependencies**

```powershell
cd frontend
npm install
cd ..
```

---

**Step 7 — Running in Development Mode**

The application is made of **two separate processes** that must both run simultaneously:

| Process | Crate | Role |
|---|---|---|
| `jarvis-app.exe` | `crates/jarvis-app` | Voice engine: wake word → STT → command routing |
| `jarvis-gui` (Tauri window) | `crates/jarvis-gui` | Configuration GUI |

```powershell
# Quick: build both binaries at once (from repository root)
cargo build -p jarvis-app -p jarvis-gui
```

Then in two separate terminals:

```powershell
# Terminal 1 — start the backend
.\target\debug\jarvis-app.exe

# Terminal 2 — start the GUI with hot reload
cd crates/jarvis-gui
cargo tauri dev
```

During the first build, `fastembed` will download embedding models (~100 MB) automatically.

**Debug logging:**

```powershell
# Terminal 1
$env:RUST_LOG = "debug"
.\target\debug\jarvis-app.exe

# Terminal 2
$env:RUST_LOG = "debug"
cd crates/jarvis-gui
cargo tauri dev
```

---

**Step 8 — Building for Production**

```powershell
# Build both release binaries at once (from repository root)
cargo build --release -p jarvis-app -p jarvis-gui
# Output: target/release/jarvis-app.exe  +  target/release/jarvis-gui.exe
```

To build the full Tauri installer (NSIS/MSI):

```powershell
cd crates/jarvis-gui
cargo tauri build
# Output: target/release/bundle/nsis/jarvis-app_<version>_x64-setup.exe
#         target/release/bundle/msi/jarvis-app_<version>_x64_en-US.msi
```

> The Tauri installer **does not yet bundle `jarvis-app.exe` automatically** (see [Required Config Changes](#required-config-changes)). Until that is applied, ship the installer and `jarvis-app.exe` together, or place `jarvis-app.exe` in the same folder as the installed `jarvis-gui.exe`.

All DLLs (`libvosk.dll`, `libpv_recorder.dll`, etc.) from `lib/windows/amd64/` are bundled into the installer automatically via `bundle.resources` in `tauri.conf.json`.

---

## Project Structure

```
jarvis/
├── crates/
│   ├── jarvis-core/        # Core library: audio, STT, commands, scripts, intent
│   ├── jarvis-app/         # Voice engine binary (wake word → STT → command routing)
│   ├── jarvis-gui/         # Tauri desktop app (GUI + backend Tauri commands)
│   └── jarvis-cli/         # CLI tool for development and testing
├── frontend/
│   └── src/routes/
│       ├── commands/       # Commands GUI (pack list + command editor)
│       └── scripts/        # Scripts GUI (script list + step builder)
├── resources/
│   ├── commands/           # Command packs — one subfolder per pack, each with command.toml
│   ├── scripts/            # Script TOML files (created and managed via GUI)
│   ├── sound/              # Voice packs and audio feedback files
│   ├── vosk/               # Vosk STT models (downloaded by setup.ps1, not in git)
│   └── keywords/           # Wake word detection files
├── lib/
│   └── windows/amd64/      # Runtime DLLs required for build (Vosk, PvRecorder)
├── .cargo/config.toml      # Linker flags pointing to lib/windows/amd64/
├── setup.ps1               # One-command setup script
├── Cargo.toml              # Workspace manifest
└── Cargo.lock
```

---

## Commands System

A **command** is a single voice-triggered action — open an application, navigate to a URL, run a shell command, or control the system.

### Voice matching pipeline

| Step | Method | Threshold |
|---|---|---|
| 1 | Regex pattern match | Exact |
| 2 | Embedding intent classifier | ≥ 88% confidence |
| 3 | Fuzzy (Levenshtein) fallback | ≥ 75% similarity |

### Command file structure

Commands live in `resources/commands/<pack-name>/command.toml`:

```toml
[[commands]]
id         = "open_youtube"
type       = "exe"
exe_path   = "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe"
exe_args   = ["https://youtube.com"]

phrases.ru = ["открой ютуб", "запусти ютуб", "включи ютуб"]
phrases.en = ["open youtube", "launch youtube", "go to youtube"]
patterns   = ["ютуб[а-яё]*|youtube"]

sounds.ru  = ["ok1", "ok2", "ok3"]
```

### Command types

| Type | What it does | Required fields |
|---|---|---|
| `exe` | Launch an executable or open a URL | `exe_path`, `exe_args` |
| `cli` | Run a PowerShell or CMD command | `cli_cmd`, `cli_args` |

### Adding a command via GUI

1. Open JARVIS → **Commands** tab
2. Select or create a pack
3. Click **Add command**, fill in fields, click **Save**

---

## Scripts System

A **script** chains multiple steps triggered by a single voice phrase.

### Execution modes

| Mode | Behavior |
|---|---|
| `sequential` | Steps run one by one; `delay` steps pause execution |
| `parallel` | All steps start simultaneously in separate threads |

### Step types

| Type | Description | Key fields |
|---|---|---|
| `command_ref` | Run an existing command from any pack | `pack`, `command_id` |
| `delay` | Pause for N milliseconds | `delay_ms` |
| `custom` | Run any PowerShell or CMD command | `cli_cmd`, `cli_args` |
| `spotify` | Control Spotify playback | `spotify_action`, `spotify_track_id` |

### Script file structure

```toml
id          = "work_mode"
name        = "Work Mode"
mode        = "sequential"

phrases_ru = ["режим работы", "рабочий режим"]
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
```

### Adding a script via GUI

1. Open JARVIS → **Scripts** tab → **New script**
2. Set ID, name, mode, trigger phrases, regex patterns
3. Add steps via the visual builder → **Save** (active immediately, no restart)

---

## Commands vs Scripts

| | Command | Script |
|---|---|---|
| **Purpose** | One action | Multi-step workflow |
| **Storage** | `resources/commands/<pack>/command.toml` | `resources/scripts/<id>.toml` |
| **Steps** | Single (exe or cli) | Many (any combination) |
| **Use when** | "Open Chrome", "Volume up" | "Work mode", "Gaming setup" |

---

## Adding Custom Workflows

**Scenario:** Say *"gaming mode"* → open Steam, wait 3 seconds, start Spotify playlist.

**Step 1 — Ensure Steam has a command** (`resources/commands/games/command.toml`):

```toml
[[commands]]
id         = "open_steam"
type       = "exe"
exe_path   = "C:\\Program Files (x86)\\Steam\\steam.exe"
exe_args   = []
phrases.ru = ["открой стим", "запусти стим"]
phrases.en = ["open steam", "launch steam"]
```

**Step 2 — Create the script** (Scripts → New script):

| Field | Value |
|---|---|
| ID | `gaming_mode` |
| Mode | `sequential` |
| Phrases (RU) | `игровой режим`, `режим игры` |
| Phrases (EN) | `gaming mode`, `start gaming` |

Steps:
1. `command_ref` → pack: `games`, command: `open_steam`
2. `delay` → `3000` ms
3. `spotify` → your track URI

---

## Required Config Changes

> These changes are **not yet applied**. Needed for production builds where `jarvis-app.exe` is bundled into the installer and auto-launched by the GUI.

### 1. `crates/jarvis-gui/tauri.conf.json` — add `externalBin`

```json
"bundle": {
  "externalBin": [
    "../../target/release/jarvis-app"
  ],
  "resources": {
    "../../resources/commands": "resources/commands",
    "../../resources/sound": "resources/sound",
    "../../resources/rustpotter": "resources/rustpotter",
    "../../resources/vosk": "resources/vosk",
    "../../resources/keywords": "resources/keywords",
    "../../lib/windows/amd64/*.dll": "."
  }
}
```

### 2. `crates/jarvis-gui/src/main.rs` — auto-spawn backend on startup

```rust
let _backend = app.shell()
    .sidecar("jarvis-app")
    .expect("jarvis-app sidecar not configured")
    .spawn()
    .expect("failed to spawn jarvis-app");
```

### 3. `crates/jarvis-gui/Cargo.toml` — no change needed

`jarvis-app` is a binary crate, not a library. The two-process architecture is intentional — processes communicate via IPC sockets defined in `jarvis-core`.

---

## Troubleshooting

### Build error: `linker error` or `libvosk not found`

```powershell
ls lib/windows/amd64/
# Expected: libvosk.dll, libvosk.lib, libpv_recorder.dll, etc.
```

Re-clone the repository if the folder is missing.

### Build error: `cargo-tauri: command not found`

```powershell
cargo install tauri-cli --version "^2"
```

### GUI opens but shows a blank window

```powershell
cd frontend && npm install && cd ..
cd crates/jarvis-gui && cargo tauri dev
```

### GUI opens but voice commands have no effect

`jarvis-app.exe` is not running. Start it in a separate terminal:

```powershell
.\target\debug\jarvis-app.exe   # development
.\target\release\jarvis-app.exe # release
```

### Backend exits immediately or crashes

1. **Missing Vosk models** — run `setup.ps1` or download manually (Step 5)
2. **Missing DLLs** — run `cargo build -p jarvis-app` first (`build.rs` copies DLLs automatically)
3. **Full log** — `$env:RUST_LOG = "debug"; .\target\debug\jarvis-app.exe`

### Voice commands are not recognized

1. Confirm `jarvis-app.exe` is running
2. Set your microphone as the default recording device in Windows Sound Settings
3. Verify models in `resources/vosk/` with exact folder names
4. Check backend terminal output for STT errors

### Build downloads ~100 MB during compilation

The `fastembed` crate downloads ONNX embedding model weights on first build. Automatic, happens only once.

---

## Neural Networks Used

| Purpose | Library | Notes |
|---|---|---|
| Speech-to-Text | [Vosk](https://github.com/alphacep/vosk-api) via [vosk-rs](https://github.com/Bear-03/vosk-rs) | Fully offline |
| Intent Classification | [fastembed](https://github.com/Anush008/fastembed-rs) — all-MiniLM-L6-v2 | Offline, downloads on first build |
| Wake Word | [Rustpotter](https://github.com/GiviMAD/rustpotter) | Partially implemented |

---

## Development

```powershell
# Check all crates compile without building
cargo check --workspace

# Run core library unit tests
cargo test -p jarvis-core

# Lint
cargo clippy --workspace

# Build both debug binaries at once
cargo build -p jarvis-app -p jarvis-gui

# Build both release binaries at once
cargo build --release -p jarvis-app -p jarvis-gui

# Full Tauri installer
cd crates/jarvis-gui && cargo tauri build
```

---

## Credits

- Original project: [Priler/jarvis](https://github.com/Priler/jarvis) by Abraham Tugalov
- This fork: [@vovazlo97](https://github.com/vovazlo97)

## License

[Attribution-NonCommercial-ShareAlike 4.0 International](https://creativecommons.org/licenses/by-nc-sa/4.0/)
See `LICENSE.txt` for full terms.
