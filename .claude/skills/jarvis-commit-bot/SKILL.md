---
name: jarvis-commit-bot
description: "Generate conventional commit messages and PR descriptions for the Jarvis project. ALWAYS invoke before every git commit and when creating a PR — especially after: completing a task, fixing a bug, adding a feature, refactoring code, updating documentation, or running cargo fmt/clippy. If you've made changes and are about to commit, this skill must run first. It ensures messages follow Conventional Commits format with Jarvis-specific scopes and enforces formatting/linting checks before commit."
disable-model-invocation: false
---

## Commit Bot — генерация сообщений коммита

### Шаг 1 — проверь форматирование и линтинг

Перед коммитом обязательно:

```bash
cargo fmt --all
cargo clippy -- -D warnings
```

Если clippy возвращает ошибки — не коммить, исправь сначала.

### Шаг 2 — анализ изменений

```bash
git diff --staged
# если нет staged файлов:
git diff HEAD
git status
```

### Шаг 3 — классификация типа коммита

| Тип | Когда использовать |
|---|---|
| `feat` | Новая функциональность для пользователя |
| `fix` | Исправление бага |
| `refactor` | Рефакторинг без изменения поведения |
| `docs` | Только документация |
| `test` | Добавление / исправление тестов |
| `chore` | Зависимости, конфиги, CI, инструменты |
| `perf` | Улучшение производительности |

**Scopes для Jarvis:**
`core`, `registry`, `fast-path`, `event-bus`, `stt`, `audio`, `intent`, `plugin`, `gui`, `app`, `docs`, `cli`, `models`, `skills`

### Шаг 4 — сформируй сообщение коммита

Формат:
```
type(scope): короткое описание (max 72 символа)

[опциональное тело — объясни ПОЧЕМУ, не ЧТО]

Refs: TASK-XXX
```

**Примеры хороших сообщений:**
```
feat(plugin): add weather_fetch plugin with Event Bus integration
fix(fast-path): prevent blocking I/O in wake-word handler
refactor(event-bus): extract AudioFeedback subscriber to separate module
docs(sdk): document ISpeechToText trait and VoskBackend contract
perf(intent): reduce embedding lookup from O(n) to O(log n)
chore(deps): update tokio to 1.36, kira to 0.9
```

### Шаг 5 — подтверждение

Предложи пользователю подтвердить сообщение перед выполнением `git commit`. Не коммить без явного подтверждения.

### Шаг 6 — описание PR (если создаём PR)

```markdown
## Summary
- <Что сделано — 1-3 bullet points>

## Changes
- `<file>` — <что изменилось>

## Testing
- [ ] `cargo test --package jarvis-<crate>` passed
- [ ] `cargo clippy -- -D warnings` passed
- [ ] `cargo fmt --all` applied
- [ ] Fast Path не нарушен (если касались audio/STT)

## Related
- TASK-XXX
```

### Вывод

```
## Commit готов

Сообщение: <type(scope): description>
Файлов:    N staged
Ветка:     <branch-name>

[ожидание подтверждения пользователя]
```
