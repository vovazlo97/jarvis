# User Data Persistence Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Separate bundled defaults (git, read-only) from user commands/scripts (app_data_dir, persistent across rebuilds).

**Architecture:** Two-layer data model — bundled defaults live in `target/debug/resources/` (build output, ephemeral), user data lives in `APP_CONFIG_DIR/commands/` and `APP_CONFIG_DIR/scripts/` (Windows: `%APPDATA%\com.priler.jarvis\`). `parse_commands()` and `parse_scripts()` in jarvis-core merge both layers (user overrides bundled by pack name / script id). GUI CRUD writes only to user layer. At first run, bundled defaults are seeded into user dir so the user can edit them.

**Tech Stack:** Rust, `once_cell::sync::OnceCell`, `std::fs`, `platform_dirs::AppDirs`, Tauri.

**⚠️ Why NOT `tauri::api::path::app_data_dir()`:**
`jarvis-core` is shared between `jarvis-app` (no Tauri) and `jarvis-gui` (Tauri).
Adding Tauri as a core dependency would break the standalone app.
`platform_dirs::AppDirs::new(Some("com.priler.jarvis"), false)` resolves to the **same path**
as Tauri's `app_data_dir()` with identifier `"com.priler.jarvis"` (verified from `tauri.conf.json`).
Both dev and release resolve to `%APPDATA%\com.priler.jarvis\` — the identifier is stable.

**Verified from codebase:**
- `tauri.conf.json` identifier: `"com.priler.jarvis"` ✅
- `config::BUNDLE_IDENTIFIER`: `"com.priler.jarvis"` ✅ (matches)
- Dev path = Release path = `%APPDATA%\com.priler.jarvis\` ✅ (platform_dirs is exe-path-independent)

**Root Cause (confirmed):**
- `APP_DIR` = `current_exe().parent()` = `target/debug/`
- All GUI CRUD writes to `APP_DIR/resources/commands/` = `target/debug/resources/commands/`
- `post_build.py --sync` deletes orphan files in target dir → user packs (not in git) are wiped
- Fix: user writes go to `APP_CONFIG_DIR/commands/` which is never touched by builds

---

### Task 1: Add `user_commands_dir()` / `user_scripts_dir()` to config.rs

**Files:**
- Modify: `crates/jarvis-core/src/config.rs`

**Context:** `APP_CONFIG_DIR` is a `OnceCell<PathBuf>` set by `config::init_dirs()` to
`AppDirs::new(Some("com.priler.jarvis"), false).config_dir`. On Windows:
`C:\Users\<user>\AppData\Roaming\com.priler.jarvis`. The `app.db` file already lives there.
We are adding two subdirectories under that same root.

**Step 1: Add the two public functions at the bottom of `config.rs` (before the `get_wake_phrases` function)**

```rust
/// Returns the directory where user-created command packs are stored.
/// Persistent across builds and updates.
/// Windows: %APPDATA%\com.priler.jarvis\commands\
pub fn user_commands_dir() -> std::path::PathBuf {
    crate::APP_CONFIG_DIR
        .get()
        .expect("config::init_dirs() must be called before user_commands_dir()")
        .join("commands")
}

/// Returns the directory where user-created scripts are stored.
/// Persistent across builds and updates.
/// Windows: %APPDATA%\com.priler.jarvis\scripts\
pub fn user_scripts_dir() -> std::path::PathBuf {
    crate::APP_CONFIG_DIR
        .get()
        .expect("config::init_dirs() must be called before user_scripts_dir()")
        .join("scripts")
}
```

**Step 2: Add startup log in `crates/jarvis-gui/src/main.rs`**

After `config::init_dirs().expect(...)`, add:

```rust
// Log user data paths so operators can verify persistence at a glance
let user_cmds = jarvis_core::config::user_commands_dir();
let user_scripts = jarvis_core::config::user_scripts_dir();
info!("User data — commands: {:?}", user_cmds);
info!("User data — scripts:  {:?}", user_scripts);
```

Expected output in dev:
```
[INFO] User data — commands: "C:\\Users\\<user>\\AppData\\Roaming\\com.priler.jarvis\\commands"
[INFO] User data — scripts:  "C:\\Users\\<user>\\AppData\\Roaming\\com.priler.jarvis\\scripts"
```
This MUST NOT contain `target/` — if it does, the init_dirs contract is broken.

**Step 3: Run `cargo build --package jarvis-core` to verify it compiles**

```
cargo build --package jarvis-core
```
Expected: `Finished` with no errors.

**Step 4: Commit**

```bash
git add crates/jarvis-core/src/config.rs crates/jarvis-gui/src/main.rs
git commit -m "feat(core): add user_commands_dir() and user_scripts_dir() to config; log at startup"
```

---

### Task 2: Merge bundled + user in `jarvis-core::commands::parse_commands()`

**Files:**
- Modify: `crates/jarvis-core/src/commands.rs`

**Context:** `parse_commands()` currently reads only `APP_DIR/resources/commands/`.
We need it to also read `user_commands_dir()`, merging results.
Merge rule: if user has a pack with the same folder name as bundled, user wins (bundled skipped).
To keep this testable without touching global statics, extract the merge logic into
`parse_commands_from_dirs(bundled: &Path, user: &Path)`.

**Step 1: Write the failing test (add at bottom of `commands.rs`, inside a `#[cfg(test)] mod tests` block)**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_command_toml(dir: &std::path::Path, id: &str) {
        let content = format!(
            "[[commands]]\nid = \"{}\"\ntype = \"cli\"\ncli_cmd = \"echo\"\nsounds.ru = [\"ok1\"]\n",
            id
        );
        fs::write(dir.join("command.toml"), content).unwrap();
    }

    /// User pack with same name as bundled must override bundled.
    #[test]
    fn user_pack_overrides_bundled_same_name() {
        let tmp = tempfile::tempdir().unwrap();
        let bundled = tmp.path().join("bundled");
        let user = tmp.path().join("user");

        // bundled: pack "games" with command "witcher"
        let b_games = bundled.join("games");
        fs::create_dir_all(&b_games).unwrap();
        write_command_toml(&b_games, "witcher");

        // user: pack "games" with command "cyberpunk" (overrides)
        let u_games = user.join("games");
        fs::create_dir_all(&u_games).unwrap();
        write_command_toml(&u_games, "cyberpunk");

        let result = parse_commands_from_dirs(&bundled, &user);
        let ids: Vec<_> = result.iter().flat_map(|l| l.commands.iter().map(|c| c.id.as_str())).collect();
        assert!(ids.contains(&"cyberpunk"), "user pack must override bundled");
        assert!(!ids.contains(&"witcher"), "bundled pack must be overridden by user");
    }

    /// Unique user pack (not in bundled) must appear.
    #[test]
    fn unique_user_pack_included() {
        let tmp = tempfile::tempdir().unwrap();
        let bundled = tmp.path().join("bundled");
        let user = tmp.path().join("user");
        fs::create_dir_all(&bundled).unwrap();

        let u_custom = user.join("my-custom");
        fs::create_dir_all(&u_custom).unwrap();
        write_command_toml(&u_custom, "my-cmd");

        // write at least one bundled pack so parse_commands doesn't error
        let b_default = bundled.join("default");
        fs::create_dir_all(&b_default).unwrap();
        write_command_toml(&b_default, "default-cmd");

        let result = parse_commands_from_dirs(&bundled, &user);
        let ids: Vec<_> = result.iter().flat_map(|l| l.commands.iter().map(|c| c.id.as_str())).collect();
        assert!(ids.contains(&"my-cmd"), "user-only pack must be included");
        assert!(ids.contains(&"default-cmd"), "bundled-only pack must be included");
    }

    /// User dir not existing must not cause a panic — gracefully skip.
    #[test]
    fn missing_user_dir_is_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let bundled = tmp.path().join("bundled");
        let user = tmp.path().join("nonexistent_user");
        fs::create_dir_all(&bundled).unwrap();

        let b_pack = bundled.join("pack");
        fs::create_dir_all(&b_pack).unwrap();
        write_command_toml(&b_pack, "cmd");

        let result = parse_commands_from_dirs(&bundled, &user);
        assert_eq!(result.len(), 1);
    }
}
```

**Step 2: Run test to confirm it FAILS (function doesn't exist yet)**

```
cargo test --package jarvis-core user_pack_overrides_bundled_same_name
```
Expected: FAIL — `error[E0425]: cannot find function 'parse_commands_from_dirs'`

**Step 3: Add `parse_commands_from_dirs()` and update `parse_commands()`**

Find the existing `parse_commands()` function in `crates/jarvis-core/src/commands.rs` and replace it with:

```rust
/// Core merge logic — reads packs from two directories.
/// User packs override bundled packs with the same folder name.
/// Missing directories are silently skipped.
pub fn parse_commands_from_dirs(bundled_dir: &Path, user_dir: &Path) -> Vec<JCommandsList> {
    let mut packs: std::collections::HashMap<String, JCommandsList> =
        std::collections::HashMap::new();

    // Load bundled packs first (lower priority)
    if let Ok(entries) = fs::read_dir(bundled_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let toml_file = path.join("command.toml");
            if !toml_file.exists() {
                continue;
            }
            let pack_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            match load_pack(&path, &toml_file) {
                Some(list) => { packs.insert(pack_name, list); }
                None => {}
            }
        }
    }

    // Load user packs second — overrides bundled packs with same name
    if user_dir.exists() {
        if let Ok(entries) = fs::read_dir(user_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let toml_file = path.join("command.toml");
                if !toml_file.exists() {
                    continue;
                }
                let pack_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                match load_pack(&path, &toml_file) {
                    Some(list) => { packs.insert(pack_name, list); }
                    None => {}
                }
            }
        }
    }

    packs.into_values().collect()
}

