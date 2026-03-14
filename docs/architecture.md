# Jarvis Architecture

> Version: 0.2.0 (Phase A) · Stack: Rust 1.75+ / Tauri / Vosk / Vite + Svelte + TypeScript

## Overview

Jarvis is a 100% offline, privacy-first voice assistant and automation OS. There are no cloud dependencies for core functionality. The architecture is built around one hard constraint: the voice pipeline (Fast Path) must complete end-to-end in **<250ms P50, <400ms P95**.

---

## System Layers

```
┌─────────────────────────────────────────────────────────────┐
│  jarvis-gui  (Svelte + Tauri)                               │
│  UI layer — communicates only via IpcEvent / Tauri invoke   │
└────────────────────┬────────────────────────────────────────┘
                     │ Tauri invoke / WebSocket IpcEvent
┌────────────────────▼────────────────────────────────────────┐
│  jarvis-app  (main.rs + app.rs + fast_path.rs)             │
│  Entry point — owns the Fast Path pipeline thread           │
└────────────────────┬────────────────────────────────────────┘
                     │ direct crate import (jarvis_core::*)
┌────────────────────▼────────────────────────────────────────┐
│  jarvis-core                                                │
│  Core library — event_bus, stt, intent, audio, commands,   │
│  scripts, models, config, plugin, agent_registry            │
└─────────────────────────────────────────────────────────────┘
```

`jarvis-gui` talks to `jarvis-app` via:
- **Tauri `invoke`** commands (request/response)
- **WebSocket IpcEvent** stream (server-push events)

`jarvis-app` is a thin orchestrator; all domain logic lives in `jarvis-core`.

---

## Fast Path Pipeline

The Fast Path is the latency-critical voice pipeline. It runs on a dedicated thread. **Hard constraints apply** (see `.claude/rules/fast-path.md`):

- NO async LLM API calls — ever
- NO blocking HTTP / network I/O
- All stages complete in <250ms P50

### Pipeline stages (`crates/jarvis-app/src/fast_path.rs` + `app.rs`)

```
Microphone → [1] Wake Word → [2] STT → [3] Intent → [4] Executor → [5] Audio Feedback
```

| # | Stage | Module | Output |
|---|---|---|---|
| 1 | Wake word detection | `listener::vosk` | Wakes up VAD loop |
| 2 | Speech-to-Text | `stt::vosk` | Raw transcript string |
| 3 | Intent classification | `intent::EmbeddingClassifier` | Command ID + utterance |
| 4 | Command execution | `commands` / `scripts` | Process spawn / Lua script |
| 5 | Audio feedback | `audio::kira` | ok.wav / notfound.wav |

**VAD (Voice Activity Detection)** is built into the pipeline. `AudioRingBuffer` keeps 5 seconds of pre-roll audio so that speech starting before wake-word silence is not lost.

### Event emission

Each stage publishes to the Event Bus for observability and Slow Path integration:

```
WakeWordDetected → Listening → SpeechRecognized → CommandRecognized → CommandExecuted → StateChanged
```

---

## Event Bus (`jarvis_core::event_bus`)

The Event Bus is the **only** communication channel between modules (ADR-001).

```rust
// Initialize once at startup
let _tx = event_bus::init();

// Publish from any module
event_bus::publish(JarvisEvent::CommandExecuted { id: "open_browser".into(), success: true });

// Subscribe from any module (new feature = new subscriber, no existing code changes)
let mut rx = event_bus::subscribe();
tokio::spawn(async move {
    while let Ok(event) = rx.recv().await {
        // handle event
    }
});
```

**Implementation:** `tokio::sync::broadcast` channel, capacity 64. Backed by a `OnceCell<broadcast::Sender>` — idempotent `init()`, safe to call multiple times.

### JarvisEvent variants

| Event | When |
|---|---|
| `WakeWordDetected` | Wake word matched |
| `Listening` | Active recording started |
| `SpeechRecognized { text }` | STT produced a transcript |
| `CommandRecognized { id, utterance }` | Intent matched a command |
| `CommandExecuted { id, success }` | Command finished |
| `StateChanged { state }` | Assistant state changed |
| `Error { message }` | Any pipeline stage error |

**Rule:** `IpcEvent` (WebSocket) is separate from `JarvisEvent` (internal). Do not conflate them.

---

