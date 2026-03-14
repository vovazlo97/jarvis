# Core Stabilization — Phase A Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Стабилизировать Fast Path pipeline: устранить ERROR при старте, восстановить нейросеть intent classification, построить единый Model Registry для GUI.

**Architecture:** Четыре независимых изменения в порядке приоритета. Каждое — минимальный точечный фикс. Модули остаются изолированными и общаются только через существующие интерфейсы. Model Registry Module строится поверх уже существующего `models/catalog.rs` + `models/registry.rs` — добавляем единый публичный API и Tauri-команду.

**Tech Stack:** Rust 1.75+, tokio, fastembed, once_cell, parking_lot, Tauri

---

## Контекст: Текущее состояние (из аудита)

```
app.db: intent_backend="EmbeddingClassifier", slots_backend="None", language="ru"

Модели:
  ✅ all-MiniLM-L6-v2             — реальный бинарник (90MB)
  ❌ paraphrase-multilingual       — LFS pointer, исключён из каталога
  ✅ vosk-model-small-ru-0.22     — реальный
  ❌ GLiNER                       — отсутствует

Fast Path компоненты:
  Wake-word: ⚠️  WARN (Vosk, медленный, функционирует)
  STT:       ✅  OK
  Intent:    ⚠️  FALLBACK — нейросеть отключена, работает regex/fuzzy
  Slots:     ❌  ERROR при каждом старте ("None" ≠ "none")
  Audio:     ✅  OK
```

---

## Task 1: Fix Slots ERROR — case-insensitive "none" matching

**Файлы:**
- Modify: `crates/jarvis-core/src/slots.rs`
- Test: `crates/jarvis-core/src/slots.rs` (inline `#[cfg(test)]`)

**Root cause:** `slots_backend = "None"` (capital N из GUI) не матчит `"none"` →
падает в model_id ветку → `models::gliner::load(registry, "None")` → `Err("Model 'None' not found in catalog")` → `error!()` при каждом старте.

**Step 1: Написать падающий тест**

Добавить в конец `slots.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// slots::init() must treat "None" (capital N, legacy GUI value)
    /// the same as "none" — no GLiNER load attempt, no error.
    #[test]
    fn init_treats_none_variants_as_disabled() {
        // We can't call init() (needs DB), but we can test the normalization directly.
        // Test: normalize_backend("None") == "none"
        assert_eq!(normalize_backend("None"), "none");
        assert_eq!(normalize_backend("NONE"), "none");
        assert_eq!(normalize_backend("none"), "none");
        assert_eq!(normalize_backend(""), "none");
        assert_eq!(normalize_backend("some-model-id"), "some-model-id");
    }
}
```

Запустить: `cargo test --package jarvis-core slots`
Ожидание: FAIL — `normalize_backend` не существует.

**Step 2: Добавить `normalize_backend` и применить к `slots::init()`**

В `slots.rs`, после строки `use crate::{models, DB};` добавить:

```rust
/// Normalize backend value from DB — treat empty / any case of "none" as "none".
/// The GUI historically sends "None" (Python-style None-string); the match arm
/// expects lowercase. Normalization here prevents a GLiNER load attempt for a
/// non-existent model ID.
fn normalize_backend(raw: &str) -> &str {
    match raw.to_lowercase().as_str() {
        "none" | "" => "none",
        _ => raw,
    }
}
```

Изменить `init()`:

```rust
pub fn init() -> Result<(), String> {
    if BACKEND.get().is_some() {
        return Ok(());
    }

    let raw = DB
        .get()
        .map(|db| db.read().slots_backend.clone())
        .unwrap_or_else(|| "none".to_string());

    let backend = normalize_backend(&raw).to_string();

    BACKEND
        .set(backend.clone())
        .map_err(|_| "Slot backend already set")?;

    match backend.as_str() {
        "none" => {
            info!("Slot extraction disabled");
        }
        model_id => {
            info!(
                "Initializing GLiNER slot extraction with model '{}'.",
                model_id
            );
            let model = models::gliner::load(models::registry(), model_id)?;
            gliner::init_with_model(model)?;
            info!("GLiNER slot extraction initialized.");
        }
    }

    Ok(())
}
```

**Проблема:** `normalize_backend` возвращает `&str`, но `raw` — локальная переменная.
Нужно вернуть `String`. Скорректированная сигнатура:

```rust
fn normalize_backend(raw: &str) -> String {
    if raw.is_empty() || raw.eq_ignore_ascii_case("none") {
        "none".to_string()
    } else {
        raw.to_string()
    }
}
```

И тест:
```rust
assert_eq!(normalize_backend("None"), "none");
assert_eq!(normalize_backend("NONE"), "none");
assert_eq!(normalize_backend("none"), "none");
assert_eq!(normalize_backend(""), "none");
assert_eq!(normalize_backend("some-model-id"), "some-model-id".to_string());
```