fn load_pack(pack_path: &Path, toml_file: &Path) -> Option<JCommandsList> {
    let content = match fs::read_to_string(toml_file) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read {}: {}", toml_file.display(), e);
            return None;
        }
    };
    match toml::from_str::<JCommandsList>(&content) {
        Ok(file) => Some(JCommandsList {
            path: pack_path.to_path_buf(),
            commands: file.commands,
        }),
        Err(e) => {
            warn!("Failed to parse {}: {}", toml_file.display(), e);
            None
        }
    }
}

pub fn parse_commands() -> Result<Vec<JCommandsList>, String> {
    let bundled_dir = APP_DIR.join(config::COMMANDS_PATH);
    let user_dir = config::user_commands_dir();

    let commands = parse_commands_from_dirs(&bundled_dir, &user_dir);

    if commands.is_empty() {
        Err("No commands found".into())
    } else {
        info!("Loaded {} command pack(s)", commands.len());
        Ok(commands)
    }
}
```

**Step 4: Run tests to confirm GREEN**

```
cargo test --package jarvis-core
```
Expected: All tests pass including the 3 new ones.

**Step 5: Commit**

```bash
git add crates/jarvis-core/src/commands.rs
git commit -m "feat(core): merge bundled + user command packs in parse_commands()"
```

---

### Task 3: Merge bundled + user in `jarvis-core::scripts::parse_scripts()`

**Files:**
- Modify: `crates/jarvis-core/src/scripts.rs`

**Context:** Same pattern as Task 2. `scripts_dir()` → `APP_DIR/resources/scripts`.
Need to also read `user_scripts_dir()`. Scripts are identified by `id` field (not folder name).
Merge rule: if user has a script with same `id` as bundled, user wins.
Extract `parse_scripts_from_dirs(bundled: &Path, user: &Path)` for testability.

**Step 1: Write the failing test (add at bottom of `scripts.rs`)**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_script_toml(dir: &std::path::Path, id: &str, name: &str) {
        let content = format!(
            "id = \"{}\"\nname = \"{}\"\nmode = \"sequential\"\n",
            id, name
        );
        fs::write(dir.join(format!("{}.toml", id)), content).unwrap();
    }

    /// User script with same id as bundled must override bundled.
    #[test]
    fn user_script_overrides_bundled_same_id() {
        let tmp = tempfile::tempdir().unwrap();
        let bundled = tmp.path().join("bundled");
        let user = tmp.path().join("user");
        fs::create_dir_all(&bundled).unwrap();
        fs::create_dir_all(&user).unwrap();

        write_script_toml(&bundled, "morning-routine", "Morning Routine (default)");
        write_script_toml(&user, "morning-routine", "Morning Routine (user)");

        let result = parse_scripts_from_dirs(&bundled, &user);
        let script = result.iter().find(|s| s.id == "morning-routine").unwrap();
        assert_eq!(script.name, "Morning Routine (user)");
    }

    /// Unique user script (not in bundled) must appear.
    #[test]
    fn unique_user_script_included() {
        let tmp = tempfile::tempdir().unwrap();
        let bundled = tmp.path().join("bundled");
        let user = tmp.path().join("user");
        fs::create_dir_all(&bundled).unwrap();
        fs::create_dir_all(&user).unwrap();

        write_script_toml(&bundled, "default-script", "Default");
        write_script_toml(&user, "my-script", "My Script");

        let result = parse_scripts_from_dirs(&bundled, &user);
        let ids: Vec<_> = result.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&"my-script"));
        assert!(ids.contains(&"default-script"));
    }

    /// Missing user dir is gracefully skipped.
    #[test]
    fn missing_user_scripts_dir_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let bundled = tmp.path().join("bundled");
        let user = tmp.path().join("nonexistent");
        fs::create_dir_all(&bundled).unwrap();
        write_script_toml(&bundled, "s1", "Script 1");

        let result = parse_scripts_from_dirs(&bundled, &user);
        assert_eq!(result.len(), 1);
    }
}
```

