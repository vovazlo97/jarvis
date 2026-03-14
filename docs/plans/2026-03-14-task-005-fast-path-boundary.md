# TASK-005: Fast Path Boundary Module Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extract Fast Path code (`VadState`, `drain_echo`, `recognize_command`, `process_text_command`, `execute_command`) from `app.rs` into a dedicated `fast_path.rs` module so the boundary is explicit and the constraints are enforced at the module level.

**Architecture:** Pure structural refactor — zero behavior change. `fast_path.rs` becomes the canonical location for all latency-critical code. The module-level doc comment declares the hard constraints (NO LLM, <250ms). `app.rs` keeps only `start` + `main_loop` (orchestration layer) and calls into `fast_path::*`.

**Tech Stack:** Rust, jarvis-app binary crate (`crates/jarvis-app/`).

---

## Prerequisite Reading

Before starting, read these files:
- `crates/jarvis-app/src/app.rs:1-25` — imports and VadState enum
- `crates/jarvis-app/src/app.rs:375-427` — `drain_echo` + `process_text_command`
- `crates/jarvis-app/src/app.rs:429-584` — `execute_command`
- `crates/jarvis-app/src/app.rs:187-373` — `recognize_command`
- `crates/jarvis-app/src/main.rs:1-25` — where `mod app;` is declared

---

## Task 1: Create `fast_path.rs` with its own failing compile test

**Files:**
- Create: `crates/jarvis-app/src/fast_path.rs`
- Modify: `crates/jarvis-app/src/main.rs` — add `mod fast_path;`

### Step 1: Create empty `fast_path.rs` with module doc

```rust
//! Fast Path — latency-critical pipeline.
//!
//! # HARD CONSTRAINTS (see .claude/rules/fast-path.md)
//! - **NO** async LLM API calls (OpenAI, Anthropic, Ollama, etc.)
//! - **NO** blocking HTTP / network I/O
//! - **NO** file I/O heavier than config reads
//! - All processing MUST complete in <250ms P50
//!
//! Violations will be caught by `cargo clippy` and blocked in CI.
```

Save as `crates/jarvis-app/src/fast_path.rs`.

### Step 2: Register the module in `main.rs`

In `crates/jarvis-app/src/main.rs`, find `mod app;` and add `mod fast_path;` directly below it:

```rust
mod app;
mod fast_path;
```

### Step 3: Verify it compiles

```bash
cargo check -p jarvis-app 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 2: Move `VadState` and imports into `fast_path.rs`

**Files:**
- Modify: `crates/jarvis-app/src/fast_path.rs`
- Modify: `crates/jarvis-app/src/app.rs` — remove VadState and add import

### Step 1: Add imports + VadState to `fast_path.rs`

Replace the file content with:

```rust
//! Fast Path — latency-critical pipeline.
//!
//! # HARD CONSTRAINTS (see .claude/rules/fast-path.md)
//! - **NO** async LLM API calls (OpenAI, Anthropic, Ollama, etc.)
//! - **NO** blocking HTTP / network I/O
//! - **NO** file I/O heavier than config reads
//! - All processing MUST complete in <250ms P50
//!
//! Violations will be caught by `cargo clippy` and blocked in CI.

use std::time::SystemTime;

use jarvis_core::{
    audio,
    audio_buffer::AudioRingBuffer,
    audio_processing, command_registry, commands, config, i18n, intent,
    ipc::{self, IpcEvent},
    recorder, scripts, slots, stt, voices, AssistantState, SOUND_DIR,
};

use crate::should_stop;

// VAD state machine
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum VadState {
    WaitingForVoice,
    VoiceActive,
}
```

### Step 2: In `app.rs` — remove VadState definition and update imports

Remove lines 11-20 from `app.rs` (the `rand` import and `VadState` enum):

```rust
// DELETE these lines:
use rand::seq::SliceRandom;

