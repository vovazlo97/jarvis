# TASK-006: Hot-reload Commands & Scripts Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** After any GUI add/edit/delete of a command or script, the running Jarvis assistant picks up the change instantly — no restart needed.

**Architecture:** All plumbing already exists. `IpcAction::ReloadCommands` → WebSocket → `main.rs` handler reloads registry + retrains intent classifier. The only missing piece is (1) `commands/index.svelte` calling `reload_jarvis_commands` after CRUD, and (2) `main.rs` including scripts as virtual commands when retraining the intent classifier on reload.

**Tech Stack:** Svelte/TypeScript (frontend), Rust (jarvis-app, jarvis-core), Tauri `invoke()`.

---

## Prerequisite Reading

Before starting, read these files to build context:
- `crates/jarvis-app/src/main.rs:160-195` — `IpcAction::ReloadCommands` handler
- `crates/jarvis-gui/src/tauri_commands/sys.rs:126-144` — `reload_jarvis_commands` tauri cmd
- `frontend/src/routes/commands/index.svelte:106-139` — CRUD handlers (no reload calls)
- `frontend/src/routes/scripts/index.svelte:145-200` — reference: scripts already calls reload

---

## Task 1: Frontend — commands/index.svelte calls reload after CRUD

> **Context:** The file `frontend/src/routes/commands/index.svelte` has four CRUD paths — edit, create new pack, append to existing pack, delete pack, delete command. Scripts already does this (see scripts/index.svelte:149,197). We just need to mirror that pattern.

**Files:**
- Modify: `frontend/src/routes/commands/index.svelte` — three blocks: `saveCmd`, `deletePack`, `deleteCmd`

### Step 1: Write the failing integration test (manual)

There is no automated frontend test harness for this. The "failing test" is:

1. Start `cargo run -p jarvis-app` (or the full Tauri app)
2. In the GUI Commands tab, add a new command with phrase "тест хот релоад"
3. Without restarting, say "тест хот релоад" to Jarvis
4. **Expected:** Command executes. **Actual:** "not found" — Jarvis still uses stale registry.

Document this in a comment above the fix.

### Step 2: Add reload calls in `saveCmd` function

In `frontend/src/routes/commands/index.svelte`, find the `saveCmd` async function (around line 106).

**Current code (lines 106-121):**
```typescript
saving = true
try {
    if (cmdModalMode === "edit") {
        await invoke("update_command", { packName: editingPack, oldId: editingOldId, command: payload })
        flashCmd(`Command "${cmdForm.id}" updated`)
    } else if (cmdForm.packTarget === "__new__") {
        await invoke("create_command_pack", { packName: effectivePack, command: payload })
        flashCmd(`Pack "${effectivePack}" created`)
    } else {
        await invoke("append_command_to_pack", { packName: effectivePack, command: payload })
        flashCmd(`Command "${cmdForm.id}" added`)
    }
    closeCmdModal(); await loadPacks()
} catch (e) { cmdForm.formError = String(e) }
saving = false
```

**Replace with:**
```typescript
saving = true
try {
    if (cmdModalMode === "edit") {
        await invoke("update_command", { packName: editingPack, oldId: editingOldId, command: payload })
        flashCmd(`Command "${cmdForm.id}" updated`)
    } else if (cmdForm.packTarget === "__new__") {
        await invoke("create_command_pack", { packName: effectivePack, command: payload })
        flashCmd(`Pack "${effectivePack}" created`)
    } else {
        await invoke("append_command_to_pack", { packName: effectivePack, command: payload })
        flashCmd(`Command "${cmdForm.id}" added`)
    }
    // Hot-reload: push changes to running jarvis-app without restart.
    // Silently ignored if jarvis-app is not running.
    invoke("reload_jarvis_commands").catch(() => {/* jarvis-app may not be running */})
    closeCmdModal(); await loadPacks()
} catch (e) { cmdForm.formError = String(e) }
saving = false
```

### Step 3: Add reload call in `deletePack` function

**Current code (lines 123-130):**
```typescript
async function deletePack(name: string) {
    if (deletePackTarget !== name) {
        deletePackTarget = name; setTimeout(() => { deletePackTarget = "" }, 3000); return
    }
    deletePackTarget = ""
    try { await invoke("delete_command_pack", { packName: name }); await loadPacks() }
    catch (e) { cmdError = String(e) }
}
```

**Replace with:**
```typescript
async function deletePack(name: string) {
    if (deletePackTarget !== name) {
        deletePackTarget = name; setTimeout(() => { deletePackTarget = "" }, 3000); return
    }
    deletePackTarget = ""
    try {
        await invoke("delete_command_pack", { packName: name })
        invoke("reload_jarvis_commands").catch(() => {/* jarvis-app may not be running */})
        await loadPacks()
    }
    catch (e) { cmdError = String(e) }
}
```

