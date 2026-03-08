# JARVIS v0.2.0 — Developer & User Guide

## Quick Start

### Prerequisites
- Rust 1.75+
- Node.js 18+
- Tauri CLI: `cargo install tauri-cli`

### Run in development mode
```bash
cd jarvis-master
cargo tauri dev
```

### Build release
```bash
cargo tauri build
```

---

## Project Structure

```
jarvis-master/
├── crates/
│   ├── jarvis-core/        # Core voice engine (STT, matching, scripts, commands)
│   │   └── src/
│   │       ├── scripts.rs  # Script data model, fuzzy matching, execution
│   │       ├── commands/   # JCommand struct, command parsing
│   │       ├── voices.rs   # Voice pack management, sound playback
│   │       └── config.rs   # All thresholds and constants
│   ├── jarvis-app/         # Background voice process (VAD, STT, wake word)
│   │   └── src/app.rs      # Main recognition loop + command/script dispatch
│   └── jarvis-gui/         # Tauri desktop GUI
│       └── src/
│           ├── main.rs                      # Tauri builder + invoke_handler
│           └── tauri_commands/
│               ├── scripts.rs               # Script CRUD + execution Tauri commands
│               ├── voices.rs                # Voice listing + sound file management
│               └── audio.rs                 # play_sound command
├── frontend/
│   └── src/routes/
│       ├── scripts/index.svelte   # Scripts builder UI
│       ├── commands/index.svelte  # Commands + Scripts combined editor
│       └── settings/index.svelte  # Settings with Sound Manager tab
└── resources/
    ├── scripts/        # Script TOML files (created by GUI)
    ├── commands/       # Command pack directories (*.toml)
    └── sound/
        └── voices/
            ├── <voice-pack>/      # Built-in voice packs
            └── user_custom/       # Files imported via Sound Manager
```

---

## Features Guide

### 1. Scripting Engine

Scripts live in `resources/scripts/<id>.toml`. Create/edit them via GUI (Scripts page or Commands > Scripts tab).

**Execution modes:**
- `sequential` — steps run one after another; Delay steps actually pause execution
- `parallel` — all steps spawn simultaneously in separate threads; Delay steps are ignored in this mode

**Step types:**

| Type | Required fields | Description |
|------|----------------|-------------|
| `command_ref` | `pack`, `command_id` | Runs an existing command from a pack |
| `delay` | `delay_ms` | Pauses execution (ms); sequential only |
| `custom` | `cli_cmd`, `cli_args` | Runs CMD/PowerShell command |
| `spotify` | `spotify_action`, `spotify_track_id`* | Controls Spotify via protocol/keys |

*`spotify_track_id` only required for `play_track` action.

**Spotify actions:**
- `play_track` — opens `spotify:track:<id>` URI via `Start-Process`
- `pause` — sends media-key pause via WScript.Shell
- `next` — sends media-key next track via WScript.Shell

---

### 2. Custom Voice Responses (`response_sound`)

Both **commands** and **scripts** support a `response_sound` field.

**How it works:**
1. On voice trigger, JARVIS checks `response_sound`
2. If non-empty: plays the file at `SOUND_DIR/<response_sound>`
3. If empty: plays random sound from the current voice pack (default behaviour)

**Path format:** relative from `SOUND_DIR` (= `<app_dir>/resources/sound`), e.g.:
```
voices/user_custom/general/ok_jarvis.wav
voices/jarvis-og/ru/ok1.wav
```

**Set via GUI:** In the command or script modal → expand "Voice triggers" → "Response Sound" dropdown.

---

### 3. Sound Resource Manager

**Location:** Settings → "Звуки" tab

**Features:**
- View all `.wav`/`.mp3`/`.ogg` files in `resources/sound/voices/`
- Click `▶` to preview any file (uses the same audio engine as voice responses)
- Import new files: enter a category name → click "Добавить звук" → pick file
  - Files are copied to `resources/sound/voices/user_custom/<category>/`
  - Immediately appear in the list and in all "Response Sound" dropdowns

**Tauri commands:**
- `list_sound_files()` → `Vec<String>` (relative paths from `SOUND_DIR`)
- `import_sound_file(src_path, category)` → `Result<String, String>`

---

### 4. Matching Thresholds

All thresholds live in `crates/jarvis-core/src/config.rs`:

```rust
pub const INTENT_CLASSIFIER_MIN_CONFIDENCE: f64 = 0.88; // must exceed for intent to fire
pub const CMD_RATIO_THRESHOLD: f64 = 75f64;             // fuzzy fallback for commands
pub const SCRIPT_RATIO_THRESHOLD: f64 = 88f64;          // fuzzy fallback for scripts (stricter)
```

**Intent classifier** fires first (fastest). Requires ≥ 88% confidence — anything weaker falls through to fuzzy matching. This prevents phonetically similar but unrelated phrases (e.g. "гитхаб" scored at 82% → "open_youtube") from triggering the wrong command.

**Script fuzzy** uses a **harmonic mean** of trigger-coverage and input-coverage — both the trigger phrase AND the spoken input must be well-matched. Prevents a 2-word trigger from matching an unrelated short phrase.

To adjust, edit `config.rs` and rebuild.

---

### 5. Voice Activation Flow

```
Voice input
    │
    ├─► Intent classifier (trained on command + script phrases)
    │       └─ confidence >= 0.88? → execute command or script
    │                (below 0.88 → falls through, no false positive)
    │
    └─► (if no intent match) Levenshtein fuzzy on commands (>= 75%)
            └─► (if still no match) Fuzzy on scripts (>= 88%)
                    └─► (if still no match) play not_found.wav
```

---

## Adding a New Script Manually (TOML)

Create `resources/scripts/my_script.toml`:

```toml
id = "my_script"
name = "My Script"
description = "Optional description"
mode = "sequential"
phrases_ru = ["запусти мой скрипт", "мой сценарий"]
phrases_en = ["run my script"]
patterns = []
sounds_ru = []
response_sound = ""   # leave empty for voice-pack default

[[steps]]
step_type = "command_ref"
pack = "browser"
command_id = "open_youtube"

[[steps]]
step_type = "delay"
delay_ms = 1500

[[steps]]
step_type = "spotify"
spotify_action = "play_track"
spotify_track_id = "4uLU6hMCjMI75M1A2tKUQC"
label = "Play favourite track"
```

The script is picked up immediately — no restart required (live disk reads).

---

## GitHub Push Checklist

Before pushing:

1. `cargo build -p jarvis-core -p jarvis-gui -p jarvis-app` — must produce **no errors**
2. `cd frontend && npm run build` — frontend must compile clean
3. Verify `resources/scripts/` and `resources/sound/voices/user_custom/` are in `.gitignore` if they contain personal data
4. Check that no API keys are committed (`.env`, `config.json`)

```bash
git add crates/ frontend/src/ README.md INSTRUCTION.md CLAUDE.md
git commit -m "feat: v0.2.0 — Spotify steps, response_sound, Sound Manager, script threshold fix"
git push origin main
```

---

## Troubleshooting

**Script fires on every unrecognized phrase**
- Raise `SCRIPT_RATIO_THRESHOLD` in `config.rs` (try `92f64`)
- Add more specific voice trigger phrases (multi-word = harder to false-match)
  - Bad:  `phrases_ru = ["запусти"]`    ← single word, fires on ANYTHING that says "запусти"
  - Good: `phrases_ru = ["запусти режим работы", "рабочий режим"]`
- Check logs for "Script fuzzy match" lines to see actual scores

**Command fires on wrong phrase ("гитхаб" → YouTube)**
- This was caused by `INTENT_CLASSIFIER_MIN_CONFIDENCE = 0.75` being too permissive.
- Fixed in v0.2.1: raised to `0.88` — only near-exact intent matches fire.
- If after fix a command stops triggering, lower the threshold slightly (try `0.85`).
- **Requires app restart** after editing `config.rs` — the classifier trains at startup.

**Spotify step doesn't work**
- Spotify must be installed and logged in
- For `play_track`: copy the track ID from the Spotify URL (the part after `track/`)
- `pause`/`next` use media keys — requires Spotify to be in focus or in background

**Sound file not appearing in dropdown**
- File must be in `resources/sound/voices/` (any subdirectory)
- Supported extensions: `.wav`, `.mp3`, `.ogg`
- Re-open the modal after importing (list reloads on open)

**`play_sound` command not found**
- Ensure `tauri_commands::play_sound` is in `invoke_handler![]` in `main.rs` (it is by default)