// VAD state machine
#[derive(Debug, Clone, Copy, PartialEq)]
enum VadState {
    WaitingForVoice,
    VoiceActive,
}
```

Add at top of `app.rs` (after existing imports):

```rust
use crate::fast_path::VadState;
```

Also clean up `app.rs` imports — remove what's only needed by the functions being moved:
- Keep: `audio_buffer::AudioRingBuffer`, `audio_processing`, `config`, `i18n`, `ipc`, `listener`, `recorder`, `stt`, `voices`, `AssistantState`
- Remove from app.rs: `audio`, `command_registry`, `commands`, `intent`, `scripts`, `slots`, `SOUND_DIR` (these will be in fast_path.rs)

### Step 3: Verify compile

```bash
cargo check -p jarvis-app 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 3: Move `drain_echo` to `fast_path.rs`

**Files:**
- Modify: `crates/jarvis-app/src/fast_path.rs` — add function
- Modify: `crates/jarvis-app/src/app.rs` — remove function, add import

### Step 1: Copy `drain_echo` into `fast_path.rs`

After the VadState enum in `fast_path.rs`, add:

```rust
/// Drain microphone frames while Kira audio is playing, then add a dead zone
/// and a reverb tail drain. Call this after ANY audio playback + execute_command
/// to prevent speaker echo from reaching Vosk or the wake word detector.
///
/// - Reads and discards frames until audio::is_playing() returns false
/// - Adds 300 ms dead zone (flushes audio-card output buffer residue from PvRecorder)
/// - Adds 1 s reverb tail (clears remaining ring-buffer frames)
/// - Safety cap: never blocks longer than `max_secs` seconds total
pub(crate) fn drain_echo(
    frame_buffer: &mut [i16],
    sample_rate: usize,
    frame_length: usize,
    max_secs: u64,
) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(max_secs);
    let mut n: usize = 0;
    while !crate::should_stop() && audio::is_playing() && std::time::Instant::now() < deadline {
        recorder::read_microphone(frame_buffer);
        n += 1;
    }
    // 300 ms dead zone + 1 s reverb tail: discard audio-card / PvRecorder residue
    let extra = ((0.3 + 1.0) * sample_rate as f32 / frame_length as f32) as usize;
    for _ in 0..extra {
        recorder::read_microphone(frame_buffer);
    }
    debug!(
        "[EchoDrain] drained {} playback + {} tail = {} frames total",
        n,
        extra,
        n + extra
    );
}
```

### Step 2: Remove `drain_echo` from `app.rs` (lines 375-401)

Delete the entire `drain_echo` function from `app.rs`.

### Step 3: Add import in `app.rs`

In `app.rs`, add to the imports:

```rust
use crate::fast_path::{drain_echo, VadState};
```

### Step 4: Verify compile

```bash
cargo check -p jarvis-app 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 4: Move `execute_command` and `process_text_command` to `fast_path.rs`

**Files:**
- Modify: `crates/jarvis-app/src/fast_path.rs`
- Modify: `crates/jarvis-app/src/app.rs`

### Step 1: Copy `execute_command` (app.rs lines 429-584) into `fast_path.rs`

Copy the entire function verbatim, changing visibility to `pub(crate)`:

```rust
// Execute command, returns true if chaining should continue
pub(crate) fn execute_command(text: &str, rt: &tokio::runtime::Runtime) -> bool {
    // ... (entire body copied verbatim from app.rs)
}
```

### Step 2: Copy `process_text_command` (app.rs lines 403-427) into `fast_path.rs`

```rust
pub(crate) fn process_text_command(text: &str, rt: &tokio::runtime::Runtime) {
    // ... (entire body copied verbatim from app.rs)
}
```

### Step 3: Delete both functions from `app.rs`

Remove lines 403-584 from `app.rs`.

### Step 4: Update app.rs imports

```rust
use crate::fast_path::{drain_echo, execute_command, process_text_command, VadState};
```

### Step 5: Verify compile

```bash
cargo check -p jarvis-app 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 5: Move `recognize_command` to `fast_path.rs`