**Step 2: Run test to confirm FAIL**

```
cargo test --package jarvis-core user_script_overrides_bundled_same_id
```
Expected: FAIL — `error[E0425]: cannot find function 'parse_scripts_from_dirs'`

**Step 3: Add `parse_scripts_from_dirs()` and update `parse_scripts()` and `scripts_dir()`**

Find `parse_scripts()` and `scripts_dir()` in `crates/jarvis-core/src/scripts.rs` and replace:

```rust
/// Core merge logic — reads scripts from two directories.
/// User scripts override bundled scripts with the same `id`.
/// Missing directories are silently skipped.
pub fn parse_scripts_from_dirs(bundled_dir: &std::path::Path, user_dir: &std::path::Path) -> Vec<Script> {
    let mut scripts: std::collections::HashMap<String, Script> =
        std::collections::HashMap::new();

    // Bundled first (lower priority)
    load_scripts_from_dir(bundled_dir, &mut scripts);

    // User second (overrides bundled)
    if user_dir.exists() {
        load_scripts_from_dir(user_dir, &mut scripts);
    }

    let mut result: Vec<Script> = scripts.into_values().collect();
    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}

fn load_scripts_from_dir(dir: &std::path::Path, out: &mut std::collections::HashMap<String, Script>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<Script>(&content) {
                Ok(script) => { out.insert(script.id.clone(), script); }
                Err(e) => warn!("Failed to parse script {}: {}", path.display(), e),
            },
            Err(e) => warn!("Failed to read script {}: {}", path.display(), e),
        }
    }
}

pub fn parse_scripts() -> Vec<Script> {
    let bundled = APP_DIR.join("resources/scripts");
    let user = config::user_scripts_dir();
    info!("[DEBUG_FIX] parse_scripts() scanning bundled: {:?}, user: {:?}", bundled, user);
    parse_scripts_from_dirs(&bundled, &user)
}

fn scripts_dir() -> PathBuf {
    config::user_scripts_dir()
}
```

