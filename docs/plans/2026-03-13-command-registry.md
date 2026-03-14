# Command Registry Refactor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a `command_registry` module in `jarvis-core` that encapsulates the global `COMMANDS_LIST` behind a clean API, enforcing that all writes are atomic full-replacements and preventing callers from doing partial mutations.

**Architecture:** A new `crates/jarvis-core/src/command_registry.rs` wraps the existing `Lazy<RwLock<Vec<JCommandsList>>>` global with `load()` (atomic full-replace write), `read()` (shared guard), `get_snapshot()` (cloned Vec), and `is_loaded()`. The raw `COMMANDS_LIST` global is kept for backward-compat of existing readers. All three **write** call sites in `jarvis-app/main.rs` and `jarvis-cli/main.rs` are migrated to `command_registry::load()`. Read call sites are left unchanged (YAGNI — they already use safe shared guards).

**Tech Stack:** Rust, `parking_lot::RwLock` (already a dependency), `once_cell::sync::Lazy` (already used)

---

## Current State (from codebase exploration)

- `COMMANDS_LIST: Lazy<RwLock<Vec<JCommandsList>>>` declared in `crates/jarvis-core/src/lib.rs:71`
- **3 write sites** (must migrate):
  - `crates/jarvis-app/src/main.rs:120` — initial load: `*COMMANDS_LIST.write() = cmds;`
  - `crates/jarvis-app/src/main.rs:175` — reload on IPC: `*COMMANDS_LIST.write() = new_cmds;`
  - `crates/jarvis-cli/src/main.rs:132` — initial load: `*COMMANDS_LIST.write() = cmds;`
- **5 read sites** (leave unchanged):
  - `crates/jarvis-app/src/app.rs:432` — `COMMANDS_LIST.read()`
  - `crates/jarvis-app/src/main.rs:138,179` — `COMMANDS_LIST.read().to_vec()`
  - `crates/jarvis-core/src/scripts.rs:300` — `crate::COMMANDS_LIST.read()`
  - `crates/jarvis-cli/src/main.rs:136,174,175,177,198` — `COMMANDS_LIST.read()`
- `JCommandsList` is re-exported from `commands::JCommandsList` in lib.rs

---

### Task 1: Write failing tests for the `command_registry` API

**Files:**
- Create: `crates/jarvis-core/src/command_registry.rs`

**Step 1: Create the file with tests only — no implementation**

```rust
// crates/jarvis-core/src/command_registry.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{JCommandsList};

    fn make_pack(id: &str) -> JCommandsList {
        JCommandsList {
            path: std::path::PathBuf::from(id),
            commands: vec![],
        }
    }

    #[test]
    fn test_load_stores_commands() {
        let pack = make_pack("test_pack_a");
        load(vec![pack]);
        assert!(is_loaded());
    }

    #[test]
    fn test_load_is_atomic_replacement() {
        // First load
        load(vec![make_pack("old_pack")]);
        // Second load replaces entirely
        load(vec![make_pack("new_pack_1"), make_pack("new_pack_2")]);
        let snapshot = get_snapshot();
        // Registry has exactly 2 packs (not 3)
        assert_eq!(snapshot.len(), 2);
    }

    #[test]
    fn test_get_snapshot_returns_all() {
        load(vec![make_pack("snap_a"), make_pack("snap_b")]);
        let snapshot = get_snapshot();
        assert_eq!(snapshot.len(), 2);
    }

    #[test]
    fn test_read_guard_gives_access() {
        load(vec![make_pack("guard_pack")]);
        let guard = read();
        // guard is a RwLockReadGuard<Vec<JCommandsList>>
        // must have at least 1 entry
        assert!(!guard.is_empty());
    }

    #[test]
    fn test_is_loaded_true_after_load() {
        load(vec![make_pack("loaded_pack")]);
        assert!(is_loaded());
    }
}
```