### Step 4: Add reload call in `deleteCmd` function

**Current code (lines 132-139):**
```typescript
async function deleteCmd(pack: string, id: string) {
    if (!deleteCmdTarget || deleteCmdTarget.pack !== pack || deleteCmdTarget.id !== id) {
        deleteCmdTarget = { pack, id }; setTimeout(() => { deleteCmdTarget = null }, 3000); return
    }
    deleteCmdTarget = null
    try { await invoke("delete_command", { packName: pack, commandId: id }); await loadPacks() }
    catch (e) { cmdError = String(e) }
}
```

**Replace with:**
```typescript
async function deleteCmd(pack: string, id: string) {
    if (!deleteCmdTarget || deleteCmdTarget.pack !== pack || deleteCmdTarget.id !== id) {
        deleteCmdTarget = { pack, id }; setTimeout(() => { deleteCmdTarget = null }, 3000); return
    }
    deleteCmdTarget = null
    try {
        await invoke("delete_command", { packName: pack, commandId: id })
        invoke("reload_jarvis_commands").catch(() => {/* jarvis-app may not be running */})
        await loadPacks()
    }
    catch (e) { cmdError = String(e) }
}
```

### Step 5: Verify TypeScript compiles

```bash
cd frontend && npx tsc --noEmit
```

Expected: no errors.

### Step 6: Commit

```bash
git add frontend/src/routes/commands/index.svelte
git commit -m "feat(hot-reload): trigger reload_jarvis_commands after command CRUD"
```

---

## Task 2: Backend — include script virtual commands in intent reinit on reload

> **Context:** When `IpcAction::ReloadCommands` fires, `main.rs:178` retrains the intent classifier on `command_registry::get_snapshot()` — but this snapshot only has regular commands, NOT scripts as virtual commands. Scripts have `phrases_ru`/`phrases_en` that feed the intent classifier. Without this fix, adding a script via GUI won't register its voice trigger.

**Files:**
- Modify: `crates/jarvis-app/src/main.rs:170-189` — `ReloadCommands` handler
- Test: `crates/jarvis-app/src/` — add unit test in `main.rs` `#[cfg(test)]` block or dedicated integration test

### Step 1: Read the current ReloadCommands handler

Read `crates/jarvis-app/src/main.rs` lines 170–189 to confirm exact current state.

Current logic:
```rust
IpcAction::ReloadCommands => {
    info!("Received reload commands request — reloading from disk");
    match commands::parse_commands() {
        Ok(new_cmds) => {
            command_registry::load(new_cmds);
            info!("Commands reloaded successfully");
            let cmds_snapshot = command_registry::get_snapshot();
            let reload_rt = Arc::clone(&rt_for_reload);
            std::thread::spawn(move || {
                if let Err(e) = reload_rt.block_on(intent::reinit(&cmds_snapshot)) {
                    error!("Intent classifier reload failed: {}", e);
                }
            });
        }
        Err(e) => {
            error!("Failed to reload commands: {}", e);
        }
    }
}
```

### Step 2: Write a unit test documenting expected behaviour

Add to `crates/jarvis-core/src/scripts.rs` in the existing `#[cfg(test)]` module:

```rust
#[test]
fn test_as_virtual_commands_includes_phrases() {
    use crate::scripts::{Script, ScriptStep, as_virtual_commands};

    let script = Script {
        id: "test_script".to_string(),
        phrases_ru: vec!["привет мир".to_string()],
        phrases_en: vec!["hello world".to_string()],
        steps: vec![],
        mode: "sequential".to_string(),
        sounds_ru: vec![],
        response_sound: String::new(),
    };
    let virtual_cmds = as_virtual_commands(&[script]);
    assert_eq!(virtual_cmds.len(), 1);
    let cmd = &virtual_cmds[0].commands[0];
    assert_eq!(cmd.cmd_type, "script_ref");
    assert_eq!(cmd.id, "test_script");
}
```

Run:
```bash
cargo test -p jarvis-core test_as_virtual_commands_includes_phrases -- --nocapture
```

Expected: PASS (this documents the interface we rely on).

### Step 3: Update the ReloadCommands handler to merge script virtual commands

In `crates/jarvis-app/src/main.rs`, replace the `ReloadCommands` handler body:

**Find:**
```rust
IpcAction::ReloadCommands => {
    info!("Received reload commands request — reloading from disk");
    match commands::parse_commands() {
        Ok(new_cmds) => {
            command_registry::load(new_cmds);
            info!("Commands reloaded successfully");
            let cmds_snapshot = command_registry::get_snapshot();
            let reload_rt = Arc::clone(&rt_for_reload);
            std::thread::spawn(move || {
                if let Err(e) = reload_rt.block_on(intent::reinit(&cmds_snapshot)) {
                    error!("Intent classifier reload failed: {}", e);
                }
            });
        }
        Err(e) => {
            error!("Failed to reload commands: {}", e);
        }
    }
}
```

**Replace with:**
```rust
IpcAction::ReloadCommands => {
    info!("Received reload commands request — reloading from disk");
    match commands::parse_commands() {
        Ok(new_cmds) => {
            command_registry::load(new_cmds);
            info!("Commands reloaded successfully");
            // Merge script virtual commands so the intent classifier
            // also learns new script voice triggers on hot-reload.
            let mut all_cmds = command_registry::get_snapshot();
            let script_virtual = scripts::as_virtual_commands(&scripts::parse_scripts());
            all_cmds.extend(script_virtual);
            let reload_rt = Arc::clone(&rt_for_reload);
            std::thread::spawn(move || {
                if let Err(e) = reload_rt.block_on(intent::reinit(&all_cmds)) {
                    error!("Intent classifier reload failed: {}", e);
                }
            });
        }
        Err(e) => {
            error!("Failed to reload commands: {}", e);
        }
    }
}
```

### Step 4: Verify it compiles

```bash
cargo build -p jarvis-app 2>&1
```

Expected: no errors. If `scripts` is not in scope at that point in main.rs, add `use jarvis_core::scripts;` to the imports at the top (check what's already imported).

### Step 5: Run clippy

```bash
cargo clippy -p jarvis-app -- -D warnings
```

Expected: no warnings.

### Step 6: Commit

```bash
git add crates/jarvis-app/src/main.rs crates/jarvis-core/src/scripts.rs
git commit -m "fix(hot-reload): include script virtual cmds in intent reinit on ReloadCommands"
```

---

## Task 3: Verification (manual smoke test)

This is the "did it work" gate before marking TASK-006 done.

### Step 1: Build and run

```bash
cargo build --release -p jarvis-app
# then start the full Tauri app
```

### Step 2: Test command hot-reload

1. Open GUI → Commands tab
2. Add a new command in any pack with phrase "горячая перезагрузка"
3. **Without restarting Jarvis**, trigger wake word, say "горячая перезагрузка"
4. Expected: command executes (or "not found" sound if exe_path is bogus — but it should NOT be the generic "command not found")

### Step 3: Test command delete hot-reload

1. Delete the command just created
2. Say "горячая перезагрузка" again
3. Expected: "not found"

### Step 4: Test script hot-reload (intent path)

1. Open GUI → Scripts tab
2. Add a script with phrase "скрипт горячий тест"
3. **Without restarting**, trigger wake word, say "скрипт горячий тест"
4. Expected: script executes

### Step 5: Run unit tests

```bash
cargo test -p jarvis-core -- --nocapture
cargo test -p jarvis-app -- --nocapture
```

Expected: all pass.

### Step 6: Run clippy + fmt

```bash
cargo fmt --all
cargo clippy -- -D warnings
```

Expected: clean.

---

## Task 4: Mark TASK-006 done + commit docs

### Step 1: Update TASKS.md

In `docs/TASKS.md`, move TASK-006 from `### In Progress` to `### Done`:

```markdown
- [x] TASK-006: Hot-reload команд и скриптов без перезапуска
```

### Step 2: Update MEMORY.md

Add ADR to `docs/MEMORY.md`:

| ADR-005 | Hot-reload via IpcAction::ReloadCommands | Existing WebSocket IPC path, no new Event Bus events needed | 2026-03-14 |

Update "Last milestone":
```
**Last milestone:** TASK-006 Hot-reload — commands/index.svelte now calls reload_jarvis_commands after CRUD; ReloadCommands handler includes script virtual cmds in intent reinit (2026-03-14)
```

### Step 3: Commit

```bash
git add docs/TASKS.md docs/MEMORY.md
git commit -m "docs(tasks): mark TASK-006 Hot-reload as done"
```

---

## Summary of Changes

| File | Change |
|------|--------|
| `frontend/src/routes/commands/index.svelte` | +3 `invoke("reload_jarvis_commands")` calls after CRUD |
| `crates/jarvis-app/src/main.rs` | ReloadCommands handler merges script virtual cmds into intent reinit |
| `crates/jarvis-core/src/scripts.rs` | +1 unit test for `as_virtual_commands` |
| `docs/TASKS.md` | TASK-006 → Done |
| `docs/MEMORY.md` | ADR-005 + Last milestone |

**Total: ~30 lines of code changed across 2 files + docs.**