Note: `scripts_dir()` is now `user_scripts_dir()` — used by `fetch_script_live()` which only reads the live user script by path. This is correct: live-read follows the same hierarchy as parse.

**Step 4: Run tests to confirm GREEN**

```
cargo test --package jarvis-core
```
Expected: All pass (64 total now).

**Step 5: Commit**

```bash
git add crates/jarvis-core/src/scripts.rs
git commit -m "feat(core): merge bundled + user scripts in parse_scripts()"
```

---

### Task 4: GUI commands CRUD → user dir + first-run seeding

**Files:**
- Modify: `crates/jarvis-gui/src/tauri_commands/commands.rs`

**Context:** Every write operation currently uses `APP_DIR.join(config::COMMANDS_PATH)`.
Replace with `config::user_commands_dir()`. Also add `seed_user_commands()` which copies
bundled packs to user dir on first run (if user dir is empty). `list_command_packs()` reads
from user dir only (after seeding it contains copies of defaults; user can edit them there).

**Step 1: Update all path references in `commands.rs`**

Replace the helper `pack_toml_path()` function:

```rust
fn pack_toml_path(pack_name: &str) -> PathBuf {
    jarvis_core::config::user_commands_dir()
        .join(pack_name)
        .join("command.toml")
}
```

Replace `list_command_packs()`:

```rust
#[tauri::command]
pub fn list_command_packs() -> Vec<CommandPackInfo> {
    let commands_path = jarvis_core::config::user_commands_dir();
    let mut packs = Vec::new();

    let entries = match fs::read_dir(&commands_path) {
        Ok(e) => e,
        Err(_) => return packs,
    };

    for entry in entries.flatten() {
        let pack_path = entry.path();
        let toml_file = pack_path.join("command.toml");
        if !toml_file.exists() {
            continue;
        }
        let content = match fs::read_to_string(&toml_file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let file: JCommandsList = match toml::from_str(&content) {
            Ok(f) => f,
            Err(_) => continue,
        };
        packs.push(CommandPackInfo {
            pack_name: entry.file_name().to_string_lossy().into_owned(),
            commands: file.commands,
        });
    }

    packs
}
```

Replace `create_command_pack()`:

```rust
#[tauri::command]
pub fn create_command_pack(pack_name: String, command: NewCommandInput) -> Result<(), String> {
    let safe = sanitize_pack_name(&pack_name)?;
    let pack_path = jarvis_core::config::user_commands_dir().join(&safe);
    fs::create_dir_all(&pack_path)
        .map_err(|e| format!("Cannot create directory '{}': {}", safe, e))?;

    let toml_path = pack_path.join("command.toml");
    let cmd = input_to_jcommand(&command)?;
    save_commands(&[cmd], &toml_path)
}
```

Replace `delete_command()` — the pack removal path:

```rust
#[tauri::command]
pub fn delete_command(pack_name: String, command_id: String) -> Result<(), String> {
    validate_name(&pack_name)?;
    let toml_path = pack_toml_path(&pack_name);

    let content = fs::read_to_string(&toml_path).map_err(|e| format!("Cannot read pack: {}", e))?;
    let mut list: JCommandsList =
        toml::from_str(&content).map_err(|e| format!("Cannot parse pack: {}", e))?;

    let before = list.commands.len();
    list.commands.retain(|c| c.id != command_id);
    if list.commands.len() == before {
        return Err(format!("Command '{}' not found in '{}'", command_id, pack_name));
    }

    if list.commands.is_empty() {
        let pack_path = jarvis_core::config::user_commands_dir().join(&pack_name);
        let _ = fs::remove_dir_all(&pack_path);
        return Ok(());
    }

    save_commands(&list.commands, &toml_path)
}
```

Replace `delete_command_pack()`:

```rust
#[tauri::command]
pub fn delete_command_pack(pack_name: String) -> Result<(), String> {
    validate_name(&pack_name)?;
    let pack_path = jarvis_core::config::user_commands_dir().join(&pack_name);
    if !pack_path.exists() {
        return Err(format!("Pack '{}' not found", pack_name));
    }
    fs::remove_dir_all(&pack_path)
        .map_err(|e| format!("Cannot delete pack '{}': {}", pack_name, e))
}
```