**Step 2: Add `pub mod command_registry;` to `lib.rs`** — insert after `pub mod commands;` (line 13):
```rust
pub mod command_registry;
```

**Step 3: Run — expect compile error (functions not defined)**

```bash
cd "D:\Jarvis\jarvis2\jarvis-master\jarvis-master" && cargo test --package jarvis-core command_registry 2>&1 | head -15
```

Expected: `error[E0425]: cannot find function 'load' in this scope`

---

### Task 2: Implement the `command_registry` module — make all 5 tests pass

**Files:**
- Modify: `crates/jarvis-core/src/command_registry.rs`

**Step 1: Add implementation above the `#[cfg(test)]` block**

```rust
use parking_lot::RwLockReadGuard;

use crate::{JCommandsList, COMMANDS_LIST};

/// Load (atomically replace) the entire command registry.
///
/// This is the ONLY way to write to the registry. The entire `Vec<JCommandsList>`
/// is replaced as one atomic operation under an exclusive write lock.
/// Callers cannot do partial mutations.
pub fn load(commands: Vec<JCommandsList>) {
    *COMMANDS_LIST.write() = commands;
    log::info!(
        "CommandRegistry: loaded {} pack(s)",
        COMMANDS_LIST.read().len()
    );
}

/// Acquire a shared read guard over the registry.
///
/// Multiple readers can hold guards simultaneously.
/// Drops automatically when the guard goes out of scope.
pub fn read<'a>() -> RwLockReadGuard<'a, Vec<JCommandsList>> {
    COMMANDS_LIST.read()
}

/// Return a cloned snapshot of the registry.
///
/// Use when you need an owned `Vec` (e.g., to pass to a background thread).
pub fn get_snapshot() -> Vec<JCommandsList> {
    COMMANDS_LIST.read().to_vec()
}

/// Returns `true` if the registry contains at least one command pack.
pub fn is_loaded() -> bool {
    !COMMANDS_LIST.read().is_empty()
}
```

**Step 2: Run tests — expect all 5 to pass**

```bash
cd "D:\Jarvis\jarvis2\jarvis-master\jarvis-master" && cargo test --package jarvis-core command_registry 2>&1
```

Expected:
```
test command_registry::tests::test_load_stores_commands ... ok
test command_registry::tests::test_load_is_atomic_replacement ... ok
test command_registry::tests::test_get_snapshot_returns_all ... ok
test command_registry::tests::test_read_guard_gives_access ... ok
test command_registry::tests::test_is_loaded_true_after_load ... ok
test result: ok. 5 passed; 0 failed
```

**Step 3: Run full package tests to verify no regressions**

```bash
cd "D:\Jarvis\jarvis2\jarvis-master\jarvis-master" && cargo test --package jarvis-core --all-features 2>&1 | grep "test result"
```

Expected: `test result: ok. 33 passed; 0 failed` (28 prev + 5 new)

**Step 4: Commit**

```bash
cd "D:\Jarvis\jarvis2\jarvis-master\jarvis-master" && git add crates/jarvis-core/src/command_registry.rs crates/jarvis-core/src/lib.rs && git commit -m "feat(command-registry): add load/read/get_snapshot/is_loaded with atomic write guarantee"
```

---

### Task 3: Migrate write call sites to `command_registry::load()`

**Files:**
- Modify: `crates/jarvis-app/src/main.rs`
- Modify: `crates/jarvis-cli/src/main.rs`

**Step 1: Read `crates/jarvis-app/src/main.rs`** to find exact import and write lines.

The file currently imports:
```rust
use jarvis_core::{
    ..., COMMANDS_LIST, DB,
};
```

**Step 2: In `crates/jarvis-app/src/main.rs` — update import**

Change the `COMMANDS_LIST` import to add `command_registry`:
```rust
use jarvis_core::{
    ..., command_registry, COMMANDS_LIST, DB,
};
```

