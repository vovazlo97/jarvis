---
name: jarvis-fast-path-guardian
description: "Enforce Fast Path invariants — MUST be called before committing ANY changes that touch: fast_path.rs, audio pipeline, wake-word detection, STT (speech-to-text), command execution, or any code in crates/jarvis-core/src/ that runs in the real-time audio loop. Also invoke when merging a feature branch that touches performance-critical code. If in doubt about whether something is Fast Path — call this skill. It's far cheaper to check unnecessarily than to ship a latency violation or LLM call into the hot path."
allowed-tools: ["Read", "Bash"]
---

## Fast Path Guardian — проверка инвариантов

Цель: поймать нарушения Fast Path ДО коммита, пока исправить дёшево.

### Шаг 1 — прочитай правила

Прочитай `.claude/rules/fast-path.md` — там полный список жёстких ограничений.

### Шаг 2 — найди изменённые файлы Fast Path

```bash
git diff --name-only HEAD | grep -E "(fast_path|audio|wake|stt|speech|command_exec)"
```

Если список пуст — Fast Path не затронут. Сообщи "✅ Fast Path не затронут" и выходи.

### Шаг 3 — проверь запрещённые паттерны

Для каждого изменённого файла:

```bash
# LLM API вызовы (запрещены всегда)
git diff HEAD -- <файл> | grep -E "(openai|anthropic|ollama|reqwest::get|HttpClient|Client::new)"

# Blocking I/O
git diff HEAD -- <файл> | grep -E "(std::fs::|\.blocking_|thread::sleep|std::io::stdin)"

# Синхронные сетевые вызовы
git diff HEAD -- <файл> | grep -E "(TcpStream::connect|UdpSocket::bind|\.connect\()"
```

### Шаг 4 — запусти тесты

```bash
cargo test --package jarvis-core 2>&1 | tail -30
```

### Шаг 5 — вывод отчёта

**Если нарушений нет:**

```
✅ Fast Path Guardian: PASSED

Проверено файлов: N
LLM calls:        0
Blocking I/O:     0
Network calls:    0
Tests:            PASS

Можно коммитить.
```

**Если найдено нарушение — СТОП, не продолжай:**

```
🚨 Fast Path VIOLATION

Файл:      <path/to/file.rs>
Строка:    <N>
Нарушение: <тип — LLM call / blocking I/O / network>
Код:       <строка кода>

Требуемое действие:
  → Вынести логику в Slow Path через Event Bus
  → Опубликовать событие из Fast Path, обработать async в подписчике

Пример исправления:
  БЫЛО:  let result = llm_client.complete(text).await?;
  СТАЛО: eventbus::publish(JarvisEvent::SlowPathRequest { text })?;
```

Не продолжай реализацию пока нарушения не устранены.