## Command Registry (`jarvis_core::command_registry`)

Thread-safe, globally shared command list backed by `parking_lot::RwLock` (ADR-002).

```rust
// Load (atomic full replacement — only write operation)
command_registry::load(commands);

// Read (shared guard, zero-copy)
let guard = command_registry::read();

// Snapshot (owned Vec, for background threads)
let snapshot = command_registry::get_snapshot();
```

**Write model:** The entire `Vec<JCommandsList>` is replaced atomically. There are no partial mutations. This prevents read-modify-write races.

**Hot-reload** (ADR-005): `IpcAction::ReloadCommands` from the GUI triggers `command_registry::load()` + `intent::reinit()`. The EmbeddingClassifier supports hot-swap via `RwLock` — no process restart required.

---

## Data Storage (ADR-010)

**Rule: NEVER write user data to `resources/` or `target/`.**

| Data | Location | Notes |
|---|---|---|
| Bundled defaults | `resources/commands/`, `resources/scripts/` | Git-tracked, read-only, copied into build |
| User commands | `config::user_commands_dir()` = `APP_CONFIG_DIR/commands/` | Persistent, survives rebuilds |
| User scripts | `config::user_scripts_dir()` = `APP_CONFIG_DIR/scripts/` | Persistent, survives rebuilds |
| Settings | `APP_CONFIG_DIR/app.db` | SQLite via `db::manager` |

`APP_CONFIG_DIR` resolves to `%APPDATA%\com.priler.jarvis\` on Windows.

**Merge strategy:** At startup `parse_commands_from_dirs()` merges bundled + user packs (user wins on conflict). First-run seeding copies bundled defaults to user dir if user dir is empty.

All Tauri CRUD commands (`create_command_pack`, `save_script`, etc.) write **only** to the user layer.

---

## Intent Classification (`jarvis_core::intent`)

Classifies a transcript to a command ID using cosine similarity over sentence embeddings.

**Fallback chain (ADR-007, ADR-008):**
1. `EmbeddingClassifier` with the configured model (e.g., `paraphrase-multilingual-MiniLM-L12-v2`)
2. If the model is a Git LFS pointer (not downloaded) → fallback to `all-MiniLM-L6-v2` (English, always present as real binary)
3. If no embedding model is available → `backend = "none"`, routing falls back to regex / fuzzy matching

`intent::reinit()` hot-swaps the classifier without restarting — called after `ReloadCommands`.

---

## Model Catalog (`jarvis_core::models::catalog`)

Models live in `resources/models/<id>/model.toml`. The catalog is scanned at startup.

**LFS guard (ADR-007):** If `model.onnx` starts with `version https://git-lfs` (Git LFS pointer), the model is excluded from the catalog with `WARN`. It never reaches the intent classifier, preventing a runtime ONNX parse crash.

`list_available_models` Tauri command exposes the catalog to the GUI (`available: bool` per model).

### Active model stack

| Model | Task | Status |
|---|---|---|
| `vosk-model-small-ru-0.22` | STT (wake word + transcript) | Active |
| `all-MiniLM-L6-v2` (90 MB) | Intent embeddings (English) | Active |
| `paraphrase-multilingual-MiniLM-L12-v2` | Intent embeddings (multilingual) | Not downloaded (LFS pointer — excluded) |
| GLiNER | Slot extraction | Disabled |

---

## Plugin System (`jarvis_core::plugin`)

Every plugin **must** ship a `plugin.json` manifest (ADR-003). The manifest is validated at load time.

```json
{
  "id": "unique-plugin-id",
  "version": "1.0.0",
  "name": "Human Readable Name",
  "description": "What this plugin does",
  "author": "username",
  "commands": [],
  "agents": [],
  "capabilities": [],
  "permissions": {
    "filesystem": false,
    "network": false,
    "processes": false
  },
  "endpoint": null,
  "signature": null
}
```

`PluginPermissions` defaults to deny-all. Plugins must explicitly request capabilities.

---

## Agent Registry (`jarvis_core::agent_registry`)

Registry of automation agents that plugins can register. Agents are discovered from plugin manifests and made available to the Slow Path for LLM-driven automation workflows.

---

## Scripts (`jarvis_core::scripts`)

Scripts are Lua files executed by the `lua::engine`. They are loaded **live from disk** on each execution — no stale cache (ADR-004). Deleting a script file takes effect immediately.

