# Model System Robustness Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Eliminate the crash when "Embedding Classifier" is selected in GUI and make the model system resilient to missing binaries and extensible without code changes.

**Architecture:** Three independent layers fix three distinct root causes. (1) The catalog scan validates binary files — models with Git LFS pointers are excluded before they can be selected. (2) `main.rs` gracefully degrades on intent init failure instead of calling `close(1)`. (3) The frontend replaces hardcoded backend values with a dynamic list from a new Tauri command, so adding a new model only requires a `model.toml` file.

**Tech Stack:** Rust (jarvis-core, jarvis-app, jarvis-gui), Svelte/TypeScript (frontend), Tauri commands.

---

## Root Cause Summary

| # | Root cause | Where | Symptom |
|---|-----------|--------|---------|
| RC-1 | `paraphrase-multilingual/model.onnx` is a 134-byte Git LFS pointer | `target/debug/resources/models/` | `protobuf parsing failed` |
| RC-2 | `app::close(1)` on any intent init error | `crates/jarvis-app/src/main.rs:141-143` | Hard crash instead of graceful degradation |
| RC-3 | Frontend hardcodes `"EmbeddingClassifier"` / `"IntentClassifier"` | `frontend/src/routes/settings/index.svelte:413-414` | User can select a backend that doesn't exist in catalog |

---

## Task 1: LFS Pointer Validation in Catalog Scan

Exclude models whose binary files are Git LFS pointers. The catalog becomes the single source of truth: only models with valid, loadable files appear.

**Files:**
- Modify: `crates/jarvis-core/src/models/catalog.rs`

**Step 1: Write failing test**

Add to the bottom of `crates/jarvis-core/src/models/catalog.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    /// A model.toml whose model.onnx is a Git LFS pointer must NOT appear in catalog.
    #[test]
    fn lfs_pointer_model_excluded_from_catalog() {
        let dir = TempDir::new().unwrap();
        let model_dir = dir.path().join("test-model");
        std::fs::create_dir_all(&model_dir).unwrap();

        // write a valid model.toml
        std::fs::write(
            model_dir.join("model.toml"),
            "[model]\nid = \"test-model\"\nname = \"Test\"\ntasks = [\"intent\"]\n",
        )
        .unwrap();

        // write a Git LFS pointer as model.onnx
        std::fs::write(
            model_dir.join("model.onnx"),
            "version https://git-lfs.github.com/spec/v1\noid sha256:abc123\nsize 235052644\n",
        )
        .unwrap();

        let models = scan_models(dir.path());
        assert!(
            models.is_empty(),
            "Model with LFS pointer should be excluded from catalog"
        );
    }

    /// A model.toml whose model.onnx is a real binary must appear in catalog.
    #[test]
    fn real_binary_model_included_in_catalog() {
        let dir = TempDir::new().unwrap();
        let model_dir = dir.path().join("real-model");
        std::fs::create_dir_all(&model_dir).unwrap();

        std::fs::write(
            model_dir.join("model.toml"),
            "[model]\nid = \"real-model\"\nname = \"Real\"\ntasks = [\"intent\"]\n",
        )
        .unwrap();

        // write a real binary (does not start with LFS header)
        let mut f = std::fs::File::create(model_dir.join("model.onnx")).unwrap();
        f.write_all(&[0x08, 0x01, 0x12, 0x04]).unwrap(); // protobuf magic
        f.write_all(&[0u8; 2000]).unwrap(); // > 1 KB

        let models = scan_models(dir.path());
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "real-model");
    }
}
```

**Step 2: Run test — verify it FAILS**

```bash
cargo test -p jarvis-core lfs_pointer_model_excluded_from_catalog 2>&1 | tail -5
```
Expected: `FAILED` — function `is_lfs_pointer` doesn't exist yet.

**Step 3: Implement LFS detection in `load_model_def`**

In `crates/jarvis-core/src/models/catalog.rs`, add the helper and validate after parsing:

