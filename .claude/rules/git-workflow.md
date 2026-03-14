# Git Workflow Rules

## Branch Strategy
- main — protected, только через PR + review
- develop/phase-* — активные фазы разработки
- feature/<module>-<description> — новые фичи
- fix/<issue-id>-<description> — баг-фиксы
- ai/fix-<id> — изменения инициированные Claude
- ai/refactor-<module> — рефакторинги от Claude
- docs/update-* — только документация

## Commit Format (Conventional Commits)
```
feat(core): add Event Bus implementation
fix(registry): atomic write for Command Registry
refactor(fast-path): extract boundary module
docs(claude): update CLAUDE.md architecture section
chore(deps): update tokio to 1.x
test(event-bus): add integration tests
perf(fast-path): optimize wake-word latency
```

## PR Rules
1. Все PR должны содержать: описание изменений, тесты, ссылку на TASK-ID
2. Перед merge обязательно: /verification-before-completion + /requesting-code-review
3. После merge: обновить docs/TASKS.md + docs/MEMORY.md (Last milestone)
4. Bump версии при завершении фазы: v0.2.0 (Phase A), v0.3.0 (Phase B) и т.д.

## Versioning
- v0.1.x — текущий (bootstrap)
- v0.2.0 — Phase A complete (Core Stabilization)
- v0.3.0 — Phase B complete (Documentation)
- v0.4.0 — Phase C complete (Skills + MCP)
- v0.5.0 — Phase D complete (GitOps)
- v1.0.0 — Phase E complete (Community Ready)
