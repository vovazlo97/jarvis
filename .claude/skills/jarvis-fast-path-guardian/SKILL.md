---
name: jarvis-fast-path-guardian
description: "Enforce Fast Path invariants. Use before ANY changes to audio pipeline, wake-word, STT, or command execution. Checks for LLM calls and latency violations."
allowed-tools: ["Read", "Bash"]
---

## Fast Path Guardian

При активации:
1. Прочитай .claude/rules/fast-path.md
2. Проверь изменённые файлы на наличие LLM/HTTP вызовов в fast path
3. Запусти: cargo test --package jarvis-core 2>&1 | grep -E "(FAILED|latency|ms)"
4. Если нашёл нарушение — ОСТАНОВИСЬ и сообщи пользователю до внесения изменений
5. Только после проверки продолжай реализацию