**Files:**
- Modify: `crates/jarvis-app/src/fast_path.rs`
- Modify: `crates/jarvis-app/src/app.rs`

### Step 1: Copy `recognize_command` (app.rs lines 187-373) into `fast_path.rs`

```rust
// Voice recognition for command after wake word
pub(crate) fn recognize_command(
    frame_buffer: &mut [i16],
    rt: &tokio::runtime::Runtime,
    frame_length: usize,
    sample_rate: usize,
    prefed_audio: bool,
) {
    // ... (entire body copied verbatim from app.rs)
}
```

### Step 2: Delete `recognize_command` from `app.rs` (lines 187-373)

### Step 3: Update app.rs imports

```rust
use crate::fast_path::{drain_echo, execute_command, process_text_command, recognize_command, VadState};
```

Wait — `drain_echo` is called only from `recognize_command` (which is now in fast_path.rs), so remove `drain_echo` from the app.rs import list. Keep only what main_loop uses directly.

### Step 4: Clean up `app.rs` imports

After all moves, `app.rs` should only import what `start` and `main_loop` actually use:

```rust
use std::sync::mpsc::Receiver;

use jarvis_core::{
    audio_buffer::AudioRingBuffer,
    audio_processing, config, i18n,
    ipc::{self, IpcEvent},
    listener, recorder, stt, voices, AssistantState,
};

use crate::{
    fast_path::{process_text_command, recognize_command, VadState},
    should_stop,
};
```

### Step 5: Verify compile

```bash
cargo check -p jarvis-app 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 6: Run tests + clippy + fmt, then commit

### Step 1: Run all tests

```bash
cargo test -p jarvis-core 2>&1 | tail -5
cargo test -p jarvis-app 2>&1 | tail -5
```

Expected: all pass.

### Step 2: Run clippy

```bash
cargo clippy -p jarvis-app --no-deps 2>&1 | grep "^error"
```

Expected: no errors. (Pre-existing warnings are OK as long as no new ones in fast_path.rs.)

### Step 3: Run fmt

```bash
cargo fmt --all
```

### Step 4: Verify app still builds

```bash
cargo check -p jarvis-app 2>&1 | grep "^error"
```

Expected: no errors.

### Step 5: Commit

```bash
git add crates/jarvis-app/src/fast_path.rs crates/jarvis-app/src/app.rs crates/jarvis-app/src/main.rs
git commit -m "refactor(fast-path): extract Fast Path boundary into dedicated module (TASK-005)"
```

---

## Task 7: Update TASKS.md + MEMORY.md

### Step 1: Mark TASK-005 done in `docs/TASKS.md`

Move `TASK-005` from `### In Progress` to `### Done`.

### Step 2: Update `docs/MEMORY.md`

Add ADR entry:

```markdown
| ADR-006 | Fast Path in fast_path.rs module | Clear latency boundary; all <250ms code lives in one auditable module | 2026-03-14 |
```

Update Last milestone.

### Step 3: Commit

```bash
git add docs/TASKS.md docs/MEMORY.md
git commit -m "docs(tasks): mark TASK-005 done — Fast Path boundary extracted"
```

---

## Summary of Changes

| File | Change |
|------|--------|
| `crates/jarvis-app/src/fast_path.rs` | New file: module doc + constraints, VadState, drain_echo, execute_command, process_text_command, recognize_command |
| `crates/jarvis-app/src/app.rs` | Remove all moved code; keep start + main_loop; add `use crate::fast_path::*` |
| `crates/jarvis-app/src/main.rs` | Add `mod fast_path;` |
| `docs/TASKS.md` | TASK-005 → Done |
| `docs/MEMORY.md` | ADR-006 + Last milestone |

**Total: ~400 lines moved, zero lines changed in logic.**