Remove the `use jarvis_core::{config, APP_DIR};` import and replace with:
```rust
use jarvis_core::commands::{self, JCommand, JCommandsList};
```
(APP_DIR and config are no longer needed here)

**Step 2: Add `seed_user_commands()` function to the same file**

```rust
/// Copy all bundled command packs to the user commands directory.
/// Only runs if user commands dir is empty (first run).
/// This allows the user to edit default packs from the GUI.
pub fn seed_user_commands() {
    use jarvis_core::APP_DIR;
    use jarvis_core::config::{COMMANDS_PATH, user_commands_dir};

    let user_dir = user_commands_dir();
    if user_dir.exists() {
        // If user dir already has packs, skip seeding
        if fs::read_dir(&user_dir)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
        {
            return;
        }
    }

    let bundled_dir = APP_DIR.join(COMMANDS_PATH);
    if !bundled_dir.exists() {
        return;
    }

    let _ = fs::create_dir_all(&user_dir);

    let entries = match fs::read_dir(&bundled_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let src_pack = entry.path();
        let pack_name = entry.file_name();
        let dst_pack = user_dir.join(&pack_name);
        if src_pack.is_dir() && src_pack.join("command.toml").exists() {
            if let Err(e) = copy_dir_recursive(&src_pack, &dst_pack) {
                eprintln!("Seed: failed to copy pack {:?}: {}", pack_name, e);
            }
        }
    }

    info!("Seeded user commands directory from bundled defaults");
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let dst_path = dst.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path)?;
        } else {
            fs::copy(&entry.path(), &dst_path)?;
        }
    }
    Ok(())
}
```

**Step 3: Run `cargo build --package jarvis-gui` to verify it compiles**

```
cargo build --package jarvis-gui
```
Expected: `Finished` with no errors.

**Step 4: Commit**

```bash
git add crates/jarvis-gui/src/tauri_commands/commands.rs
git commit -m "feat(gui): commands CRUD writes to user_commands_dir(); add first-run seeding"
```

---

### Task 5: GUI scripts → user dir + startup wiring + CLAUDE.md invariant

**Files:**
- Modify: `crates/jarvis-gui/src/tauri_commands/scripts.rs`
- Modify: `crates/jarvis-gui/src/main.rs`
- Modify: `.claude/CLAUDE.md`

**Step 1: Update `scripts_dir()` in `crates/jarvis-gui/src/tauri_commands/scripts.rs`**

Replace the `scripts_dir()` function:

```rust
fn scripts_dir() -> PathBuf {
    jarvis_core::config::user_scripts_dir()
}
```

Also update `list_scripts()` — it should read from user dir only (bundled scripts merge happens
in jarvis-core for jarvis-app; GUI only manages user scripts):

No change needed — `list_scripts()` already calls `scripts_dir()`, which now returns `user_scripts_dir()`.

Remove the now-unused constant:
```rust
// DELETE this line:
const SCRIPTS_DIR: &str = "resources/scripts";
```

Remove the now-unused imports if any (`APP_DIR`, `config`) from `scripts.rs`.

**Step 2: Run `cargo build --package jarvis-gui` to verify**

```
cargo build --package jarvis-gui
```
Expected: Finished, no errors.

**Step 3: Call `seed_user_commands()` in jarvis-gui startup**

In `crates/jarvis-gui/src/main.rs`, after `config::init_dirs()`:

```rust
fn main() {
    config::init_dirs().expect("Failed to init dirs");

    // Seed user data directories on first run
    tauri_commands::seed_user_commands();

    // basic logging setup (simpler for GUI)
    simple_log::quick!("info");
    // ...rest of main unchanged
```

Also add the public re-export if `seed_user_commands` is not already visible. In
`crates/jarvis-gui/src/tauri_commands/mod.rs` (or wherever commands module is exported),
ensure `seed_user_commands` is accessible as `tauri_commands::seed_user_commands()`.