**Step 3: Запустить тесты**

```bash
cargo test --package jarvis-core slots
```
Ожидание: все тесты PASS, в том числе новый `init_treats_none_variants_as_disabled`.

```bash
cargo test --package jarvis-core
```
Ожидание: все 51 теста PASS.

**Step 4: Проверить clippy на изменённом файле**

Нет новых ошибок в `slots.rs`.

---

## Task 2: Intent — fallback к all-MiniLM-L6-v2 для русского языка

**Файлы:**
- Modify: `crates/jarvis-core/src/intent.rs`
- Test: `crates/jarvis-core/src/intent.rs` (inline)

**Root cause:** `EmbeddingClassifier` + `language="ru"` → выбирает
`paraphrase-multilingual-MiniLM-L12-v2` (LFS pointer, исключён из catalog) →
`Err("Model not found in catalog")` → warn + BACKEND = "none" → нейросеть отключена.
Но `all-MiniLM-L6-v2` (90MB) РЕАЛЬНО ДОСТУПЕН и может обрабатывать русские команды.

**Стратегия:**
В ветке `EmbeddingClassifier`: если предпочтительная модель недоступна — попробовать
`all-MiniLM-L6-v2` как универсальный fallback перед тем как отключать нейросеть.

**Step 1: Написать падающий тест**

Тест трудно изолировать (нужен DB, registry). Пишем тест для вспомогательной
функции `select_embedding_model_id`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// select_embedding_model_id must return "all-MiniLM-L6-v2" for English.
    #[test]
    fn selects_minilm_for_english() {
        assert_eq!(select_embedding_model_id("en"), "all-MiniLM-L6-v2");
    }

    /// select_embedding_model_id must return the multilingual model for non-English.
    #[test]
    fn selects_multilingual_for_russian() {
        assert_eq!(
            select_embedding_model_id("ru"),
            "paraphrase-multilingual-MiniLM-L12-v2"
        );
    }

    /// FALLBACK_EMBEDDING_MODEL must always be all-MiniLM-L6-v2.
    #[test]
    fn fallback_model_is_minilm() {
        assert_eq!(FALLBACK_EMBEDDING_MODEL, "all-MiniLM-L6-v2");
    }
}
```

Запустить: `cargo test --package jarvis-core intent::tests`
Ожидание: FAIL — `select_embedding_model_id` и `FALLBACK_EMBEDDING_MODEL` не существуют.

**Step 2: Извлечь константу и функцию, изменить ветку EmbeddingClassifier**

В начало `intent.rs` (после `use` блоков):

```rust
/// Universal fallback embedding model — always available (real binary, not LFS pointer).
const FALLBACK_EMBEDDING_MODEL: &str = "all-MiniLM-L6-v2";

/// Select preferred embedding model ID based on language.
fn select_embedding_model_id(lang: &str) -> &'static str {
    match lang {
        "en" => "all-MiniLM-L6-v2",
        _ => "paraphrase-multilingual-MiniLM-L12-v2",
    }
}
```

Изменить ветку `"EmbeddingClassifier"` в `init()`:

```rust
"EmbeddingClassifier" => {
    let lang = crate::i18n::get_language();
    let preferred = select_embedding_model_id(&lang);
    info!(
        "EmbeddingClassifier (auto) → preferred model '{}' (language: {}).",
        preferred, lang
    );
    let model_result = models::embedding::load(models::registry(), preferred)
        .or_else(|e| {
            if preferred != FALLBACK_EMBEDDING_MODEL {
                warn!(
                    "Preferred model '{}' unavailable ({}). \
                     Trying universal fallback '{}'...",
                    preferred, e, FALLBACK_EMBEDDING_MODEL
                );
                models::embedding::load(models::registry(), FALLBACK_EMBEDDING_MODEL)
            } else {
                Err(e)
            }
        });
    match model_result {
        Ok(model) => {
            embeddingclassifier::init_with_model(model, commands)?;
            BACKEND.set(backend).map_err(|_| "Backend already set")?;
            info!("EmbeddingClassifier backend initialized.");
        }
        Err(e) => {
            set_fallback(&e);
        }
    }
}
```

**Step 3: Запустить тесты**

```bash
cargo test --package jarvis-core intent
```
Ожидание: новые тесты PASS.

```bash
cargo test --package jarvis-core
```
Ожидание: все тесты PASS.

---

## Task 3: Graceful GLiNER disable в slots::init()

**Статус:** Частично решено в Task 1 (normalize_backend устраняет ERROR).
Дополнительно: убедиться что `slots::init()` не является fatal при любой ошибке.

Смотрим `main.rs:153`:
```rust
slots::init()
    .map_err(|e| error!("Slot extraction init failed: {}", e))
    .ok();
