# CLAUDE.md — Jarvis Project Configuration

## 1. Overview & Mission

**Jarvis** is a local AI Automation OS — 100% offline, privacy-first voice assistant built in Rust/Tauri.
- No cloud dependencies for core functionality
- Fast Path latency guarantee: <250ms P50, <400ms P95
- Open plugin marketplace vision (target: 100K GitHub stars)

## 2. Architecture Principles

### Fast Path (<250ms P50, <400ms P95)
- Wake-word → STT → Intent → Command execution
- **NO LLM calls** in Fast Path — ever
- **NO blocking HTTP/network I/O**
- See `.claude/rules/fast-path.md` for full constraints

### Event Bus (tokio broadcast channels)
- Only communication channel between modules
- Fast Path emits events → Slow Path handles async work
- No direct module-to-module calls

### Command Registry (atomic write)
- Thread-safe, atomic reads/writes
- Commands loaded from `resources/commands/*.toml`

### Plugin Manifest (plugin.json)
- Every plugin MUST have a valid `plugin.json`
- Schema defined in `.claude/rules/plugins.md`

### Agent Registry
- Registry of available automation agents
- Plugins register agents via manifest

### STT Interface (ISpeechToText)
- Abstraction over Vosk (or future backends)
- Must not block Fast Path thread

## 3. Claude Code Responsibilities

### MAY:
- Create feature branches (`feature/`, `fix/`, `ai/`, `docs/`)
- Edit files in `crates/`, `frontend/`, `resources/`, `docs/`, `.claude/`
- Run `cargo fmt`, `cargo clippy`, `cargo test`, `cargo build`
- Commit to non-main branches
- Install tools listed in `.claude/rules/plugins.md`

### MUST NOT:
- Commit directly to `main`
- Push to `main` (force push NEVER)
- Add LLM API calls to Fast Path code
- Add blocking I/O to Fast Path code
- Install unauthorized external tools without user confirmation

## 4. Skills & Sub-agents

### Project Skills (project-level, .claude/skills/)
| Skill | Когда вызывать |
|---|---|
| `jarvis-architecture` | Первым — при любом изменении ядра. Загружает ADRs и контекст |
| `jarvis-fast-path-guardian` | Перед ЛЮБЫМИ изменениями audio/STT/wake-word/Fast Path |
| `jarvis-doc-writer` | После реализации фичи — обновить docs/ |
| `jarvis-commit-bot` | Перед каждым коммитом |
| `jarvis-plugin-scaffold` | При создании нового плагина |
| `tasks` | Управление docs/TASKS.md |