**Step 4: Add invariant to `.claude/CLAUDE.md` Section 10**

In `.claude/CLAUDE.md`, find Section 10 (Architecture Map) and add:

```markdown
### Data Storage Invariant

**NEVER write user data to `resources/` or `target/`.**

| Data Type | Storage | Why |
|---|---|---|
| Bundled defaults (commands, scripts) | `resources/commands/`, `resources/scripts/` (git, read-only) | Copied to `target/` at build — ephemeral |
| User commands | `config::user_commands_dir()` = `APP_CONFIG_DIR/commands/` | Persistent across rebuilds |
| User scripts | `config::user_scripts_dir()` = `APP_CONFIG_DIR/scripts/` | Persistent across rebuilds |
| Settings (app.db) | `APP_CONFIG_DIR/app.db` | Already correct |

**Rule:** All Tauri CRUD commands (create_command_pack, save_script, etc.) MUST write to
`user_commands_dir()` / `user_scripts_dir()`. Violation = data loss on rebuild.
```

**Step 5: Run full test suite**

```
cargo test --workspace
```
Expected: All tests pass.

**Step 6: Manual verification — the key test (verification-before-completion)**

```bash
# 1. Build
cargo build --package jarvis-gui

# 2. Run jarvis-gui and check startup logs — MUST see:
#    [INFO] User data — commands: "C:\Users\...\AppData\Roaming\com.priler.jarvis\commands"
#    Path MUST NOT contain "target/" — fail immediately if it does.
target\debug\jarvis-gui.exe 2>&1 | grep "User data"

# 3. Open GUI → Commands → Create new pack "test-persistence" with one command "test-cmd"

# 4. Verify pack written to APP_CONFIG_DIR (NOT target/)
ls "%APPDATA%\com.priler.jarvis\commands\test-persistence\command.toml"
# Expected: file exists ✅

# 5. Rebuild (simulates next dev session)
cargo build --package jarvis-gui

# 6. Verify pack survived rebuild — this is the REGRESSION TEST
ls "%APPDATA%\com.priler.jarvis\commands\test-persistence\command.toml"
# Expected: file still exists ✅  (before fix: would be gone)

# 7. Run GUI again — pack must appear in Commands list
# Expected: "test-persistence" pack visible in GUI ✅
```

**Only claim success when ALL 7 steps produce expected output.**

**Step 7: Commit**

```bash
git add crates/jarvis-gui/src/tauri_commands/scripts.rs
git add crates/jarvis-gui/src/main.rs
git add .claude/CLAUDE.md
git commit -m "feat(gui): scripts → user_scripts_dir(); wire seeding in startup; add data invariant to CLAUDE.md"
```

---

## Review Checklist (before Phase A close)

After all 5 tasks:

- [ ] `cargo test --workspace` — all green
- [ ] `cargo build --package jarvis-gui` — no errors
- [ ] User commands dir: `%APPDATA%\com.priler.jarvis\commands\` (Windows)
- [ ] User scripts dir: `%APPDATA%\com.priler.jarvis\scripts\` (Windows)
- [ ] First run: bundled packs seeded into user dir
- [ ] `cargo build` does NOT wipe user data
- [ ] jarvis-app `parse_commands()` still returns merged bundled + user
- [ ] GUI CRUD writes only to user dir

## Notes on the broader review items

These were requested alongside the regression fix. Quick status:

**Event Bus (#2):** modules still use direct calls in several places (intent/stt/audio are NOT on the bus). This is pre-existing architecture — not changed in Phase A. Document as known tech debt in MEMORY.md, address in Phase B.

**Model Registry (#3):** `list_available_models` works for all 5 task types (verified via test + startup logs). GUI dropdowns still hardcode some options on the frontend side — frontend audit is Phase B scope.

**Fast Path (#4):** The fallback chain in `intent.rs` runs ONCE at init, not in the hot path. `or_else()` is lazy — only evaluated if preferred model fails. No allocations in the hot path were added. No blocking calls introduced.
