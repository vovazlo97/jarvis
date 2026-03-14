# Jarvis SDK

> Integration reference for external tools, GUI extensions, and Rust crate consumers.

---

## Overview

Jarvis exposes three integration surfaces:

| Surface | Who uses it | Transport |
|---|---|---|
| **WebSocket IPC** | External tools, dashboards, automation scripts | `ws://127.0.0.1:9712` |
| **Tauri invoke API** | Frontend (Svelte) and GUI extensions | Tauri `invoke()` |
| **jarvis-core Rust crate** | Internal crates, custom Rust extensions | Crate import |

---

## WebSocket IPC Protocol

`jarvis-app` listens on `ws://127.0.0.1:9712`. Both directions use JSON with a discriminant tag field.

### Connecting

```javascript
const ws = new WebSocket("ws://127.0.0.1:9712");

ws.onmessage = (e) => {
  const event = JSON.parse(e.data);
  console.log(event.event, event); // event.event = discriminant
};
```

### Events: jarvis-app → client (`IpcEvent`)

All events have an `"event"` tag field with a snake_case discriminant.

| `event` | Payload fields | Description |
|---|---|---|
| `"wake_word_detected"` | — | Wake word matched, pipeline starts |
| `"listening"` | — | Actively recording command |
| `"speech_recognized"` | `text: string` | STT produced a transcript |
| `"command_executed"` | `id: string`, `success: bool` | Command dispatch finished |
| `"state_changed"` | `state: AssistantState` | High-level state transition |
| `"idle"` | — | Returned to wake-word mode |
| `"started"` | — | App startup complete |
| `"stopping"` | — | App shutting down |
| `"pong"` | — | Response to a `ping` action |
| `"reveal_window"` | — | GUI should focus/reveal itself |
| `"error"` | `message: string` | Pipeline error |

**`AssistantState`** values: `"idle"` → `"activated"` → `"listening"` → `"processing"` → `"responding"` → `"idle"`

```json
{ "event": "state_changed", "state": "listening" }
{ "event": "speech_recognized", "text": "open browser" }
{ "event": "command_executed", "id": "open_browser", "success": true }
```

### Actions: client → jarvis-app (`IpcAction`)

All actions have an `"action"` tag field.

| `action` | Payload fields | Description |
|---|---|---|
| `"ping"` | — | Health check (triggers `"pong"` response) |
| `"stop"` | — | Graceful shutdown |
| `"reload_commands"` | — | Re-scan and reload commands + intent |
| `"set_muted"` | `muted: bool` | Mute/unmute wake-word listening |
| `"text_command"` | `text: string` | Inject a text command (bypasses STT) |

```json
{ "action": "ping" }
{ "action": "text_command", "text": "open browser" }
{ "action": "set_muted", "muted": true }
{ "action": "reload_commands" }
```

### Python example

```python
import asyncio, json, websockets

async def main():
    async with websockets.connect("ws://127.0.0.1:9712") as ws:
        # Send a text command
        await ws.send(json.dumps({"action": "text_command", "text": "open browser"}))
        # Listen for response
        msg = await ws.recv()
        print(json.loads(msg))

asyncio.run(main())
```

### JavaScript / Node example

```javascript
import WebSocket from "ws";

const ws = new WebSocket("ws://127.0.0.1:9712");

ws.on("open", () => {
  ws.send(JSON.stringify({ action: "ping" }));
});

ws.on("message", (data) => {
  const event = JSON.parse(data.toString());
  if (event.event === "pong") console.log("Jarvis is alive");
  if (event.event === "command_executed") {
    console.log(`Command '${event.id}' success=${event.success}`);
  }
});
```

---

## Tauri Invoke API

The Svelte frontend communicates with `jarvis-gui` via Tauri's `invoke()`. These are also available to any Tauri-based GUI extension.

```typescript
import { invoke } from "@tauri-apps/api/tauri";
```

### App info

```typescript
const version: string    = await invoke("get_app_version");
const author: string     = await invoke("get_author_name");
const repoUrl: string    = await invoke("get_repository_link");
const logPath: string    = await invoke("get_log_file_path");
```

### System stats

```typescript
interface JarvisAppStats { running: boolean; ram_mb: number; cpu_usage: number; }

const stats: JarvisAppStats = await invoke("get_jarvis_app_stats");
const running: boolean      = await invoke("is_jarvis_app_running");
const ramMb: number         = await invoke("get_current_ram_usage");
const cpuUsage: number      = await invoke("get_cpu_usage");
const cpuTemp: string       = await invoke("get_cpu_temp");       // "62.3" or "N/A"
const peakRam: string       = await invoke("get_peak_ram_usage"); // in GB
```

### Process control

```typescript
await invoke("run_jarvis_app");       // spawn jarvis-app (no-op if already running)
await invoke("reload_jarvis_commands"); // send ReloadCommands via WebSocket
```

### Commands CRUD

```typescript
interface JCommand { id: string; type: string; description: string; /* ... */ }
interface CommandPackInfo { pack_name: string; commands: JCommand[]; }

const packs: CommandPackInfo[] = await invoke("list_command_packs");
const all: JCommand[]          = await invoke("get_commands_list");
const count: number            = await invoke("get_commands_count");

// Create a new command pack
await invoke("create_command_pack", {
  packName: "my-pack",
  command: {
    id: "open_browser",
    type: "cli",
    description: "Open browser",
    phrases_en: ["open browser", "launch browser"],
    phrases_ru: ["открой браузер"],
    cli_cmd: "cmd",
    cli_args: ["/C", "start", "https://google.com"],
    exe_path: "", exe_args: [], patterns: [],
    sounds_ru: [], response_sound: ""
  }
});

// Update an existing command
await invoke("update_command", { packName: "my-pack", commandId: "open_browser", command: { /* ... */ } });

// Delete a command
await invoke("delete_command", { packName: "my-pack", commandId: "open_browser" });
```

