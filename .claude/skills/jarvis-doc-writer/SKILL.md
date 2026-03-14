---
name: jarvis-doc-writer
description: "Update Jarvis project documentation after implementing features, fixing bugs, or making architectural changes. ALWAYS invoke after: adding new Tauri commands, changing the Event Bus schema, adding plugins, modifying Fast Path behavior, changing public API or traits, updating CLI commands, or completing any task that changes how the system works. If you've just written or changed code and docs haven't been updated — this skill needs to run. Don't skip it even for 'small' changes."
---

## Doc Writer — обновление документации

### Шаг 1 — определи что изменилось

```bash
git diff HEAD~1 --stat
```

Или используй описание завершённой задачи / PR.

### Шаг 2 — обнови соответствующие файлы

| Что изменилось | Файл для обновления |
|---|---|
| Новое архитектурное решение / ADR | `docs/MEMORY.md` → раздел Architectural Decisions |
| Новый плагин или изменение Plugin API | `docs/plugin.md` |
| Изменение публичных трейтов / SDK | `docs/sdk.md` |
| Изменение Event Bus схемы или событий | `docs/architecture.md` |
| Новая пользовательская фича или команда | `README.md` (секция Features) |
| Любое значимое изменение | `CHANGELOG.md` |

### Шаг 3 — запись в CHANGELOG.md

Найди или создай секцию `## [Unreleased]`:

```markdown
## [Unreleased]

### Added
- <Описание новой фичи> ([TASK-XXX])

### Changed
- <Что изменилось и почему>

### Fixed
- <Что исправлено>
```

### Шаг 4 — обнови README.md при изменении публичного API

Обнови если изменился:
- Список поддерживаемых голосовых команд
- Архитектурная схема или диаграмма
- Шаги установки или конфигурации
- Список поддерживаемых STT/Intent моделей

### Шаг 5 — коммит документации

Документацию коммить отдельно от кода (или в том же PR, но отдельным коммитом):

```
docs(<area>): <what> after <why>
```

**Примеры:**
```
docs(plugin): add GLiNER slot extraction API reference
docs(architecture): update Event Bus schema after AudioFeedback refactor
docs(sdk): document ISpeechToText trait and VoskBackend contract
docs(changelog): add v0.4.0 Skills + MCP release notes
```

### Вывод (обязателен)

```
## Документация обновлена

Файлы:
- docs/MEMORY.md — <что добавлено>
- docs/plugin.md — <что обновлено>
- CHANGELOG.md  — <версия и секции>

Коммит: docs(<area>): <message>
```