```rust
/// Returns true when the file is a Git LFS pointer (starts with "version https://git-lfs").
fn is_lfs_pointer(path: &std::path::Path) -> bool {
    use std::io::Read;
    let mut buf = [0u8; 32];
    match std::fs::File::open(path).and_then(|mut f| f.read(&mut buf).map(|n| n)) {
        Ok(n) if n >= 7 => buf[..7] == *b"version",
        _ => false,
    }
}
```

In `load_model_def`, after building `def` and before returning `Ok(def)`, add:

```rust
// Reject models whose binary files are Git LFS pointers (not yet downloaded).
for entry in std::fs::read_dir(model_dir).map_err(|e| format!("read_dir: {}", e))? {
    let entry = entry.map_err(|e| format!("entry: {}", e))?;
    let p = entry.path();
    if matches!(p.extension().and_then(|e| e.to_str()), Some("onnx") | Some("bin")) {
        if is_lfs_pointer(&p) {
            return Err(format!(
                "binary {:?} is a Git LFS pointer — run 'git lfs pull' to download",
                p.file_name().unwrap_or_default()
            ));
        }
    }
}
```

**Step 4: Run tests — verify they PASS**

```bash
cargo test -p jarvis-core lfs_pointer_model_excluded_from_catalog real_binary_model -- --nocapture 2>&1 | tail -10
```
Expected: `test result: ok. 2 passed`

**Step 5: Run full jarvis-core test suite**

```bash
cargo test -p jarvis-core 2>&1 | tail -5
```
Expected: all pass.

**Step 6: Commit**

```bash
git add crates/jarvis-core/src/models/catalog.rs
git commit -m "fix(catalog): exclude models with Git LFS pointer binaries from scan"
```

---

## Task 2: Graceful Intent Init Failure (No More `close(1)`)

When the intent backend is misconfigured or its model binary is missing, the app should log a warning and continue without intent recognition — not crash.

**Files:**
- Modify: `crates/jarvis-app/src/main.rs` — lines 139-144

**Step 1: Write failing test (manual verification — no unit test possible for this)**

Confirm the crash is reproducible:
```bash
# Set intent_backend to a non-existent model ID
python -c "
import json, pathlib
p = pathlib.Path.home() / 'AppData/Roaming/com.priler.jarvis/app.db'
d = json.loads(p.read_text())
d['intent_backend'] = 'nonexistent-model'
p.write_text(json.dumps(d, indent=2))
"
./target/debug/jarvis-app.exe &
sleep 3
grep -c "Closing application" "$APPDATA/com.priler.jarvis/log.txt"
```
Expected: `1` (app crashed).

**Step 2: Implement graceful degradation**

In `crates/jarvis-app/src/main.rs`, replace lines 139-144:

```rust
// BEFORE:
rt.block_on(async {
    if let Err(e) = intent::init(&cmds_for_intent).await {
        error!("Failed to initialize intent classifier: {}", e);
        app::close(1);
    }
});

// AFTER:
rt.block_on(async {
    if let Err(e) = intent::init(&cmds_for_intent).await {
        warn!(
            "Intent classifier unavailable ({}). Voice commands will use fuzzy matching only.",
            e
        );
    }
});
```

**Step 3: Suppress repeated "Classifier not initialized" errors in classify()**

`embeddingclassifier::classify()` returns `Err("Classifier not initialized")` which intent::classify() currently logs as ERROR on every utterance. Change it to debug:

In `crates/jarvis-core/src/intent.rs`, in the `classify()` function, change the error arm for the embedding backend:

```rust
// BEFORE:
Err(e) => {
    error!("Embedding classification error: {}", e);
    None
}

// AFTER:
Err(e) => {
    debug!("Embedding classification unavailable: {}", e);
    None
}
```

**Step 4: Verify manually**

```bash
# intent_backend = "nonexistent-model" still set
./target/debug/jarvis-app.exe &
sleep 4
grep "Intent classifier unavailable" "$APPDATA/com.priler.jarvis/log.txt" | tail -1
grep "Recording started" "$APPDATA/com.priler.jarvis/log.txt" | tail -1
```
Expected: both lines appear — warning logged, app still started.

