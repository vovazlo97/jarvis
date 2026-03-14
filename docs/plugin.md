# Jarvis Plugin Guide

> How to create commands, scripts, and plugins for Jarvis.

---

## Overview

Jarvis supports three levels of extensibility:

| Level | What | Where |
|---|---|---|
| **Command pack** | Voice-triggered actions (launch apps, run CLI, etc.) | `APP_CONFIG_DIR/commands/<pack>.toml` |
| **Script** | Lua scripts with full `jarvis.*` API access | `APP_CONFIG_DIR/scripts/<name>.lua` |
| **Plugin** | Bundle of commands + agents + manifest | `plugins/<id>/plugin.json` |

User data lives in `APP_CONFIG_DIR` (`%APPDATA%\com.priler.jarvis\` on Windows) and persists across rebuilds.

---

## Command Packs

A command pack is a TOML file containing one or more voice commands.

### Minimal example

```toml
[[commands]]
id = "open_browser"
type = "cli"
description = "Open browser"

[commands.phrases]
en = ["open browser", "launch browser", "browser"]
ru = ["открой браузер", "запусти браузер"]

[commands.sounds]
en = ["voices/pack/en/ok1.wav"]
```

### Command types

| `type` | Description | Required fields |
|---|---|---|
| `cli` | Run a system command | `cli_cmd`, optionally `cli_args` |
| `lua` | Execute a Lua script | `script` (filename in scripts dir), optionally `sandbox` |
| `ahk` | Launch an executable | `exe_path`, optionally `exe_args` |
| `voice` | Play a sound, no action | `sounds` |
| `terminate` | Stop current pipeline | — |
| `stop_chaining` | End a chaining session | — |

### All fields

```toml
[[commands]]
id          = "unique-command-id"    # required, must be globally unique
type        = "cli"                   # required, see table above
description = "Human description"    # optional

# CLI command
cli_cmd  = "notepad.exe"
cli_args = []

# AHK/executable
exe_path = "C:/tools/myapp.exe"
exe_args = ["--flag"]

# Lua script
script  = "my_script"               # looks for scripts/my_script.lua
sandbox = "standard"                 # "minimal" | "standard" | "full"
timeout = 10000                      # script timeout in ms (default 10000)

# Phrases for intent matching (per language)
[commands.phrases]
en = ["open notes", "launch notepad"]
ru = ["открой блокнот"]

# Regex patterns (take priority over phrase matching)
patterns = ["open (.+)", "launch (.+)"]

# Response sounds (per language)
[commands.sounds]
en = ["voices/pack/en/ok1.wav"]

# Custom response sound (overrides default ok.wav)
response_sound = "voices/pack/en/custom.wav"

# Stay in LISTENING mode after execution (command chaining)
allow_chaining = false

# Slot extraction (for named entity capture)
[commands.slots.city]
entity  = "city name"
context = ["in", "for", "to"]
```

### Phrase matching

Commands are matched by:
1. **Regex patterns** (`patterns`) — exact regex, highest priority
2. **Embedding similarity** — cosine similarity over sentence embeddings (EmbeddingClassifier)
3. **Fuzzy matching** — fallback when no embedding model is available

**Tip:** Add more phrase variants to improve recognition accuracy, especially for short commands.

---

## Lua Scripts

Scripts are Lua files executed by the Jarvis Lua engine (mlua). They run in a sandboxed environment and have access to the `jarvis.*` API.

### File location

Place scripts in `APP_CONFIG_DIR/scripts/`. They are merged with bundled scripts at startup.

### Minimal script

```lua
-- my_script.lua
jarvis.log("info", "Hello from my script!")
jarvis.system.open("https://example.com")
```

### Trigger a script via command

```toml
[[commands]]
id      = "open_example"
type    = "lua"
script  = "my_script"      # filename without .lua
sandbox = "standard"
timeout = 5000

[commands.phrases]
en = ["open example", "go to example"]
```

---

## Lua API Reference

All APIs are available as `jarvis.<module>.<function>`. The available modules depend on the sandbox level.

### Sandbox levels

| Level | Available APIs |
|---|---|
| `minimal` | `jarvis.log`, `jarvis.print`, `jarvis.sleep`, `jarvis.speak`, `jarvis.audio.*`, `jarvis.context.*` |
| `standard` (default) | minimal + `jarvis.http.*`, `jarvis.state.*`, `jarvis.fs.*` (command folder only) |
| `full` | standard + `jarvis.system.exec`, expanded `jarvis.fs.*`, clipboard write |

### `jarvis` (core) — always available

```lua
jarvis.log(level, message)   -- level: "debug"|"info"|"warn"|"error"
jarvis.print(...)            -- prints to log
jarvis.sleep(ms)             -- block for N milliseconds
jarvis.speak(text)           -- TTS (placeholder, logs for now)
```

### `jarvis.audio` — always available

```lua
jarvis.audio.play(path)      -- play a .wav file (relative to SOUND_DIR)
jarvis.audio.stop()          -- stop current audio playback
```

### `jarvis.context` — always available

```lua
local lang = jarvis.context.lang()       -- current UI language ("en", "ru", ...)
local slots = jarvis.context.slots()     -- extracted slots table {name -> value}
```

### `jarvis.http` — sandbox: standard, full

```lua
local resp = jarvis.http.get(url, headers?)
-- resp = { status = 200, body = "..." }

local resp = jarvis.http.post(url, body?, headers?)

local resp = jarvis.http.post_json(url, data_table, headers?)
-- data_table is a Lua table, auto-serialized to JSON
```

### `jarvis.state` — sandbox: standard, full

```lua
jarvis.state.set(key, value)  -- persist a value (string)
local v = jarvis.state.get(key)         -- retrieve it (nil if not set)
```

### `jarvis.fs` — sandbox: standard, full

Standard sandbox: access limited to command folder only.
Full sandbox: access to any path.

```lua
local content = jarvis.fs.read(path)
jarvis.fs.write(path, content)
local exists = jarvis.fs.exists(path)
jarvis.fs.delete(path)
local files = jarvis.fs.list(dir)        -- returns table of filenames
```

### `jarvis.system` — sandbox levels vary

```lua
-- Always available (all sandbox levels):
jarvis.system.open(url_or_path)    -- open URL or file with default app

-- sandbox: full only:
local result = jarvis.system.exec(cmd, args?)
-- result = { stdout = "...", stderr = "...", code = 0 }

jarvis.system.clipboard_write(text)  -- write to clipboard
```

---

## Plugin Manifest

A plugin bundles commands, scripts, and agents with a `plugin.json` manifest.

### Directory structure

```
plugins/
└── my-plugin/
    ├── plugin.json
    ├── commands/
    │   └── commands.toml
    └── scripts/
        └── helper.lua
```

### plugin.json schema

```json
{
  "id": "my-plugin",
  "version": "1.0.0",
  "name": "My Plugin",
  "description": "What this plugin does",
  "author": "your-username",
  "commands": ["commands/commands.toml"],
  "agents": ["my-agent"],
  "capabilities": ["automation", "web"],
  "permissions": {
    "filesystem": false,
    "network": false,
    "processes": false
  },
  "endpoint": null,
  "signature": null
}
```

### Field reference

| Field | Required | Description |
|---|---|---|
| `id` | ✅ | Unique plugin identifier (kebab-case) |
| `version` | ✅ | SemVer string |
| `name` | ✅ | Human-readable display name |
| `description` | ✅ | Short description |
| `author` | ✅ | Author username |
| `commands` | — | List of command pack file paths (relative to plugin dir) |
| `agents` | — | List of agent IDs this plugin registers |
| `capabilities` | — | Capability tags (e.g. `["web", "automation"]`) |
| `permissions.filesystem` | — | Requests filesystem access (default: false) |
| `permissions.network` | — | Requests network access (default: false) |
| `permissions.processes` | — | Requests process spawning (default: false) |
| `endpoint` | — | HTTP endpoint for remote agent (null = local only) |
| `signature` | — | Reserved for future manifest signing |

**Validation:** `id`, `version`, and `name` must be non-empty. Invalid manifests are skipped at load with a `WARN` log — they do not crash the runtime.

---

## Agent Registry

Plugins can register automation agents that the Slow Path can dispatch work to.

```rust
// Rust-side: register an agent at plugin load
agent_registry::register(AgentEntry {
    id: "my-agent".to_string(),
    name: "My Agent".to_string(),
    capabilities: vec!["web".to_string(), "automation".to_string()],
    plugin_id: "my-plugin".to_string(),
})?;

// Look up
let agent = agent_registry::get("my-agent");

// Unregister all agents for a plugin (called on plugin unload)
agent_registry::unregister_plugin("my-plugin");
```

The agent registry is a global `RwLock<HashMap<String, AgentEntry>>`. Registering a duplicate ID returns an error (no silent override).

---

## Slots (Named Entity Extraction)

Commands can define slots to extract structured data from the user's utterance.

```toml
[[commands]]
id   = "play_song"
type = "cli"

[commands.phrases]
en = ["play", "play song", "play music"]

[commands.slots.song_title]
entity  = "song title"       # semantic label for GLiNER
context = ["play", "put on"]  # fallback context words
```

The extracted slot value is available in Lua scripts via `jarvis.context.slots()`:

```lua
local slots = jarvis.context.slots()
local title = slots["song_title"]   -- e.g. "Bohemian Rhapsody"
```

**Note:** Slot extraction requires the GLiNER model. If GLiNER is disabled, slot values will be nil.

---

## Hot-Reload

Command packs and scripts support hot-reload without restarting Jarvis.

From the GUI: **Settings → Reload Commands** triggers `IpcAction::ReloadCommands`, which:
1. Re-scans `user_commands_dir()` and `resources/commands/`
2. Merges and reloads the Command Registry (atomic replacement)
3. Calls `intent::reinit()` to hot-swap the EmbeddingClassifier with the new command set

Scripts are loaded live from disk on each execution — no reload needed for script changes.

---

## Bundled vs User Data

| Source | Location | Editable | Persists rebuild |
|---|---|---|---|
| Bundled defaults | `resources/commands/`, `resources/scripts/` | Read-only | No (ephemeral) |
| User data | `APP_CONFIG_DIR/commands/`, `APP_CONFIG_DIR/scripts/` | Read/Write | Yes |

On first run, bundled defaults are seeded into the user directory. User edits always take precedence over bundled files with the same name.