```
`ok()` уже означает non-fatal. ERROR логируется но не крашит.

**Единственная проблема** — само `error!()` в `.map_err()` создаёт шум в логах.
После Task 1 это не будет происходить для `slots_backend="None"`.

Если `slots_backend` = реальная несуществующая модель → `error!` правильно.
Если "None" → тихий `info!` после Task 1.

**Дополнительное действие Task 3:** Убедиться что `main.rs` использует `warn!` вместо `error!` для slots init failure, так как slots не является критическим компонентом:

В `main.rs`, строка ~153-155:

```rust
// Slots are non-critical: missing model → warn, not error
slots::init()
    .map_err(|e| warn!("Slot extraction unavailable: {}", e))
    .ok();
```

**Тест:** После Task 1 + Task 3 — запустить `cargo test --workspace`.
Смотреть что все тесты PASS.

---

## Task 4: Model Registry — публичный API для GUI

**Файлы:**
- Modify: `crates/jarvis-core/src/models.rs` — добавить `pub fn list_available(task: Task)`
- Modify: `crates/jarvis-core/src/models/catalog.rs` — добавить `available` флаг в `ModelDef`
- Modify: `crates/jarvis-core/src/models/structs.rs` — добавить поле `available: bool`
- Modify: `crates/jarvis-gui/src/tauri_commands/stt.rs` — добавить Tauri-команду `list_available_models`
- Modify: `crates/jarvis-gui/src/main.rs` — зарегистрировать команду
- Test: `crates/jarvis-core/src/models/catalog.rs` (inline)

**Цель:** GUI вызывает одну Tauri-команду `list_available_models(task)` и получает список
реально доступных моделей для задачи. "Доступна" = файл существует И не LFS pointer.

**Архитектурные решения:**
- Не создаём новый модуль — расширяем существующий `models.rs` + `catalog.rs`
- `ModelDef` получает поле `available: bool` (устанавливается при scan)
- `BackendOption` получает поле `available: bool` — GUI знает что показывать
- `get_options()` продолжает возвращать все опции, но с флагом `available`
- Новая функция `list_available(task)` возвращает только доступные

**Шаг 1: Добавить `available` в `ModelDef` и `BackendOption`**

`crates/jarvis-core/src/models/structs.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDef {
    pub id: String,
    pub name: String,
    pub tasks: Vec<Task>,

    #[serde(default)]
    pub description: String,

    /// true if the primary binary file exists and is not a Git LFS pointer.
    /// Set at runtime during catalog scan.
    #[serde(skip)]
    pub available: bool,

    #[serde(skip)]
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackendOption {
    pub id: String,
    pub name: String,
    pub model_id: Option<String>,
    /// true if the backend is usable (code backend always true; model backend
    /// true only if the model binary is present and not an LFS pointer).
    pub available: bool,
}
```

**Шаг 2: Написать падающий тест**

```rust
// в catalog.rs tests:

/// ModelDef from a folder with real binary must have available=true.
#[test]
fn load_model_def_sets_available_true_for_real_binary() {
    let dir = tempfile::tempdir().unwrap();
    let model_dir = dir.path().join("real");
    std::fs::create_dir_all(&model_dir).unwrap();
    write_model_toml(&model_dir);
    std::fs::write(model_dir.join("model.onnx"), b"\x08\x07pytorch").unwrap();

    let models = scan_models(dir.path());
    assert_eq!(models.len(), 1);
    assert!(models[0].available, "real binary model must be available=true");
}

/// ModelDef from folder without .onnx must have available=true
/// (non-embedding models like Vosk don't have .onnx — still available).
#[test]
fn load_model_def_sets_available_true_without_onnx() {
    let dir = tempfile::tempdir().unwrap();
    let model_dir = dir.path().join("vosk");
    std::fs::create_dir_all(&model_dir).unwrap();
    write_model_toml(&model_dir);

    let models = scan_models(dir.path());
    assert_eq!(models.len(), 1);
    assert!(models[0].available, "model without onnx must be available=true");
}
```

Запустить: `cargo test --package jarvis-core models::catalog`
Ожидание: FAIL — поле `available` не существует.

**Шаг 3: Добавить `available` в `scan_models` / `load_model_def`**

В `catalog.rs::load_model_def()` — `available` всегда `true` если дошли до `Ok(def)`:

```rust
fn load_model_def(toml_path: &Path, model_dir: &Path) -> Result<ModelDef, String> {
    let content = fs::read_to_string(toml_path)...?;
    let parsed: ModelToml = toml::from_str(&content)...?;
    let mut def = parsed.model;
    def.path = model_dir.to_path_buf();

    let onnx_path = model_dir.join("model.onnx");
    if onnx_path.exists() && is_lfs_pointer(&onnx_path) {
        return Err("model.onnx is a Git LFS pointer...".to_string());
    }

    def.available = true;  // ← reached here = binary is real (or no binary = non-ONNX model)
    Ok(def)
}
```

**Шаг 4: Обновить `BackendOption` — `available` поле**

В `catalog.rs::code_backends()` — добавить `available: true` ко всем code backends.
В `catalog.rs::get_options()` — `BackendOption` для моделей: `available: model.available`.

```rust
// code_backends — все code backends всегда доступны
BackendOption {
    id: "intent-classifier".into(),
    name: "Intent Classifier".into(),
    model_id: None,
    available: true,
}

// "none" / "Disabled" — всегда доступно
BackendOption {
    id: "none".into(),
    name: "Disabled".into(),
    model_id: None,
    available: true,
}

// модели из catalog
BackendOption {
    id: model.id.clone(),
    name: model.name.clone(),
    model_id: Some(model.id.clone()),
    available: model.available,
}
```

**Шаг 5: Добавить `list_available` в `models.rs`**

```rust
/// Returns only the BackendOptions that are actually usable for a task.
/// "Disabled" (id="none") is always included.
/// Code backends are always included.
/// Model backends are included only if available=true.
pub fn list_available(task: Task) -> Vec<BackendOption> {
    get_options(task)
        .into_iter()
        .filter(|opt| opt.available)
        .collect()
}
```

**Шаг 6: Tauri-команда `list_available_models`**

В `crates/jarvis-gui/src/tauri_commands/stt.rs` добавить:

```rust
use jarvis_core::models::{self, Task};

#[tauri::command]
pub fn list_available_models(task: String) -> Vec<jarvis_core::models::BackendOption> {
    let t = match task.as_str() {
        "intent" => Task::Intent,
        "slots" => Task::Slots,
        "stt" => Task::Stt,
        "vad" => Task::Vad,
        "noise_suppression" => Task::NoiseSuppression,
        _ => return vec![],
    };
    models::list_available(t)
}
```

В `crates/jarvis-gui/src/main.rs` — добавить в `.invoke_handler()`:

```rust
tauri_commands::list_available_models,
```

**Шаг 7: Написать тест для `list_available`**

```rust
// в models/catalog.rs tests:

/// get_options must mark model with real binary as available=true.
/// get_options must always include "none" option as available.
#[test]
fn get_options_marks_real_model_as_available() {
    let dir = tempfile::tempdir().unwrap();
    let model_dir = dir.path().join("real-intent");
    std::fs::create_dir_all(&model_dir).unwrap();
    write_model_toml(&model_dir);
    std::fs::write(model_dir.join("model.onnx"), b"\x08\x07pytorch").unwrap();

    let models = scan_models(dir.path());
    let options = get_options(Task::Intent, &models);

    let none_opt = options.iter().find(|o| o.id == "none").unwrap();
    assert!(none_opt.available, "'none' must always be available");

    let model_opt = options.iter().find(|o| o.id == "test-model");
    assert!(
        model_opt.map(|o| o.available).unwrap_or(false),
        "model with real binary must be available=true"
    );
}
```

**Step 8: Запустить все тесты**

```bash
cargo test --workspace
```
Ожидание: все тесты PASS (включая jarvis-gui если есть unit tests).

---

## Порядок выполнения и критерии готовности

| # | Задача | Критерий | Время |
|---|---|---|---|
| 1 | Slots ERROR fix | cargo test PASS; `slots_backend="None"` → info!, не error | ~15 мин |
| 2 | Intent fallback | cargo test PASS; ru + EmbeddingClassifier → использует all-MiniLM | ~20 мин |
| 3 | Slots non-fatal warn | cargo test PASS; main.rs использует warn! | ~5 мин |
| 4 | Model Registry API | cargo test PASS; Tauri-команда компилируется | ~30 мин |

## Итоговый коммит (jarvis-commit-bot)

После всех задач:
```bash
cargo test --workspace
cargo fmt --all
cargo clippy --package jarvis-core --package jarvis-app --package jarvis-gui
git add -p  # только изменённые файлы
```

Сообщение коммита (предложение):
```
fix(core): stabilize Fast Path — slots/intent fallback + Model Registry API

- slots: normalize "None"→"none" case-insensitive; main: warn not error
- intent: EmbeddingClassifier falls back to all-MiniLM-L6-v2 for ru lang
- models: add available:bool to ModelDef/BackendOption; list_available()
- gui: Tauri command list_available_models(task) for dynamic dropdowns
```