**Step 5: Restore app.db**

```bash
python -c "
import json, pathlib
p = pathlib.Path.home() / 'AppData/Roaming/com.priler.jarvis/app.db'
d = json.loads(p.read_text())
d['intent_backend'] = 'all-MiniLM-L6-v2'
p.write_text(json.dumps(d, indent=2))
"
```

**Step 6: Commit**

```bash
git add crates/jarvis-app/src/main.rs crates/jarvis-core/src/intent.rs
git commit -m "fix(app): degrade gracefully on intent init failure instead of close(1)"
```

---

## Task 3: New Tauri Command `list_intent_backends`

The GUI needs to ask the backend "what intent backends are available?" instead of hardcoding them. This makes the model dropdown self-updating: add `model.toml` → appears in GUI automatically.

**Files:**
- Modify: `crates/jarvis-core/src/models.rs` — add `scan_and_get_options`
- Create: `crates/jarvis-gui/src/tauri_commands/models.rs`
- Modify: `crates/jarvis-gui/src/tauri_commands.rs` — add `mod models; pub use models::*;`
- Modify: `crates/jarvis-gui/src/main.rs` — register `list_intent_backends`

**Step 1: Write failing test for `scan_and_get_options`**

Add to `crates/jarvis-core/src/models.rs` tests (inline):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn scan_and_get_options_returns_code_backends_even_with_empty_dir() {
        let dir = TempDir::new().unwrap();
        let options = scan_and_get_options(Task::Intent, dir.path());
        // must always include "none" and "intent-classifier"
        assert!(options.iter().any(|o| o.id == "none"));
        assert!(options.iter().any(|o| o.id == "intent-classifier"));
    }
}
```

**Step 2: Run test — verify FAILS**

```bash
cargo test -p jarvis-core scan_and_get_options 2>&1 | tail -5
```
Expected: FAIL — `scan_and_get_options` not defined.

**Step 3: Add `scan_and_get_options` to `models.rs`**

```rust
/// Scan `models_dir` for valid models and return all available backend options for `task`.
/// Does NOT require models::init() — safe to call from jarvis-gui.
pub fn scan_and_get_options(task: Task, models_dir: &std::path::Path) -> Vec<BackendOption> {
    let models = catalog::scan_models(models_dir);
    catalog::get_options(task, &models)
}
```

Also make the catalog module pub for internal use (it already is `mod catalog`, keep as-is since `scan_and_get_options` is in the same crate).

**Step 4: Run test — verify PASSES**

```bash
cargo test -p jarvis-core scan_and_get_options 2>&1 | tail -5
```
Expected: PASS.

**Step 5: Create `crates/jarvis-gui/src/tauri_commands/models.rs`**

```rust
use jarvis_core::models::{scan_and_get_options, Task, APP_DIR, MODELS_PATH};
use serde::Serialize;

#[derive(Serialize)]
pub struct IntentBackend {
    pub id: String,
    pub name: String,
}

/// Returns all available intent backends: code backends ("none", "intent-classifier")
/// plus any AI models registered in the model catalog with valid binaries.
#[tauri::command]
pub fn list_intent_backends() -> Vec<IntentBackend> {
    let models_dir = APP_DIR.join(MODELS_PATH);
    scan_and_get_options(Task::Intent, &models_dir)
        .into_iter()
        .map(|b| IntentBackend { id: b.id, name: b.name })
        .collect()
}
```

Note: `APP_DIR` and `MODELS_PATH` must be re-exported from `jarvis_core`. Check they are public — `APP_DIR` is `pub static`, `MODELS_PATH` is `pub const`. If not exported at crate root, use full path `jarvis_core::APP_DIR` and `jarvis_core::models::MODELS_PATH`.

**Step 6: Register in `tauri_commands.rs` and `main.rs`**

In `crates/jarvis-gui/src/tauri_commands.rs`, append:
```rust
// import AI model commands
mod models;
pub use models::*;
```

In `crates/jarvis-gui/src/main.rs`, in `invoke_handler`, add after `list_gliner_models,`:
```rust
tauri_commands::list_intent_backends,
```

**Step 7: Build to verify**

```bash
cargo build -p jarvis-gui 2>&1 | grep -E "error|warning: unused" | grep -v "target/"
```
Expected: no errors.

**Step 8: Commit**

```bash
git add crates/jarvis-core/src/models.rs \
        crates/jarvis-gui/src/tauri_commands/models.rs \
        crates/jarvis-gui/src/tauri_commands.rs \
        crates/jarvis-gui/src/main.rs