(Keep `COMMANDS_LIST` in the import — it's still used by read sites on lines 138 and 179.)

**Step 3: In `crates/jarvis-app/src/main.rs` — migrate line 120 (initial load)**

Change:
```rust
*COMMANDS_LIST.write() = cmds;
```
To:
```rust
command_registry::load(cmds);
```

**Step 4: In `crates/jarvis-app/src/main.rs` — migrate line 175 (IPC reload)**

Change:
```rust
*COMMANDS_LIST.write() = new_cmds;
```
To:
```rust
command_registry::load(new_cmds);
```

**Step 5: Read `crates/jarvis-cli/src/main.rs`** to find exact import and write line.

**Step 6: In `crates/jarvis-cli/src/main.rs` — update import and migrate line 132**

In the import, add `command_registry`:
```rust
use jarvis_core::{commands, command_registry, config, db, intent, JCommandsList, COMMANDS_LIST, DB};
```

Change line 132:
```rust
*COMMANDS_LIST.write() = cmds;
```
To:
```rust
command_registry::load(cmds);
```

**Step 7: Build the workspace to verify no compile errors**

```bash
cd "D:\Jarvis\jarvis2\jarvis-master\jarvis-master" && cargo build --workspace 2>&1 | grep "^error"
```

Expected: no output (no errors).

**Step 8: Run full workspace tests**

```bash
cd "D:\Jarvis\jarvis2\jarvis-master\jarvis-master" && cargo test --workspace 2>&1 | grep "test result"
```

Expected: all test results show 0 failed.

**Step 9: Commit**

```bash
cd "D:\Jarvis\jarvis2\jarvis-master\jarvis-master" && git add crates/jarvis-app/src/main.rs crates/jarvis-cli/src/main.rs && git commit -m "refactor(command-registry): migrate write call sites to command_registry::load()"
```

---

### Task 4: cargo fmt + clippy clean

**Files:** none (verification only)

**Step 1: Format**

```bash
cd "D:\Jarvis\jarvis2\jarvis-master\jarvis-master" && cargo fmt --all 2>&1
```

**Step 2: Clippy on modified crates**

```bash
cd "D:\Jarvis\jarvis2\jarvis-master\jarvis-master" && cargo clippy --package jarvis-core --package jarvis-app --package jarvis-cli -- -D warnings 2>&1 | grep "^error"
```

Expected: no output. (Pre-existing warnings in other files are not our responsibility.)

**Step 3: If any changes, run tests and commit**

```bash
cd "D:\Jarvis\jarvis2\jarvis-master\jarvis-master" && cargo test --workspace 2>&1 | grep "test result"
cd "D:\Jarvis\jarvis2\jarvis-master\jarvis-master" && git diff --stat
# If changes:
cd "D:\Jarvis\jarvis2\jarvis-master\jarvis-master" && git add -u && git commit -m "chore(command-registry): apply cargo fmt + clippy fixes"
```

---

### Task 5: Update docs/TASKS.md and docs/MEMORY.md

**Files:**
- Modify: `docs/TASKS.md`
- Modify: `docs/MEMORY.md`

**Step 1: In `docs/TASKS.md`**

Move TASK-002 from In Progress to Done:
- Change `- [ ] TASK-002: Refactor Command Registry with atomic writes` → `- [x] TASK-002: Refactor Command Registry with atomic writes`
- Move under `### Done` (after TASK-001)

**Step 2: In `docs/MEMORY.md`** — update `Last milestone`:

```markdown
- **Last milestone:** TASK-002 Command Registry refactored — command_registry module with atomic load() in jarvis-core (2026-03-13)
```

**Step 3: Commit**

```bash
cd "D:\Jarvis\jarvis2\jarvis-master\jarvis-master" && git add docs/TASKS.md docs/MEMORY.md && git commit -m "docs(tasks): mark TASK-002 Command Registry as done"
```

---

## Post-Completion

After all 5 tasks complete and verified:
- Use `/finishing-a-development-branch` to push updated branch to GitHub
- No PR to main until Phase A fully complete