### Scripts CRUD

```typescript
interface ScriptStep {
  step_type: "command_ref" | "delay" | "custom";
  label: string;
  // command_ref:
  pack?: string; command_id?: string;
  // delay:
  delay_ms?: number;
  // custom:
  cli_cmd?: string; cli_args?: string[];
}
interface Script {
  id: string; name: string; description: string;
  mode: "sequential" | "parallel";
  steps: ScriptStep[];
  phrases_ru: string[]; phrases_en: string[]; patterns: string[];
  sounds_ru: string[]; response_sound: string;
}

const scripts: Script[] = await invoke("list_scripts");

await invoke("save_script",  { script: Script });
await invoke("delete_script", { scriptId: "my-script-id" });
```

### Models

```typescript
interface BackendOption { id: string; name: string; available: boolean; }
interface VoskModel     { name: string; language: string; size: string; }
interface GlinerVariant { display_name: string; value: string; }

// task: "intent" | "slots" | "stt" | "vad" | "noise_suppression"
const options: BackendOption[] = await invoke("list_available_models", { task: "intent" });
const voskModels: VoskModel[]  = await invoke("list_vosk_models");
const glinerVariants: GlinerVariant[] = await invoke("list_gliner_models");
```

### Settings (DB)

```typescript
// Settings are read/written via the db commands (get_settings / save_settings).
// Schema defined in jarvis-core/src/db/structs.rs → Settings struct.
const settings = await invoke("get_settings");
await invoke("save_settings", { settings });
```

---

## Rust Crate Integration

`jarvis-core` is a library crate. Add it as a dependency to integrate directly.

```toml
# Cargo.toml
[dependencies]
jarvis-core = { path = "../jarvis-core" }
```

### Event Bus

Subscribe to pipeline events from any crate:

```rust
use jarvis_core::event_bus::{self, JarvisEvent};

// Initialize once at startup (idempotent)
let _tx = event_bus::init();

// Subscribe — receives all future events
let mut rx = event_bus::subscribe();
tokio::spawn(async move {
    while let Ok(event) = rx.recv().await {
        match event {
            JarvisEvent::CommandExecuted { id, success } => {
                println!("Command '{id}' success={success}");
            }
            JarvisEvent::StateChanged { state } => {
                println!("State → {state:?}");
            }
            _ => {}
        }
    }
});
```

**Rule:** Adding a feature = adding a new subscriber. Never modify existing pipeline code.

### Command Registry

```rust
use jarvis_core::{command_registry, commands};

// Load from disk (atomic replacement)
let packs = commands::parse_commands().unwrap_or_default();
command_registry::load(packs);

// Read (shared guard, zero-copy)
let guard = command_registry::read();
for pack in guard.iter() {
    for cmd in &pack.commands {
        println!("{}: {}", cmd.id, cmd.description);
    }
}

// Owned snapshot (for background threads)
let snapshot = command_registry::get_snapshot();
```

### Agent Registry

```rust
use jarvis_core::agent_registry::{self, AgentEntry};

// Register an agent (returns error if ID already exists)
agent_registry::register(AgentEntry {
    id: "my-agent".to_string(),
    name: "My Agent".to_string(),
    capabilities: vec!["web".to_string()],
    plugin_id: "my-plugin".to_string(),
})?;

// Look up
let agent = agent_registry::get("my-agent");

// List all (sorted by ID)
let all = agent_registry::list_all();

// Remove all agents from a plugin (call on plugin unload)
agent_registry::unregister_plugin("my-plugin");
```

### Plugin Manifest

```rust
use jarvis_core::plugin::manifest;

// Load and validate a single plugin.json
let manifest = manifest::load(Path::new("plugins/my-plugin/plugin.json"))?;

// Scan a directory (skips invalid manifests with WARN)
let plugins = manifest::scan_plugins_dir(Path::new("plugins/"));
for p in &plugins {
    println!("{} v{} by {}", p.name, p.version, p.author);
}
```

### JarvisEvent reference

```rust
pub enum JarvisEvent {
    WakeWordDetected,
    Listening,
    SpeechRecognized { text: String },
    CommandRecognized { id: String, utterance: String },
    CommandExecuted   { id: String, success: bool },
    StateChanged      { state: AssistantState },
    Error             { message: String },
}

pub enum AssistantState {
    Idle, Activated, Listening, Processing, Responding,
}
```

---

## IPC Connection Notes

- Port: **9712** (hardcoded, `IPC_PORT` in `ipc::server`)
- Address: `127.0.0.1` (loopback only — no remote access by design)
- Protocol: WebSocket, JSON messages
- The GUI checks port availability before spawning `jarvis-app` (no duplicate process)
- `jarvis-app` must be running for the WebSocket to be available; use `is_jarvis_app_running()` to check

---

## Build & Dev

```bash
# Build everything
cargo build --workspace

# Run tests
cargo test --workspace

# Lint (must pass before commit)
cargo clippy -- -D warnings

# Format
cargo fmt --all

# Run the app (dev mode)
cargo tauri dev
```

**CLAUDE.md** governs what changes are allowed in each crate. See also `docs/architecture.md` for system design context and `docs/plugin.md` for plugin/command authoring.