git commit -m "feat(gui): add list_intent_backends Tauri command driven by model catalog"
```

---

## Task 4: Update Frontend to Use Dynamic Backends

Replace the hardcoded `[{label, value}, ...]` array with a runtime call to `list_intent_backends`.

**Files:**
- Modify: `frontend/src/routes/settings/index.svelte`

**Step 1: Audit current hardcoded code**

Current code (lines ~85, ~225-245, ~411-420):
```javascript
// Declaration:
let selectedIntentRecognitionEngine = ""

// Load (onMount):
const intentReco = await invoke<string>("db_read", { key: "selected_intent_recognition_engine" })
selectedIntentRecognitionEngine = intentReco

// Dropdown:
<NativeSelect
    data={[
        { label: "Intent Classifier", value: "IntentClassifier" },
        { label: "Embedding Classifier", value: "EmbeddingClassifier" }
    ]}
    bind:value={selectedIntentRecognitionEngine}
/>
```

Problems:
- `"IntentClassifier"` → backend expects `"intent-classifier"` (will now be handled by alias, but still wrong)
- `"EmbeddingClassifier"` → handled by legacy mapping, but models with LFS pointers could be offered
- Hardcoded: adding a new model requires code change

**Step 2: Add `intentBackends` reactive variable and load from Tauri**

Find the `let selectedIntentRecognitionEngine = ""` line and add below it:
```javascript
let intentBackends: { id: string; name: string }[] = []
```

In the `onMount` / load block where other `invoke` calls happen, add alongside the other `list_*` calls:
```javascript
intentBackends = await invoke<{ id: string; name: string }[]>("list_intent_backends")
```

**Step 3: Replace hardcoded dropdown data**

Find and replace the `NativeSelect` for intent:

```svelte
<!-- BEFORE: -->
<NativeSelect
    data={[
        { label: "Intent Classifier", value: "IntentClassifier" },
        { label: "Embedding Classifier", value: "EmbeddingClassifier" }
    ]}
    label={t('settings-intent-engine')}
    description={t('settings-intent-engine-desc')}
    variant="filled"
    bind:value={selectedIntentRecognitionEngine}
/>

<!-- AFTER: -->
<NativeSelect
    data={intentBackends.map(b => ({ label: b.name, value: b.id }))}
    label={t('settings-intent-engine')}
    description={t('settings-intent-engine-desc')}
    variant="filled"
    bind:value={selectedIntentRecognitionEngine}
/>
```

**Step 4: Normalize legacy value on load**

When reading the stored `intent_backend` from DB, normalize legacy values so the dropdown selection matches:

After `selectedIntentRecognitionEngine = intentReco`, add:
```javascript
// Normalize legacy enum values stored by older versions
if (selectedIntentRecognitionEngine === "IntentClassifier") {
    selectedIntentRecognitionEngine = "intent-classifier"
} else if (selectedIntentRecognitionEngine === "EmbeddingClassifier") {
    // Legacy auto-select — keep as-is, the backend handles it
    // but if catalog has a specific model, prefer it
    const hasModel = intentBackends.some(b => b.id !== "none" && b.id !== "intent-classifier")
    if (hasModel) {
        selectedIntentRecognitionEngine = intentBackends
            .find(b => b.id !== "none" && b.id !== "intent-classifier")?.id
            ?? selectedIntentRecognitionEngine
    }
}
```

Wait — actually simplify: just normalize "IntentClassifier" → "intent-classifier". Leave "EmbeddingClassifier" as-is since the backend handles it gracefully. The important thing is the saved value going forward will be the catalog model ID.

Simplified normalization:
```javascript
if (selectedIntentRecognitionEngine === "IntentClassifier") {
    selectedIntentRecognitionEngine = "intent-classifier"
}
```

**Step 5: Build frontend**

```bash
cd frontend && npm run build 2>&1 | tail -10
```
Expected: no TypeScript/Svelte errors.

**Step 6: Build jarvis-gui**

```bash
cargo build -p jarvis-gui 2>&1 | grep -E "^error" | head -10
```
Expected: no errors.

**Step 7: Commit**

```bash
git add frontend/src/routes/settings/index.svelte
git commit -m "fix(frontend): replace hardcoded intent backends with dynamic catalog-driven list"
```

---

## Task 5: End-to-End Verification

**Step 1: Copy model.toml files to target/debug (if not already done)**

```bash
cp resources/models/all-MiniLM-L6-v2/model.toml \
   target/debug/resources/models/all-MiniLM-L6-v2/model.toml

