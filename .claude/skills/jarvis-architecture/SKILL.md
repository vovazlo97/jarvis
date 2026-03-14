---
name: jarvis-architecture
description: "Load Jarvis architecture context. ALWAYS invoke this skill FIRST before ANY change to jarvis-core, fast_path.rs, event bus, command registry, STT, audio pipeline, plugin system, or agent registry. Also invoke when designing a new feature, planning a refactor, or any time you're unsure where code belongs in the three-layer architecture (GUI/App/Core). Do not skip this even for 'small' changes — the Fast Path invariants catch bugs at design time, not at runtime."
---

## Контекст архитектуры Jarvis

### Шаг 1 — загрузи контекст

Прочитай оба источника перед тем как принимать архитектурные решения:
- `docs/MEMORY.md` — раздел "Architectural Decisions" (ADRs, ключевые компромиссы)
- `.claude/rules/fast-path.md` — жёсткие ограничения Fast Path

### Шаг 2 — зафиксируй инварианты

Держи в уме на протяжении всей задачи:

| Слой | Что делает | Ограничения |
|---|---|---|
| **jarvis-gui** | Svelte + Tauri UI | Только IpcEvent / WebSocket / Tauri invoke — не обращается к Core напрямую |
| **jarvis-app** | Точка входа, Fast Path pipeline | Оркестрирует модули, сам бизнес-логики не содержит |
| **jarvis-core** | Event Bus, STT, Intent, Audio, Commands | Ядро системы — не знает о GUI и App |

**Fast Path** (fast_path.rs) — 5 шагов, цель P50 < 250ms:

```
WakeWordDetected → SpeechRecognized → CommandRecognized → CommandExecuted → StateChanged
```

**Золотые правила (нарушение = auto-reject PR):**
- Fast Path: NO LLM, NO blocking I/O, NO HTTP
- Event Bus — единственный канал коммуникации между модулями (нет прямых вызовов)
- Каждый плагин ОБЯЗАН иметь `plugin.json` с валидным манифестом
- Новая фича = новый подписчик на Event Bus, существующий код не меняется

### Шаг 3 — определи границы изменений

Ответь на эти вопросы перед написанием кода:

1. **Какой слой затрагивается?** GUI / App / Core
2. **Затрагивает ли Fast Path?** → если да, вызови `/jarvis-fast-path-guardian` ПЕРЕД коммитом
3. **Нужен новый JarvisEvent?** → добавь в enum, не ломай существующие варианты
4. **Нужен новый плагин?** → вызови `/jarvis-plugin-scaffold`
5. **Затрагивает публичный API?** → вызови `/jarvis-doc-writer` после

### Вывод (обязателен)

После загрузки контекста выведи краткое резюме:

```
## Архитектурный контекст загружен

**Затронутые слои:** <GUI / App / Core>
**Модули:** <eventbus / stt / intent / audio / commands / ...>
**Fast Path риск:** <да / нет — почему>
**Следующие скилы:** <список в порядке вызова>
**Ключевые ограничения:** <что важно помнить для этой задачи>
```