### Installed Agents (project)
| Agent | Зона ответственности | НЕ использовать для |
|---|---|---|
| `rust-engineer` | Rust-код в crates/* — типы, async, lifetime, idiomatic Rust | Архитектурных решений |
| `architecture-modernizer` | Крупные рефакторинги, разбиение модулей, границы core/app/plugins | Написания кода |
| `error-detective` | Баги, падающие тесты, runtime-ошибки | Новых фич |
| `ai-engineer` | Slow Path, LLM-интеграция, Agent Registry, Companion LLM | Fast Path кода |

### Routing Rules
**Изменение ядра (jarvis-core):**
/jarvis-architecture → architecture-modernizer (план) → rust-engineer + /test-driven-development → /jarvis-fast-path-guardian → /jarvis-commit-bot

**Баг / падающий тест:**
error-detective + /systematic-debugging → rust-engineer (фикс) → /verification-before-completion → /jarvis-commit-bot

**Новый плагин:**
/jarvis-architecture → /jarvis-plugin-scaffold → rust-engineer → /jarvis-doc-writer → /jarvis-commit-bot

**LLM / AI фичи (Slow Path):**
/jarvis-architecture → ai-engineer (дизайн) → architecture-modernizer (если меняем границы) → rust-engineer → /jarvis-commit-bot

### Конфликты — чего избегать
- architecture-modernizer планирует, rust-engineer пишет — никогда не смешивай роли
- ai-engineer только Slow Path — НИКОГДА Fast Path
- /jarvis-fast-path-guardian ВСЕГДА после rust-engineer если касаемся audio/STT
- error-detective только диагностирует → передаёт задачу rust-engineer

### Skill Creation Rule

Когда нужно создать новый скилл для проекта:
1. Использовать `skill-creator` плагин (`claude plugin install skill-creator@claude-plugins-official`)
2. `skill-creator` проведёт интервью → создаст SKILL.md с правильной структурой:
   - YAML frontmatter: `name`, `description` (trigger condition)
   - Конкретные инструкции (что делать шаг за шагом)
   - Output format (что выводить в конце)
3. Сохранить результат в `.claude/skills/<skill-name>/SKILL.md`
4. Зарегистрировать в таблице скиллов выше и в `.claude/CLAUDE.md`

**Стандарт качества для каждого скилла:**
- `description` — чёткий trigger condition, достаточно "pushy" чтобы не under-trigger
- Инструкции — конкретные шаги с командами, не абстрактные советы
- Output format — явный шаблон вывода в конце
- Нет дублирования с другими скиллами
- Соответствует архитектуре Jarvis (Fast Path, Event Bus, Rust/Tauri)

## 5. MCP Servers

```
# Add when needed:
claude mcp add github-mcp -- npx -y @modelcontextprotocol/server-github
claude mcp add filesystem -- npx -y @modelcontextprotocol/server-filesystem .
claude mcp add memory -- npx -y @modelcontextprotocol/server-memory
```

Currently active: `context7` (library docs), `serena` (symbol analysis)

## 6. Rules & Guardrails

- **Formatting:** `cargo fmt --all` before every commit
- **Linting:** `cargo clippy -- -D warnings` must pass
- **Commits:** Conventional Commits format (see `.claude/rules/git-workflow.md`)
- **Branches:** See `.claude/rules/git-workflow.md`
- **Plugins:** See `.claude/rules/plugins.md`
- **Fast Path:** See `.claude/rules/fast-path.md`

## 7. Task Management

- Active tasks: `docs/TASKS.md`
- Project memory / ADRs: `docs/MEMORY.md`
- Read `docs/TASKS.md` at the start of each session
- Update `docs/MEMORY.md` after architectural decisions

## 8. Playbook

### Bugfix Flow
1. error-detective + `/systematic-debugging` → найти причину
2. Create branch `fix/<issue-id>-<description>`
3. rust-engineer + `/test-driven-development` → фикс
4. `/verification-before-completion`
5. `/jarvis-commit-bot` → commit
6. `/finishing-a-development-branch` → PR

### New Plugin Flow
1. `/jarvis-architecture` → load context
2. `/jarvis-plugin-scaffold` → scaffold structure
3. rust-engineer → implement in `plugins/<name>/`
4. `/jarvis-fast-path-guardian` if touching audio pipeline
5. `/jarvis-doc-writer` → update docs
6. `/jarvis-commit-bot` → commit

### Refactor Flow
1. `/jarvis-architecture` → load context
2. architecture-modernizer → план рефакторинга
3. `/jarvis-fast-path-guardian` if touching Fast Path
4. rust-engineer + `/test-driven-development` → реализация
5. `/verification-before-completion`
6. `/jarvis-commit-bot` → commit

### Release Flow
1. All Phase tasks done in `docs/TASKS.md`
2. Bump version in `Cargo.toml` (workspace)
3. Update `CHANGELOG.md`
4. `/finishing-a-development-branch` → PR to main
5. Tag release after merge

### Debug Flow
1. error-detective + `/systematic-debugging`
2. Check `cargo test` output
3. Check `cargo clippy` warnings
4. Isolate to crate: `cargo test --package jarvis-<crate>`

### AI Feature Flow
1. `/jarvis-architecture` → Slow Path boundary context
2. ai-engineer → проектирование
3. architecture-modernizer → если меняем границы модулей
4. rust-engineer + `/test-driven-development` → реализация
5. `/jarvis-commit-bot` → commit

---

## 9. Performance Contracts

| Metric | Target | Test |
|---|---|---|
| Wake-word detection | <50ms P50 | `cargo bench wake_word` |
| STT (short phrase) | <200ms P50 | `cargo bench stt` |
| Full Fast Path | <250ms P50, <400ms P95 | `cargo bench fast_path` |

Benchmarks are **mandatory** before merging any Fast Path changes.
Use `cargo-criterion` for benchmark tracking.

## 10. Project Structure (Architecture Map)

### Три слоя системы
- **jarvis-gui** (Svelte + Tauri) — UI, общается только через IpcEvent/WebSocket и Tauri invoke
- **jarvis-app** (main.rs + app.rs + fast_path.rs) — точка входа, Fast Path пайплайн
- **jarvis-core** — ядро: eventbus, stt, intent, audio, commands, scripts, models

### Fast Path — 5 шагов (fast_path.rs)
1. Listener — Vosk слушает микрофон, ловит wake-word → publish(WakeWordDetected)
2. STT — Vosk транскрибирует речь → publish(SpeechRecognized { text })
3. Intent Classifier — EmbeddingClassifier находит команду → publish(CommandRecognized { id, text })
4. Command Executor — ищет в registry, запускает процесс/скрипт → publish(CommandExecuted { id, success })
5. Audio Feedback — Kira играет ok.wav / notfound.wav → publish(StateChanged)

### Event Bus — правило расширения
Новая фича = новый подписчик. Не меняет существующий код:
```rust
let mut rx = eventbus::subscribe().unwrap();
tokio::spawn(async move {
    while let Ok(event) = rx.recv().await {
        // твоя логика
    }
});
```

### Model Catalog — правило добавления модели
Добавить модель = создать папку `resources/models/<id>/model.toml` + реальный бинарник (не LFS pointer).
GUI читает каталог динамически. Код не меняется.
**ВАЖНО:** Если `model.onnx` является Git LFS pointer (первые байты = `version https://git-lfs`) — модель автоматически исключается из каталога с WARN.

### Текущий стек моделей
| Модель | Задача | Статус |
|---|---|---|
| vosk-model-small-ru-0.22 | STT | АКТИВНА |
| all-MiniLM-L6-v2 (90MB, English) | Intent | АКТИВНА |
| paraphrase-multilingual-MiniLM-L12-v2 | Intent | НЕ СКАЧАНА (LFS pointer — исключена из каталога) |
| GLiNER | Slots | ОТКЛЮЧЁН |

### Intent Classifier Fallback Policy
Если выбранная embedding-модель недоступна (LFS pointer / не скачана):
- `catalog::scan_models()` исключает модель с WARN — не попадает в GUI
- `intent::init()` логирует WARN и устанавливает backend = "none" — **не падает**
- Роутинг команд работает через regex/fuzzy matching

### Data Storage Invariant

**NEVER write user data to `resources/` or `target/`.**

| Data Type | Storage | Why |
|---|---|---|
| Bundled defaults (commands, scripts) | `resources/commands/`, `resources/scripts/` (git, read-only) | Copied to `target/` at build — ephemeral |
| User commands | `config::user_commands_dir()` = `APP_CONFIG_DIR/commands/` | Persistent across rebuilds |
| User scripts | `config::user_scripts_dir()` = `APP_CONFIG_DIR/scripts/` | Persistent across rebuilds |
| Settings (app.db) | `APP_CONFIG_DIR/app.db` | Already correct |

**Rule:** All Tauri CRUD commands (`create_command_pack`, `save_script`, etc.) MUST write to
`user_commands_dir()` / `user_scripts_dir()`. Violation = data loss on rebuild.