Scripts are virtual commands: they appear in the intent classifier's command list and are dispatched via the same pipeline as native commands.

**Storage:** User scripts live in `user_scripts_dir()` and are merged with bundled scripts at startup.

---

## IPC Layer (`jarvis_core::ipc`)

Bidirectional communication between `jarvis-app` and `jarvis-gui`:

- **Server → Client:** `IpcEvent` pushed over WebSocket (port configurable)
- **Client → Server:** Tauri `invoke` commands

`IpcEvent` is separate from `JarvisEvent`. The IPC layer bridges internal events to the GUI — a Slow Path subscriber translates `JarvisEvent` → `IpcEvent`.

---

## Crate Map

```
jarvis/
├── crates/
│   ├── jarvis-core/        # Domain library — no Tauri dependency
│   │   ├── src/
│   │   │   ├── event_bus.rs        # tokio broadcast Event Bus
│   │   │   ├── command_registry.rs # atomic RwLock command store
│   │   │   ├── commands.rs         # TOML command pack loading
│   │   │   ├── scripts.rs          # Lua script loading + merge
│   │   │   ├── intent/             # EmbeddingClassifier
│   │   │   ├── models/             # catalog, loaders, LFS guard
│   │   │   ├── plugin/             # PluginManifest + validation
│   │   │   ├── agent_registry.rs   # agent registry
│   │   │   ├── stt/                # Vosk STT interface
│   │   │   ├── listener/           # wake-word backends
│   │   │   ├── audio/              # Kira / Rodio playback
│   │   │   ├── audio_processing/   # VAD, noise suppression, gain
│   │   │   ├── ipc/                # IpcEvent, WebSocket server
│   │   │   ├── config.rs           # app dirs, user data paths
│   │   │   ├── db/                 # SQLite settings
│   │   │   └── lua/                # Lua engine + sandbox
│   ├── jarvis-app/         # Tauri backend + Fast Path entry point
│   │   └── src/
│   │       ├── main.rs             # binary entry point
│   │       ├── app.rs              # main loop, wake-word detection
│   │       ├── fast_path.rs        # Fast Path pipeline (hard constraints)
│   │       └── tray.rs             # system tray
│   ├── jarvis-gui/         # Tauri commands (Rust side of GUI)
│   │   └── src/
│   │       └── tauri_commands/     # CRUD: commands, scripts, models, sys
│   └── jarvis-cli/         # CLI companion (diagnostics, dev tools)
├── frontend/               # Svelte + TypeScript UI
│   └── src/routes/         # commands, scripts, settings, models
├── resources/
│   ├── commands/           # bundled command packs (TOML)
│   ├── scripts/            # bundled Lua scripts
│   └── models/             # model.toml descriptors
└── docs/                   # this file + TASKS.md, MEMORY.md
```

---

## Performance Contracts

| Metric | Target | Benchmark |
|---|---|---|
| Wake-word detection | <50ms P50 | `cargo bench wake_word` |
| STT (short phrase) | <200ms P50 | `cargo bench stt` |
| Full Fast Path | <250ms P50, <400ms P95 | `cargo bench fast_path` |

Benchmarks are **mandatory** before merging any Fast Path changes. Use `cargo-criterion`.

---

## Architectural Decisions (summary)

| ADR | Decision |
|---|---|
| ADR-001 | Event Bus via `tokio::broadcast` — sole inter-module channel |
| ADR-002 | Fast Path: NO LLM calls — latency guarantee |
| ADR-003 | Plugin manifest (`plugin.json`) required for every plugin |
| ADR-004 | Scripts loaded live from disk — no stale cache |
| ADR-005 | Hot-reload via `IpcAction::ReloadCommands` + `intent::reinit()` |
| ADR-006 | Fast Path lives in `fast_path.rs` — auditable latency boundary |
| ADR-007 | Model catalog filters Git LFS pointers at scan time |
| ADR-008 | Intent classifier falls back to `all-MiniLM-L6-v2` if primary unavailable |
| ADR-009 | `slots_backend` normalized case-insensitively in `intent::init()` |
| ADR-010 | User data in `APP_CONFIG_DIR`, never in `resources/` or `target/` |

Full rationale for each ADR: see `docs/MEMORY.md`.
