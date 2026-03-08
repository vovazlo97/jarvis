# Response Sound for Commands — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Save `response_sound` field when creating/updating commands via the GUI (the frontend already sends it but Rust silently drops it).

**Architecture:** `NewCommandInput` struct in `commands.rs` gains a `response_sound` field; `build_toml()` and `cmd_to_toml_block()` write it to TOML when non-empty; the existing `JCommand.response_sound` field (already present) picks it up on read.

**Tech Stack:** Rust (Tauri GUI backend), TOML serialization

---

### Task 1: Add `response_sound` to `NewCommandInput` and write it to TOML

**Files:**
- Modify: `crates/jarvis-gui/src/tauri_commands/commands.rs`

**Step 1: Add field to struct**

In `NewCommandInput` (around line 44), add after `sounds_ru`:

```rust
pub sounds_ru: Vec<String>,
#[serde(default)]
pub response_sound: String,
```

**Step 2: Write it in `build_toml()`**

After the `patterns` block (around line 328), before the final `lines.join`:

```rust
if !cmd.response_sound.is_empty() {
    lines.push(format!("response_sound = {}", esc(&cmd.response_sound)));
}
```

**Step 3: Write it in `cmd_to_toml_block()`**

After the `patterns` block (around line 274), before `lines.join`:

```rust
if !cmd.response_sound.is_empty() {
    lines.push(format!("response_sound = {}", esc(&cmd.response_sound)));
}
```

**Step 4: Verify the fix compiles**

Run:
```bash
cd crates/jarvis-gui && cargo check
```
Expected: no errors.

**Step 5: Manual test**

1. Build the GUI: `cargo tauri dev` (or `cargo build`)
2. Open Commands page → Edit any command → set Response Sound to any `.wav`
3. Click "Save Changes"
4. Re-open the same command → Response Sound dropdown should show the saved value

---

### Notes on "intent threshold" issue

The user's `режим работы` script fires on `запусти` because `запусти` IS literally in the script's `phrases_ru`. This creates a **perfect match** (score ≥ 99), which bypasses the threshold by design.

`SCRIPT_RATIO_THRESHOLD` is already `88.0` in `config.rs:191` — this is correct.

**Resolution:** The user should edit the script to use a more specific trigger phrase, e.g. `запусти режим работы` instead of just `запусти`.

No code change needed for this.