cp "resources/models/paraphrase-multilingual-MiniLM-L12-v2-onnx-Q/model.toml" \
   "target/debug/resources/models/paraphrase-multilingual-MiniLM-L12-v2-onnx-Q/model.toml"
```

**Step 2: Verify catalog at startup**

```
Expected log:
INFO: Found model: all-MiniLM-L6-v2 (English) (all-MiniLM-L6-v2) - tasks: [Intent]
WARN: Failed to load model from ".../paraphrase-multilingual-MiniLM-L12-v2-onnx-Q": binary "model.onnx" is a Git LFS pointer
INFO: Found 1 model(s) in "..."
```
The multilingual model is silently excluded (warns, not errors).

**Step 3: Set intent_backend to "EmbeddingClassifier" in app.db, run app**

```bash
python -c "
import json, pathlib
p = pathlib.Path.home() / 'AppData/Roaming/com.priler.jarvis/app.db'
d = json.loads(p.read_text())
d['intent_backend'] = 'EmbeddingClassifier'
p.write_text(json.dumps(d, indent=2))
"
./target/debug/jarvis-app.exe &
sleep 3
grep "Intent classifier unavailable\|EmbeddingClassifier\|Recording started" \
    "$APPDATA/com.priler.jarvis/log.txt" | tail -5
```
Expected: `WARN: Intent classifier unavailable ... Recording started.`

**Step 4: Set intent_backend to "all-MiniLM-L6-v2", run app**

```bash
python -c "
import json, pathlib
p = pathlib.Path.home() / 'AppData/Roaming/com.priler.jarvis/app.db'
d = json.loads(p.read_text())
d['intent_backend'] = 'all-MiniLM-L6-v2'
p.write_text(json.dumps(d, indent=2))
"
./target/debug/jarvis-app.exe &
sleep 5
grep "Embedding classifier ready\|Recording started" \
    "$APPDATA/com.priler.jarvis/log.txt" | tail -3
```
Expected: `Embedding classifier ready with N intents` + `Recording started`.

**Step 5: Run full test suite**

```bash
cargo test -p jarvis-core -p jarvis-gui 2>&1 | tail -5
```
Expected: all pass.

**Step 6: Final commit (if any loose files)**

```bash
git status
```
All should be clean (or commit remaining changes).

---

## Post-Implementation Notes

### Adding a new embedding model (no code changes needed)
1. Place model files in `resources/models/<model-name>/`
2. Create `resources/models/<model-name>/model.toml`:
   ```toml
   [model]
   id = "my-model-id"
   name = "Human Readable Name"
   tasks = ["intent"]
   description = "What this model does"
   ```
3. Copy to `target/debug/resources/models/<model-name>/` for dev
4. Model appears automatically in GUI settings dropdown

### About the multilingual model
`paraphrase-multilingual-MiniLM-L12-v2-onnx-Q/model.onnx` (235 MB) needs to be downloaded via `git lfs pull` or manually from HuggingFace. Until downloaded, the model is excluded from catalog and won't appear in GUI.
